use anyhow::Result;
use serde_json::to_string_pretty;
use std::path::PathBuf;
use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::{ServerBuilder, ServerVariableBuilder};
use utoipauto::utoipauto;

pub struct ServerAddon;

impl Modify for ServerAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let server_variable = ServerVariableBuilder::new()
            .default_value("devnet")
            .enum_values(Some(vec!["devnet", "production"]))
            .build();
        openapi.servers = Some(vec![
            ServerBuilder::new()
                .url("https://{environment}.0d.finance/api/v1")
                .parameter("environment", server_variable)
                .build(),
        ]);
    }
}

#[utoipauto(paths = "./crates/pragma-api/src/")]
#[derive(OpenApi)]
#[openapi(
    modifiers(&ServerAddon),
    tags(
        (name = "pragma-bin", description = "0d, master api"),
        (name = "Vaults", description = "Vault management endpoints"),
        (name = "User", description = "User profile endpoints"),
    )
)]
pub struct ApiDoc;

impl ApiDoc {
    #[allow(dead_code)]
    pub fn generate_openapi_json(output_path: PathBuf) -> Result<()> {
        let openapi = Self::openapi();
        let json = to_string_pretty(&openapi)?;

        let file_path = output_path.join("openapi.json");

        tracing::info!("Saving OpenAPI specs to {}...", file_path.display());

        std::fs::write(&file_path, json)?;
        tracing::info!("OpenAPI specs saved!");
        Ok(())
    }
}
