pub mod docs;
pub mod dto;
pub mod errors;
pub mod handlers;
pub mod helpers;
pub mod router;

use std::net::SocketAddr;

use anyhow::Context;
use axum_tracing_opentelemetry::middleware::{OtelAxumLayer, OtelInResponseLayer};
use deadpool_diesel::postgres::Pool;
use std::{env, time::Duration};
use tokio::net::TcpListener;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::cors::CorsLayer;

use pragma_common::services::{Service, ServiceRunner};

use docs::ApiDoc;
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

            #[allow(clippy::default_constructed_unit_structs)]
            let app = {
                let base = api_router::<ApiDoc>(state.clone())
                    .with_state(state)
                    // include trace context as header into the response
                    //start OpenTelemetry trace on incoming request
                    .layer(OtelAxumLayer::default())
                    .layer(OtelInResponseLayer::default());

                let base = if limiter_enabled {
                    // Configure rate limiting from env and expose x-ratelimit headers
                    let governor_conf = GovernorConfigBuilder::default()
                        .per_second(per_second)
                        .burst_size(burst_size)
                        .use_headers()
                        .finish()
                        .expect("failed to build governor config");

                    // Periodic cleanup of the limiter's internal storage, with graceful shutdown.
                    let governor_limiter = governor_conf.limiter().clone();
                    let cancel_token = ctx.token.clone();
                    tokio::spawn(async move {
                        let mut ticker = tokio::time::interval(Duration::from_secs(cleanup_secs));
                        loop {
                            tokio::select! {
                                _ = ticker.tick() => {
                                    tracing::info!("rate limiting storage size: {}", governor_limiter.len());
                                    governor_limiter.retain_recent();
                                }
                                _ = cancel_token.cancelled() => {
                                    tracing::debug!("rate limiter cleanup task shutting down");
                                    break;
                                }
                            }
                        }
                    });

                    base.layer(GovernorLayer::new(governor_conf))
                } else {
                    tracing::info!("rate limiter disabled via env");
                    base
                };

                base.layer(CorsLayer::permissive())
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
