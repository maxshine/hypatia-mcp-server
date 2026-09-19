use super::combined_server::CombinedServer;
use rmcp::{handler::server::wrapper::Parameters, schemars, serde, tool, tool_router};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct AddParams {
    a: i32,
    b: i32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct EchoParams {
    content: String,
}

#[tool_router(router = calc_router, vis = "pub")]
impl CombinedServer {
    #[tool(description = "Add two numbers")]
    fn add(&self, Parameters(AddParams { a, b }): Parameters<AddParams>) -> String {
        (a + b).to_string()
    }

    #[tool(description = "Echo what you send")]
    fn echo(&self, Parameters(EchoParams { content }): Parameters<EchoParams>) -> String {
        content
    }
}
