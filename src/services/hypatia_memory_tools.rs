use super::combined_server::CombinedServer;
use hypatia::lab::Lab;
use rmcp::{handler::server::wrapper::Parameters, schemars, serde, tool, tool_router};
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ConnectParams {
    shelf_name: String,
}

fn dirs_home() -> std::path::PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

#[tool_router(router = memory_router, vis = "pub")]
impl CombinedServer {
    #[tool(description = "Connect to a hypatia memory shelf")]
    fn connect(
        &self,
        Parameters(ConnectParams { shelf_name }): Parameters<ConnectParams>,
    ) -> String {
        let mut hypatia_lab = Lab::new().unwrap();
        let shelf_path = dirs_home().join(".hypatia").join(&shelf_name);
        let result = hypatia_lab
            .connect_shelf(&shelf_path, Some(shelf_name.as_str()))
            .unwrap();
        format!("{} hypatia shelf connected", result)
    }
    #[tool(description = "List available hypatia memory shelves")]
    fn list(&self) -> String {
        let hypatia_lab = Lab::new().unwrap();
        let shelves = hypatia_lab.list_shelves();
        let mut ret: Vec<String> = Vec::new();
        for (name, _, _) in &shelves {
            ret.push((*name).to_string());
        }
        let sentence: String = ret.join(",");
        format!("{} hypatia shelves", sentence)
    }
    #[tool(description = "Disconnect a hypatia memory shelf from registry")]
    fn disconnect(
        &self,
        Parameters(ConnectParams { shelf_name }): Parameters<ConnectParams>,
    ) -> String {
        let mut hypatia_lab = Lab::new().unwrap();
        hypatia_lab.disconnect_shelf(shelf_name.as_str()).unwrap();
        format!("{} hypatia shelf disconnected", shelf_name)
    }
}
