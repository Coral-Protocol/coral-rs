use crate::api::generated::types::McpResources;
use crate::error::Error;
use crate::mcp_server::McpServerConnection;
use rmcp::model::ResourceContents;

///
/// A CompletionEvaluatedPrompt is made up of many [`PromptPart`] parts that will be evaluated by
/// [`crate::agent::Agent`] before it sends a completion request to a completion model.
///
/// There are many reasons to have a prompt evaluated (as close as possible) to before completion,
/// in Coral specifically this is useful because Coral offers resources that are "live" and can
/// impact the completion greatly.
///
/// A newline will separate all parts in a CompletionEvaluatedPrompt when evaluated.
///
/// A CompletionEvaluatedPrompt can be evaluated many times, each time creating a new string, using
/// the [`CompletionEvaluatedPrompt::evaluate`] function.
#[derive(Clone)]
pub struct CompletionEvaluatedPrompt {
    pub parts: Vec<PromptPart>,
}

#[derive(Clone)]
pub struct ResourceData {
    mcp_server_connection: McpServerConnection,
    resource_uri: String,
}

#[derive(Clone)]
pub enum PromptPart {
    ///
    /// A basic string
    String(String),

    ///
    /// A URI-referenced resource from a specific MCP server
    Resource(ResourceData),

    ///
    /// All resources on a specific MCP server
    AllResources(McpServerConnection),
}

impl CompletionEvaluatedPrompt {
    pub fn new() -> Self {
        Self { parts: Vec::new() }
    }

    ///
    /// Creates a new prompt starting with a single [`PromptPart::String`] part.
    pub fn from_string(string: impl Into<String>) -> Self {
        Self {
            parts: vec![PromptPart::String(string.into())],
        }
    }

    ///
    /// Appends a [`PromptPart::String`] part to this prompt.
    pub fn string(mut self, string: impl Into<String>) -> Self {
        self.parts.push(PromptPart::String(string.into()));
        self
    }

    ///
    /// Adds a single URI-referenced resource from an MCP server as a part of this dynamic prompt.
    pub fn resource(
        mut self,
        mcp_server_connection: McpServerConnection,
        resource_uri: impl Into<String>,
    ) -> Self {
        self.parts.push(PromptPart::Resource(ResourceData {
            mcp_server_connection,
            resource_uri: resource_uri.into(),
        }));
        self
    }

    ///
    /// Helper function to add a Coral resource using the [`McpResources`] enum.
    pub fn coral_resource(
        self,
        mcp_server_connection: McpServerConnection,
        resource: McpResources,
    ) -> Self {
        self.resource(mcp_server_connection, resource.to_string())
    }

    ///
    /// Adds a special part indicating that all resources from an MCP connection should be added to
    /// a prompt.  Note that the list of resources is calculated when
    /// [`CompletionEvaluatedPrompt::evaluate`] is called and not when this function is called.
    pub fn all_resources(mut self, mcp_server_connection: McpServerConnection) -> Self {
        self.parts
            .push(PromptPart::AllResources(mcp_server_connection));
        self
    }

    ///
    /// Helper function to convert a list of resource contents into a newline-separated string
    fn resource_contents_to_string(resource_contents: Vec<ResourceContents>) -> String {
        resource_contents
            .iter()
            .map(|x| {
                match x {
                    ResourceContents::TextResourceContents { text, .. } => text,
                    ResourceContents::BlobResourceContents { blob, .. } => blob,
                }
                .clone()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    ///
    /// Evaluates all parts into a new string.
    ///
    /// This function can return an [`Error`] if the prompt contains resources that fail
    /// to get fetched here.  The potential for resources is also the reason that this function is
    /// async.
    ///
    /// A newline character will separate all parts in this prompt when evaluated.
    pub async fn evaluate(&self) -> Result<String, Error> {
        let mut buffer = String::new();
        for part in &self.parts {
            buffer.push_str(
                match part {
                    PromptPart::String(string) => string.clone(),
                    PromptPart::Resource(resource_data) => Self::resource_contents_to_string(
                        resource_data
                            .mcp_server_connection
                            .read_resource(&resource_data.resource_uri)
                            .await?,
                    ),
                    PromptPart::AllResources(mcp_server_connection) => {
                        Self::resource_contents_to_string(
                            mcp_server_connection.get_resources().await?,
                        )
                    }
                }
                .as_str(),
            );
            buffer.push('\n');
        }

        Ok(buffer)
    }
}

impl Default for CompletionEvaluatedPrompt {
    fn default() -> Self {
        Self::new()
    }
}

