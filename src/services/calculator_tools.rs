use super::combined_server::CombinedServer;
use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::ErrorData,
    schemars, serde,
    serde_json::{self as serde_json, Map, Value, json},
    tool, tool_router,
};
use std::{env, fs::OpenOptions, io::Write};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct AddParams {
    a: i32,
    b: i32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct EchoParams {
    content: String,
}

// #[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
// struct HookEventParams {
//     #[serde(flatten)]
//     event: HashMap<String, Value>,
// }

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

    #[tool(description = "Dump agentic hook event content")]
    fn dump_agent_hook_event(
        &self,
        Parameters(event): Parameters<Map<String, Value>>,
    ) -> Result<Json<Value>, ErrorData> {
        let mut temp_path = env::temp_dir();
        temp_path.push("dump_hook_event.log");
        let mut dump_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&temp_path)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let formatted_time = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        let formatted_event = serde_json::to_string(&event)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        writeln!(dump_file, "{}: {}\n", formatted_time, formatted_event)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(Json(json!({"continue": true})))
    }
}
