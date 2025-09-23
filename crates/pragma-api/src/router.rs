use axum::Router;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};

use utoipa::OpenApi as OpenApiT;
use utoipa_swagger_ui::SwaggerUi;

use crate::{AppState, handlers};

pub fn api_router<T: OpenApiT>(_state: AppState) -> Router<AppState> {
    let open_api = T::openapi();
    // Group vault-related endpoints under a dedicated "/vaults" router
    let vaults_router = Router::new()
        .route("/", get(handlers::list_vaults))
        .route("/{vault_id}", get(handlers::get_vault))
        .route("/{vault_id}/stats", get(handlers::get_vault_stats))
        .route(
            "/{vault_id}/timeseries",
            get(handlers::get_vault_timeseries),
        )
        .route("/{vault_id}/kpis", get(handlers::get_vault_kpis))
        .route("/{vault_id}/liquidity", get(handlers::get_vault_liquidity))
        .route(
            "/{vault_id}/liquidity/curve",
            get(handlers::get_vault_slippage_curve),
        )
        .route(
            "/{vault_id}/liquidity/simulate",
            post(handlers::simulate_vault_liquidity),
        )
        .route("/{vault_id}/apr/summary", get(handlers::get_vault_apr_summary))
        .route("/{vault_id}/apr/series", get(handlers::get_vault_apr_series))
        .route("/{vault_id}/composition", get(handlers::get_vault_composition))
        .route(
            "/{vault_id}/composition/series",
            get(handlers::get_vault_composition_series),
        )
        .route("/{vault_id}/caps", get(handlers::get_vault_caps))
        .route("/{vault_id}/nav/latest", get(handlers::get_vault_nav_latest));

    // Group user-related endpoints under a dedicated "/users" router
    let users_router = Router::new()
        .route("/{address}", get(handlers::get_user_profile))
        .route(
            "/{address}/vaults/{vault_id}/summary",
            get(handlers::get_user_position_summary),
        )
        .route(
            "/{address}/vaults/{vault_id}/transactions",
            get(handlers::get_user_transaction_history),
        )
        .route(
            "/{address}/vaults/{vault_id}/kpis",
            get(handlers::get_user_kpis),
        )
        .route(
            "/{address}/vaults/{vault_id}/historical",
            get(handlers::get_historical_user_performance),
        );

    Router::new()
        .route("/health", get(health))
        .nest("/v1/vaults", vaults_router)
        .nest("/v1/users", users_router)
        .merge(SwaggerUi::new("/v1/docs").url("/v1/docs/openapi.json", open_api))
        .fallback(handler_404)
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn handler_404() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        "The requested resource was not found",
    )
}
