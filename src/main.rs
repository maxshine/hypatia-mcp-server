use axum::Router;
use rmcp::{
    handler::server::wrapper::Parameters, 
    serde, 
    schemars, 
    tool, 
    tool_router, 
    transport::{
        StreamableHttpServerConfig,
        streamable_http_server::{
            session::local::LocalSessionManager, 
            tower::StreamableHttpService
        },
    },
};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct AddParams {
    a: i32,
    b: i32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct EchoParams {
    content: std::string::String,
}

#[derive(Clone, Default)]
pub struct Calculator {}

#[tool_router(server_handler)]
impl Calculator {
    pub fn new() -> Self {
        Self {}
    }

    #[tool(description = "Add two numbers")]
    fn add(&self, Parameters(AddParams { a, b }): Parameters<AddParams>) -> String {
        (a + b).to_string()
    }

    #[tool(description = "Echo what you send")]
    fn echo(&self, Parameters(EchoParams { content }): Parameters<EchoParams>) -> String {
        content
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = StreamableHttpServerConfig::default();
    config.legacy_session_mode = false;
    let mcp_service = StreamableHttpService::new(
        || Ok(Calculator::new()),
        Arc::new(LocalSessionManager::default()),
        config,
    );
    // 4. Mount the MCP service as a Tower service.
    let app = Router::new()
        .route_service("/mcp", mcp_service);

    // 5. Fire up the native HTTP server 
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Streamable HTTP MCP Server running on http://{}", addr);
    
    axum::serve(listener, app).await?;
    Ok(())
}