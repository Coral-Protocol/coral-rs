use crate::completion_evaluated_prompt::CompletionEvaluatedPrompt;
use crate::error::Error;
use rig::tool::rmcp::McpTool;
use rmcp::model::{
    ClientInfo, Implementation, ProtocolVersion, ReadResourceRequestParam, ResourceContents,
};
use rmcp::service::RunningService;
use rmcp::transport::{ConfigureCommandExt, SseClientTransport, TokioChildProcess};
use rmcp::{RoleClient, ServiceExt};
use std::sync::Arc;
use tokio::process::Command;

pub struct McpConnectionBuilder {
    client_info: ClientInfo,
    transport: McpTransport,
    revalidate_tooling: bool,
    skip_tooling: bool,
}

struct SseTransport {
    url: String,
}

struct StdioTransport {
    executable: String,
    arguments: Vec<String>,
    identifier: String,
}

enum McpTransport {
    Sse(SseTransport),
    Stdio(StdioTransport),
}

impl McpConnectionBuilder {
    fn new(transport: McpTransport) -> Self {
        Self {
            client_info: ClientInfo {
                protocol_version: Default::default(),
                capabilities: Default::default(),
                client_info: Implementation::from_build_env(),
            },
            transport,
            revalidate_tooling: false,
            skip_tooling: false,
        }
    }

    ///
    /// Creates a new MCP connection builder using an SSE transport
    pub fn sse(url: impl Into<String>) -> Self {
        Self::new(McpTransport::Sse(SseTransport { url: url.into() }))
    }

    ///
    /// Creates a new MCP connection builder using a child process (stdio transport)
    pub fn stdio(
        executable: impl Into<String>,
        arguments: Vec<&str>,
        identifier: impl Into<String>,
    ) -> Self {
        Self::new(McpTransport::Stdio(StdioTransport {
            executable: executable.into(),
            arguments: arguments.iter().map(|x| x.to_string()).collect(),
            identifier: identifier.into(),
        }))
    }

    ///
    /// Helper function to set up a connection with the Coral MCP server.  This is designed to be
    /// used when the agent is orchestrated with Coral.  CORAL_CONNECTION_URL is set by the Coral
    /// server and is required for this function to work.  If CORAL_CONNECTION_URL is not set, this
    /// function will panic.
    pub fn from_coral_env() -> Self {
        Self::sse(std::env::var("CORAL_CONNECTION_URL").expect("CORAL_CONNECTION_URL not set"))
            .protocol_version(ProtocolVersion::V_2024_11_05)
    }

    ///
    /// MCP server Protocol.  The Coral MCP server currently requires that this is set to
    /// [`ProtocolVersion::V_2024_11_05`]
    pub fn protocol_version(mut self, protocol_version: ProtocolVersion) -> Self {
        self.client_info.protocol_version = protocol_version;
        self
    }

    ///
    /// The name of the agent as exposed to other agents on the MCP server
    pub fn name(mut self, name: String) -> Self {
        self.client_info.client_info.name = name;
        self
    }

    ///
    /// The version of the agent as exposed to other agents on the MCP server
    pub fn version(mut self, version: String) -> Self {
        self.client_info.client_info.version = version;
        self
    }

    ///
    /// Full client info struct used internally by RMCP
    pub fn client_info(mut self, client_info: ClientInfo) -> Self {
        self.client_info = client_info;
        self
    }

    ///
    /// Set to true if this MCP server should revalidate its tooling before making requests.
    /// Coral servers should not have this set to true.
    pub fn revalidate_tooling(mut self, revalidate_tooling: bool) -> Self {
        self.revalidate_tooling = revalidate_tooling;
        self
    }

    ///
    /// Skips processing tooling from this MCP server.  This must be used on servers that do not
    /// support tooling.
    pub fn skip_tooling(mut self, skip_tooling: bool) -> Self {
        self.skip_tooling = skip_tooling;
        self
    }

