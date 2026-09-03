use rmcp::{handler::server::wrapper::Parameters, schemars, serde, tool, tool_router};
use super::combined_server::CombinedServer;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ConnectParams { shelf_name: String }

#[tool_router(router = memory_router, vis = "pub")]
impl CombinedServer {
    #[tool(description = "Connect to a shelf")]
    fn connect(&self, Parameters(ConnectParams { shelf_name }): Parameters<ConnectParams>) -> String {
        shelf_name
    }
}