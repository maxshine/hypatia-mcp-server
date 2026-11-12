use crate::cli::init_shelf::run_init_shelf;
use crate::cli::mcp_health::check_mcp_health;
use crate::services::{CombinedServer, DEFAULT_SHELF};
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
        /// target port for the MCP server, default is 8000
        #[arg(short, long, default_value_t = 8000)]
        port: u16,
        /// bind address for the MCP server, default is localhost
        #[arg(short, long, default_value_t = "localhost".to_string())]
        address: String,
        /// shelf used by the memory hooks and by tool calls that omit `shelf`, default is `default`
        #[arg(short = 's', long, default_value_t = DEFAULT_SHELF.to_string(), value_parser = parse_shelf_name)]
        default_shelf: String,
    },
    /// Initialize a memory shelf with desired name
    InitShelf {
        /// logical name for the shelf to be initialized
        #[arg(short, long, default_value_t = DEFAULT_SHELF.to_string())]
        shelf_name: String,
    },
    /// Check the target MCP server liveness
    CheckHealth {
        /// target port for the MCP server, default is 8000
        #[arg(short, long, default_value_t = 8000)]
        port: u16,
        /// bind address for the MCP server, default is localhost
        #[arg(short, long, default_value_t = "localhost".to_string())]
        address: String,
        /// whether to use a secure connection (HTTPS) for the MCP server
        #[arg(short, long, default_value_t = false)]
        secure: bool,
        /// timeout for the health check in seconds
        #[arg(short, long, default_value_t = 5)]
        timeout_seconds: u64,
    },
}

/// A shelf name is a single directory name under `~/.hypatia`, so it cannot hold a path.
fn parse_shelf_name(raw: &str) -> Result<String, String> {
    if raw.is_empty() || raw == "." || raw == ".." || raw.contains(['/', '\\']) {
        return Err(format!(
            "invalid shelf name '{raw}': must be non-empty and contain no path separators"
        ));
    }
    Ok(raw.to_string())
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
        Commands::Serve {
            port,
            address,
            default_shelf,
        } => {
            println!(
                "Starting MCP server at {}:{} with default shelf '{}'",
                address, port, default_shelf
            );
            warn_if_shelf_unregistered(&default_shelf);
            let default_shelf: Arc<str> = default_shelf.into();
            let mut config = StreamableHttpServerConfig::default();
            config.legacy_session_mode = false;
            let mcp_service = StreamableHttpService::new(
                move || Ok(CombinedServer::new(default_shelf.clone())),
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
        Commands::InitShelf { shelf_name } => {
            // Add the logic to initialize the shelf here
            run_init_shelf(Some(&shelf_name)).unwrap();
            println!("Initialized shelf with name: {}", shelf_name);
        }
        Commands::CheckHealth {
            port,
            address,
            secure,
            timeout_seconds,
        } => {
            let scheme = if secure { "https" } else { "http" };
            let url = format!("{}://{}:{}/mcp", scheme, address, port);
            match check_mcp_health(url, timeout_seconds).await {
                Ok(()) => println!("MCP health OK: tools/list succeeded"),
                Err(error) => {
                    eprintln!("MCP health check failed: {error:#}");
                    std::process::exit(1);
                }
            }
        }
    }
    Ok(())
}

/// Hooks fail on a shelf that is not registered, so say so at startup; connecting one later
/// through the `connect_shelf` tool is still possible, hence a warning and not an error.
fn warn_if_shelf_unregistered(shelf: &str) {
    let registered = hypatia::lab::Lab::new()
        .map(|lab| lab.list_shelves().iter().any(|(name, _, _)| *name == shelf));
    if let Ok(false) = registered {
        eprintln!(
            "warning: default shelf '{shelf}' is not registered; run `hypatia-mcp-server init-shelf --shelf-name {shelf}`"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shelf_name_must_be_a_single_directory_name() {
        assert_eq!(parse_shelf_name("work").unwrap(), "work");
        for bad in ["", ".", "..", "a/b", "a\\b", "../x"] {
            assert!(parse_shelf_name(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn serve_defaults_to_the_default_shelf() {
        let cli = Cli::try_parse_from(["hypatia-mcp-server", "serve"]).unwrap();
        let Some(Commands::Serve { default_shelf, .. }) = cli.command else {
            panic!("expected serve");
        };
        assert_eq!(default_shelf, DEFAULT_SHELF);

        let cli = Cli::try_parse_from(["hypatia-mcp-server", "serve", "-s", "work"]).unwrap();
        let Some(Commands::Serve { default_shelf, .. }) = cli.command else {
            panic!("expected serve");
        };
        assert_eq!(default_shelf, "work");
    }
}
