pub mod calculator_tools;
pub mod combined_server;
pub mod hypatia_memory_hooks;
pub mod hypatia_memory_tools;
pub mod prompts;

pub use combined_server::CombinedServer;

/// Shelf used when neither the server nor a tool call names one.
pub const DEFAULT_SHELF: &str = "default";
