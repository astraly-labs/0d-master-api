use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};

use zerod_db::types::Timeframe;
use zerod_master::KpisDTO;

use crate::{
    AppState,
    dto::{ApiResponse, TimeframeQuery},
    errors::ApiError,
    helpers::{call_vault_backend, fetch_vault_with_client},
};

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/kpis",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("timeframe" = Timeframe, Query, description = "Time period for KPI calculation", example = "all")
    ),
    responses(
        (status = 200, description = "Vault performance KPIs", body = KpisDTO),
        (status = 400, description = "Invalid parameters"),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_kpis(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
    Query(params): Query<TimeframeQuery>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::info!("Getting vault KPIs for {:?}", params);

    let (vault, client) = fetch_vault_with_client(&state, &vault_id).await?;
    let timeframe = params.timeframe.as_str().to_owned();
    let kpis = call_vault_backend(&client, &vault, "fetch vault KPIs", move |backend| {
        let timeframe = timeframe.clone();
        async move { backend.get_vault_kpis(&timeframe).await }
    })
    .await?;

    Ok(Json(ApiResponse::ok(kpis)))
}
