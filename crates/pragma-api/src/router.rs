use axum::Router;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;

use utoipa::OpenApi as OpenApiT;
use utoipa_swagger_ui::SwaggerUi;

use crate::{AppState, handlers};

pub fn api_router<T: OpenApiT>(_state: AppState) -> Router<AppState> {
    let open_api = T::openapi();
    // Group vault-related endpoints under a dedicated "/vaults" router
    let vaults_router = Router::new()
        .route("/", get(handlers::list_vaults))
        .route("/{vault_id}", get(handlers::get_vault))
        .route("/{vault_id}/stats", get(handlers::get_vault_stats));

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
