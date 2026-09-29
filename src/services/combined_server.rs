use rmcp::model::{ServerCapabilities, ServerConfig};
use rmcp::{ServerHandler, tool_handler};

#[derive(Clone, Default)]
pub struct CombinedServer {}

const INSTRUCTIONS: &str = "Hypatia is a local knowledge graph of knowledge entries (named \
notes) and statements (head, relation, tail triples). These tools are thin graph primitives: \
when and how to remember, link and consolidate is decided by the hypatia skills, not by this \
server. The shelf argument defaults to \"default\". Writes return the shelf's embedding debt: \
vectors are generated later, so a fresh entry can be missing from `similar` until the debt is \
paid. Connecting shelves, installing models, export and import are done with the hypatia CLI; restart this server afterwards, since it reads shelf \
configuration once at start.";

// Combines routers generated in the sibling impl files
#[tool_handler(router = (Self::calc_router() + Self::memory_router()))]
impl ServerHandler for CombinedServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(INSTRUCTIONS)
    }
}
