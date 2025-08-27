use rig::tool::ToolSetError;
use rmcp::service::ClientInitializeError;
use rmcp::transport::sse_client::SseTransportError;
use rmcp::ServiceError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("mcp error: {0}")]
    McpClientError(ClientInitializeError),

    #[error("mcp error: {0}")]
    McpSseError(SseTransportError<reqwest::Error>),

    #[error("mcp error: {0}")]
    McpStdioError(std::io::Error),
    
    #[error("mcp error: {0}")]
    McpServiceError(ServiceError),

    #[error("completion error: {0}")]
    PromptError(rig::completion::PromptError),

    #[error("completion error: {0}")]
    CompletionError(rig::completion::CompletionError),

    #[error("tool error: {0}")]
    ToolsetError(ToolSetError)
}