    ///
    /// Builds the connection builder into a connection to an MCP server
    pub async fn connect(self) -> Result<McpServerConnection, Error> {
        match self.transport {
            McpTransport::Sse(sse) => {
                let transport = SseClientTransport::start(sse.url.clone())
                    .await
                    .map_err(Error::McpSseError)?;

                let transport = self
                    .client_info
                    .serve(transport)
                    .await
                    .map_err(Error::McpClientError)?;

                Ok(McpServerConnection::new(
                    transport,
                    self.revalidate_tooling,
                    self.skip_tooling,
                    sse.url.clone(),
                )
                .into())
            }
            McpTransport::Stdio(stdio) => {
                let cmd = Command::new(stdio.executable).configure(|c| {
                    c.args(&stdio.arguments);
                });

                let transport = TokioChildProcess::new(cmd).map_err(Error::McpStdioError)?;

                let transport = self
                    .client_info
                    .serve(transport)
                    .await
                    .map_err(Error::McpClientError)?;

                Ok(McpServerConnection::new(
                    transport,
                    self.revalidate_tooling,
                    self.skip_tooling,
                    stdio.identifier,
                )
                .into())
            }
        }
    }
}

///
/// Represents a live connection to an MCP server.
#[derive(Clone)]
pub struct McpServerConnection {
    running_service: Arc<RunningService<RoleClient, ClientInfo>>,
    pub(crate) revalidate_tooling: bool,
    pub(crate) skip_tooling: bool,
    pub(crate) identifier: String,
}

impl McpServerConnection {
    fn new(
        running_service: RunningService<RoleClient, ClientInfo>,
        revalidate_tooling: bool,
        skip_tooling: bool,
        identifier: String,
    ) -> Self {
        Self {
            running_service: Arc::new(running_service),
            revalidate_tooling,
            skip_tooling,
            identifier,
        }
    }

    ///
    /// Returns a list of tooling that this MCP server provides.  Note that a tool must live as long
    /// as the connection does.  The MCP connection wrapped in this struct therefore remains alive
    /// for as long as tooling returned by this function does.
    pub(crate) async fn get_tools(&self) -> Result<Vec<McpTool>, Error> {
        Ok(self
            .running_service
            .list_all_tools()
            .await
            .map_err(Error::McpServiceError)?
            .into_iter()
            .map(|x| McpTool::from_mcp_server(x, self.running_service.peer().clone()))
            .collect())
    }

    ///
    /// Returns a list of resolved resources from this MCP server
    pub(crate) async fn get_resources(&self) -> Result<Vec<ResourceContents>, Error> {
        let resource_list = self
            .running_service
            .list_all_resources()
            .await
            .map_err(Error::McpServiceError)?;

        let mut resource_content_list = Vec::new();
        for resource in resource_list {
            let contents = self
                .running_service
                .read_resource(ReadResourceRequestParam {
                    uri: resource.uri.clone(),
                })
                .await
                .map_err(Error::McpServiceError)?
                .contents;

            resource_content_list.extend(contents);
        }

        Ok(resource_content_list)
    }

    ///
    /// Reads a single URI-referenced resource from this connection
    pub(crate) async fn read_resource(
        &self,
        uri: impl Into<String>,
    ) -> Result<Vec<ResourceContents>, Error> {
        Ok(self
            .running_service
            .read_resource(ReadResourceRequestParam { uri: uri.into() })
            .await
            .map_err(Error::McpServiceError)?
            .contents)
    }

    ///
    /// Quick helper function to create a [`CompletionEvaluatedPrompt`] from this MCP connection,
    /// this will include an [`CompletionEvaluatedPrompt::all_resources`] call from this MCP
    /// connection, which is recommended for Coral MCP connections.
    ///
    /// This prompt will start with a passed in string
    pub fn prompt_with_resources_str(
        &self,
        prompt: impl Into<String>,
    ) -> CompletionEvaluatedPrompt {
        CompletionEvaluatedPrompt::from_string(prompt).all_resources(self.clone())
    }

    ///
    /// Helper function to create an empty [`CompletionEvaluatedPrompt`] prompt that contains
    /// nothing but all the resources provided by this MCP server.  This function is useful when
    /// making a very basic agent that only Coral resources as the preamble.  
    pub fn prompt_with_resources(&self) -> CompletionEvaluatedPrompt {
        CompletionEvaluatedPrompt::new().all_resources(self.clone())
    }
}
