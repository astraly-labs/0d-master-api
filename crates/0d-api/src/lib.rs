pub mod docs;
pub mod dto;
pub mod errors;
pub mod handlers;
pub mod helpers;
pub mod middleware;
pub mod router;

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use axum::http::{HeaderValue, Method};
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use deadpool_diesel::postgres::Pool;
use std::{env, time::Duration};
use tokio::net::TcpListener;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

use pragma_common::services::{Service, ServiceRunner};

use docs::ApiDoc;
use middleware::RateLimitConfig;
use router::api_router;

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool,
}

pub struct ApiService {
    state: AppState,
    host: String,
    port: u16,
}

impl ApiService {
    pub fn new(state: AppState, host: &str, port: u16) -> Self {
        Self {
            state,
            host: host.to_owned(),
            port,
        }
    }
}

fn cors_layer_from_env() -> CorsLayer {
    match env::var("CORS_ALLOWED_ORIGINS") {
        Ok(origins) => {
            let allowed_origins: Vec<HeaderValue> = origins
                .split(',')
                .filter_map(|origin| {
                    let trimmed = origin.trim();
                    if trimmed.is_empty() {
                        return None;
                    }
                    match HeaderValue::from_str(trimmed) {
                        Ok(value) => Some(value),
                        Err(err) => {
                            tracing::warn!(
                                origin = trimmed,
                                error = %err,
                                "Invalid origin in CORS_ALLOWED_ORIGINS, skipping",
                            );
                            None
                        }
                    }
                })
                .collect();

            if allowed_origins.is_empty() {
                tracing::warn!(
                    "CORS_ALLOWED_ORIGINS was set but no valid origins were parsed; falling back to permissive CORS",
                );
                return CorsLayer::permissive();
            }

            tracing::info!(
                allowed = %origins,
                "Configured restricted CORS origins from environment",
            );

            CorsLayer::new()
                .allow_credentials(true)
                .allow_headers(AllowHeaders::mirror_request())
                .allow_methods(AllowMethods::list([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ]))
                .allow_origin(AllowOrigin::list(allowed_origins))
        }
        Err(_) => {
            tracing::info!("CORS_ALLOWED_ORIGINS not set; using permissive CORS configuration",);
            CorsLayer::permissive()
        }
    }
}

#[async_trait::async_trait]
impl Service for ApiService {
    async fn start<'a>(&mut self, mut runner: ServiceRunner<'a>) -> anyhow::Result<()> {
        ApiDoc::generate_openapi_json("./".into())?;

        let host = self.host.clone();
        let port = self.port;
        let state = self.state.clone();

        runner.spawn_loop(move |ctx| async move {
            let address = format!("{host}:{port}");
            let socket_addr: SocketAddr = address.parse()?;
            let listener = TcpListener::bind(socket_addr).await?;

            // Env-based rate limiting configuration
            let limiter_enabled: bool = env::var("RATE_LIMIT_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true);
            let per_second: u64 = env::var("RATE_LIMIT_PER_SECOND")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2);
            let burst_size: u32 = env::var("RATE_LIMIT_BURST_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5);
            let cleanup_secs: u64 = env::var("RATE_LIMIT_CLEANUP_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60);

            // Parse whitelist domains from env
            let whitelist_domains: HashSet<String> = env::var("RATE_LIMIT_WHITELIST_DOMAINS")
                .ok()
                .map(|domains| {
                    domains
                        .split(',')
                        .map(|d| d.trim().to_lowercase())
                        .filter(|d| !d.is_empty())
                        .collect()
                })
                .unwrap_or_default();

            if !whitelist_domains.is_empty() {
                tracing::info!(
                    whitelist = ?whitelist_domains,
                    "Rate limiting whitelist configured"
                );
            }

            // Parse request timeout from env
            let timeout_secs: u64 = env::var("REQUEST_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30);

            tracing::info!(
                timeout_secs = timeout_secs,
                "Request timeout configured"
            );

            #[allow(clippy::default_constructed_unit_structs)]
            let app = {
                let base = api_router::<ApiDoc>(state.clone())
                    .with_state(state)
                    // include trace context as header into the response
                    //start OpenTelemetry trace on incoming request
                    .layer(OtelAxumLayer::default())
                    .layer(OtelInResponseLayer::default());

                let base = if limiter_enabled {
                    // Configure rate limiting from env
                    let governor_conf = GovernorConfigBuilder::default()
                        .per_second(per_second)
                        .burst_size(burst_size)
                        .key_extractor(SmartIpKeyExtractor)
                        .finish()
                        .expect("failed to build governor config");

                    let limiter = governor_conf.limiter().clone();

                    // Periodic cleanup of the limiter's internal storage, with graceful shutdown.
                    let limiter_cleanup = limiter.clone();
                    let cancel_token = ctx.token.clone();
                    tokio::spawn(async move {
                        let mut ticker = tokio::time::interval(Duration::from_secs(cleanup_secs));
                        loop {
                            tokio::select! {
                                _ = ticker.tick() => {
                                    tracing::debug!("rate limiting storage size: {}", limiter_cleanup.len());
                                    limiter_cleanup.retain_recent();
                                }
                                () = cancel_token.cancelled() => {
                                    tracing::debug!("rate limiter cleanup task shutting down");
                                    break;
                                }
                            }
                        }
                    });

                    // Create rate limit config with whitelist
                    let rate_limit_config = RateLimitConfig {
                        limiter,
                        whitelist_domains: Arc::new(whitelist_domains),
                    };

                    // Apply custom rate limiting middleware
                    base.layer(axum::middleware::from_fn(move |req, next| {
                        let config = rate_limit_config.clone();
                        middleware::rate_limit_middleware(config, req, next)
                    }))
                } else {
                    tracing::info!("rate limiter disabled via env");
                    base
                };

                // Apply timeout middleware
                let base = base.layer(middleware::TimeoutLayer::new(Duration::from_secs(timeout_secs)));

                base.layer(cors_layer_from_env())
            };

            tracing::info!("🧩 API started at http://{}", socket_addr);

            // Create a shutdown signal from our context
            let token = ctx.token.clone();
            let shutdown = async move { token.cancelled().await };

            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(shutdown)
            .await
            .context("😱 API server stopped!")
        });

        Ok(())
    }
}
