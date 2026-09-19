use rmcp::{ServerHandler, tool_handler};

#[derive(Clone, Default)]
pub struct CombinedServer {}

// Combines routers generated in the sibling impl files
#[tool_handler(router = (Self::calc_router() + Self::memory_router()))]
impl ServerHandler for CombinedServer {}
