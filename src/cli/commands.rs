use crate::services::CombinedServer;
use axum::Router;
use clap::{Parser, Subcommand};
use rmcp::transport::{
    StreamableHttpServerConfig,
    streamable_http_server::{session::local::LocalSessionManager, tower::StreamableHttpService},
};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Parser)]
#[command(
    name = "hypatia-mcp-server",
    about = "AI-oriented memory management MCP server",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the MCP server
    Serve {
        /// Path to shelf directory
        #[arg(short, long, default_value_t = 8000)]
        port: u16,
        /// Optional name for the shelf
        #[arg(short, long, default_value_t = "localhost".to_string())]
        address: String,
    },
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        None => {
            panic!("No command provided. Use --help for usage information.");
        }
        Some(cmd) => execute_command(cmd).await?,
    }
    Ok(())
}

async fn execute_command(cmd: Commands) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        Commands::Serve { port, address } => {
            println!("Starting MCP server at {}:{}", address, port);
            let mut config = StreamableHttpServerConfig::default();
            config.legacy_session_mode = false;
            let mcp_service = StreamableHttpService::new(
                || Ok(CombinedServer::default()),
                Arc::new(LocalSessionManager::default()),
                config,
            );
            // 4. Mount the MCP service as a Tower service.
            let app = Router::new().route_service("/mcp", mcp_service);

            // 5. Fire up the native HTTP server
            let addr = SocketAddr::from((address.parse::<std::net::IpAddr>()?, port));
            let listener = tokio::net::TcpListener::bind(addr).await?;
            println!("Streamable HTTP MCP Server running on http://{}", addr);

            axum::serve(listener, app).await?;
        }
    }
    Ok(())
}
