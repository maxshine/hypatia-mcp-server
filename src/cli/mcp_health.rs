use std::time::Duration;

use anyhow::{Context, Result};
use rmcp::{
    ClientLifecycleMode, ClientServiceExt,
    model::{ClientConfig, ProtocolVersion},
    transport::StreamableHttpClientTransport,
};
use tokio::time::timeout;

pub async fn check_mcp_health(url: String, deadline: u64) -> Result<()> {
    let transport = StreamableHttpClientTransport::from_uri(url);

    // This sends server/discover and waits for a valid response.
    // No legacy initialize handshake or ping is sent.
    let client = timeout(
        Duration::from_secs(deadline),
        ClientConfig::default().serve_with_lifecycle(
            transport,
            ClientLifecycleMode::Auto {
                preferred_versions: vec![ProtocolVersion::V_2026_07_28],
                legacy_version: Some(ProtocolVersion::V_2025_11_25),
            },
        ),
    )
    .await
    .context("MCP connection/negotiation timeout")?
    .context("MCP connection/negotiation failed")?;
    client
        .list_tools(None)
        .await
        .context("MCP tools/list failed")?;

    // The probe has succeeded. Cleanup gets a separate, short timeout
    // and cannot change the health result.
    let _ = timeout(Duration::from_millis(500), client.cancel()).await;
    Ok(())
}
