use rmcp::{handler::server::wrapper::Parameters, schemars, serde, tool, tool_router};

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
