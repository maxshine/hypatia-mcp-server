use super::combined_server::CombinedServer;
use hypatia;
use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    schemars, serde,
    serde_json::{Value, json},
    tool, tool_router,
};

// type ToolResult = Result<Value, String>;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ShelfNameParams {
    shelf_name: String,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct ShelfList {
    shelves: Vec<String>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct ShelfStatus {
    name: String,
    postgres: bool,
    embedder: String,
    semantic_search_off: String,
    attention: String,
    has_debt: bool,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct ModelList {
    models: Vec<String>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct ToolResult {
    result: Value,
}
const DEFAULT_SHELF: &str = "default";

fn dirs_home() -> std::path::PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

fn canonicalize_shelf_name(shelf: Option<String>) -> String {
    shelf.unwrap_or_else(|| DEFAULT_SHELF.to_string())
}

#[tool_router(router = memory_router, vis = "pub")]
impl CombinedServer {
    #[tool(description = "Connect to a hypatia memory shelf")]
    fn connect_shelf(
        &self,
        Parameters(ShelfNameParams { shelf_name }): Parameters<ShelfNameParams>,
    ) -> String {
        let mut hypatia_lab = hypatia::lab::Lab::new().unwrap();
        let shelf_path = dirs_home().join(".hypatia").join(&shelf_name);
        let result = hypatia_lab
            .connect_shelf(&shelf_path, Some(shelf_name.as_str()))
            .unwrap();
        format!("{} hypatia shelf connected", result)
    }
    #[tool(description = "List available hypatia memory shelves")]
    fn list_shelves(&self) -> Json<ShelfList> {
        let hypatia_lab = hypatia::lab::Lab::new().unwrap();
        let shelves = hypatia_lab.list_shelves();
        let ret: Vec<String> = shelves
            .iter()
            .map(|(name, _, _)| name.to_string())
            .collect();
        Json(ShelfList { shelves: ret })
    }
    #[tool(description = "Disconnect a hypatia memory shelf from registry")]
    fn disconnect_shelf(
        &self,
        Parameters(ShelfNameParams { shelf_name }): Parameters<ShelfNameParams>,
    ) -> String {
        let mut hypatia_lab = hypatia::lab::Lab::new().unwrap();
        hypatia_lab.disconnect_shelf(shelf_name.as_str()).unwrap();
        format!("{} hypatia shelf disconnected", shelf_name)
    }

    #[tool(description = "Show a shelf general status")]
    fn shelf_status(
        &self,
        Parameters(ShelfNameParams { shelf_name }): Parameters<ShelfNameParams>,
    ) -> Json<ShelfStatus> {
        let hypatia_lab = hypatia::lab::Lab::new().unwrap();
        let status = hypatia_lab.shelf_status(&shelf_name).unwrap();
        Json(ShelfStatus {
            name: status.name,
            postgres: status.postgres,
            embedder: status.embedder,
            semantic_search_off: status.semantic_search_off.unwrap_or_default(),
            attention: status.attention.unwrap_or_default(),
            has_debt: status.debt.pending_knowledge > 0 || status.debt.pending_statement > 0,
        })
    }

    #[tool(description = "List local models available for hypatia memory")]
    fn model_list(&self) -> Json<ModelList> {
        let models: Vec<String> = hypatia::embedding::config::list_local_models()
            .into_iter()
            .map(|(name, _)| name.to_string())
            .collect();
        Json(ModelList { models })
    }

    #[tool(description = "List archives for a hypatia memory shelf")]
    fn list_archive(
        &self,
        Parameters(ShelfNameParams { shelf_name }): Parameters<ShelfNameParams>,
    ) -> Json<ToolResult> {
        let shelf = canonicalize_shelf_name(Some(shelf_name));
        let mut hypatia_lab = hypatia::lab::Lab::new().unwrap();
        hypatia_lab.flush_if_overdue(&shelf).unwrap();
        let files: Vec<String> = hypatia_lab
            .list_archives(&shelf)
            .unwrap()
            .into_iter()
            .map(|f| format!("archive://{f}"))
            .collect();
        Json(ToolResult {
            result: json!({ "files": files }),
        })
    }
}
