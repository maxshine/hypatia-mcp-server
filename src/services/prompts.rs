use super::combined_server::CombinedServer;
use rmcp::model::{PromptMessage, Role};
use rmcp::{prompt, prompt_router};

#[prompt_router(router = "dummy_prompt_router", vis = "pub")]
impl CombinedServer {
    #[prompt(name = "dummy_prompt", description = "A placeholder prompt")]
    async fn dummy_prompt(&self) -> Vec<PromptMessage> {
        vec![PromptMessage::new_text(
            Role::User,
            "this is a dummy prompt",
        )]
    }
}
