use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    hypatia_mcp_server::cli::run().await?;
    Ok(())
}