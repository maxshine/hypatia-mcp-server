use rmcp::ErrorData as McpError;
use rmcp::model::{
    ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParams,
    ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult, Resource,
    ResourceContents, ResourceTemplate, ServerCapabilities, ServerConfig,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ServerHandler, prompt_handler, tool_handler};

#[derive(Clone, Default)]
pub struct CombinedServer {}

const INSTRUCTIONS: &str = "Hypatia is a local knowledge graph of knowledge entries (named \
notes) and statements (head, relation, tail triples). These tools are thin graph primitives: \
when and how to remember, link and consolidate is decided by the hypatia skills, not by this \
server. The shelf argument defaults to \"default\". Writes return the shelf's embedding debt: \
vectors are generated later, so a fresh entry can be missing from `similar` until the debt is \
paid. Connecting shelves, installing models, export and import are done with the hypatia CLI; restart this server afterwards, since it reads shelf \
configuration once at start.";

const DUMMY_RESOURCE_URI: &str = "dummy://resource";

// Combines routers generated in the sibling impl files
#[tool_handler(router = (Self::calc_router() + Self::memory_router()))]
#[prompt_handler(router = Self::dummy_prompt_router())]
impl ServerHandler for CombinedServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_instructions(INSTRUCTIONS)
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let resource = Resource::new(DUMMY_RESOURCE_URI, "dummy_resource")
            .with_description("A placeholder resource")
            .with_mime_type("text/plain");
        Ok(ListResourcesResult::with_all_items(vec![resource]))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, McpError> {
        if request.uri != DUMMY_RESOURCE_URI {
            return Err(McpError::resource_not_found(
                format!("no such resource: {}", request.uri),
                None,
            ));
        }
        let contents = ResourceContents::text("this is a dummy resource", DUMMY_RESOURCE_URI);
        Ok(ReadResourceResult::new(vec![contents]).into())
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        let template = ResourceTemplate::new("dummy://{id}", "dummy_template")
            .with_description("A placeholder resource template");
        Ok(ListResourceTemplatesResult::with_all_items(vec![template]))
    }
}
