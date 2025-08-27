use crate::error::Error;
use rig::tool::rmcp::McpTool;
use rmcp::model::{ClientInfo, Implementation, ProtocolVersion, ReadResourceRequestParam, ResourceContents};
use rmcp::service::RunningService;
use rmcp::transport::{ConfigureCommandExt, SseClientTransport, TokioChildProcess};
use rmcp::{RoleClient, ServiceExt};
use std::sync::Arc;
use tokio::process::Command;

pub struct McpConnectionBuilder {
    client_info: ClientInfo,
    transport: McpTransport,
    revalidate_tooling: bool,
    revalidate_resources: bool,
    skip_tooling: bool,
    skip_resources: bool,
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
                client_info: Implementation::from_build_env()
            },
            transport,
            revalidate_tooling: false,
            revalidate_resources: false,
            skip_tooling: false,
            skip_resources: false,
        }
    }

    ///
    /// Creates a new MCP connection builder using an SSE transport
    pub fn sse(url: impl Into<String>) -> Self {
        Self::new(McpTransport::Sse(SseTransport {
            url: url.into(),
        }))
    }

    ///
    /// Creates a new MCP connection builder using a child process (stdio transport)
    pub fn stdio(
        executable: impl Into<String>,
        arguments: Vec<&str>,
        identifier: impl Into<String>
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
        Self::sse(std::env::var("CORAL_CONNECTION_URL")
            .expect("CORAL_CONNECTION_URL not set"))
            .protocol_version(ProtocolVersion::V_2024_11_05)
            .revalidate_resources(true)
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
    /// Set to true if this MCP server should revalidate its resources before making requests.
    /// Coral servers should always have this set to true.
    pub fn revalidate_resources(mut self, revalidate_resources: bool) -> Self {
        self.revalidate_resources = revalidate_resources;
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
    /// Skips processing resources from this MCP server.  This must be used on servers that do not
    /// support resources.
    pub fn skip_resources(mut self, skip_resources: bool) -> Self {
        self.skip_resources = skip_resources;
        self
    }

    ///
    /// Builds the connection builder into a connection to an MCP server
    pub async fn connect(self) -> Result<McpServerConnection, Error> {
        match self.transport {
            McpTransport::Sse(sse) => {
                let transport = SseClientTransport::start(sse.url.clone()).await
                    .map_err(Error::McpSseError)?;

                let transport = self.client_info
                    .serve(transport)
                    .await
                    .map_err(Error::McpClientError)?;

                Ok(McpServerConnection::new(
                    transport,
                    self.revalidate_tooling,
                    self.revalidate_resources,
                    self.skip_tooling,
                    self.skip_resources,
                    sse.url.clone()
                ))
            }
            McpTransport::Stdio(stdio) => {
                let cmd = Command::new(stdio.executable).configure(|c| {
                    c.args(&stdio.arguments);
                });

                let transport = TokioChildProcess::new(cmd)
                    .map_err(Error::McpStdioError)?;

                let transport = self.client_info
                    .serve(transport)
                    .await
                    .map_err(Error::McpClientError)?;

                Ok(McpServerConnection::new(
                    transport,
                    self.revalidate_tooling,
                    self.revalidate_resources,
                    self.skip_tooling,
                    self.skip_resources,
                    stdio.identifier
                ))
            }
        }
    }
}

///
/// Represents a live connection to an MCP server.
pub struct McpServerConnection {
    running_service: Arc<RunningService<RoleClient, ClientInfo>>,
    pub(crate) revalidate_tooling: bool,
    pub(crate) revalidate_resources: bool,
    pub(crate) skip_tooling: bool,
    pub(crate) skip_resources: bool,
    pub(crate) identifier: String,
}

impl McpServerConnection {
    fn new(
        running_service: RunningService<RoleClient, ClientInfo>,
        revalidate_tooling: bool,
        revalidate_resources: bool,
        skip_tooling: bool,
        skip_resources: bool,
        identifier: String,
    ) -> Self {
        Self {
            running_service: Arc::new(running_service),
            revalidate_tooling,
            revalidate_resources,
            skip_tooling,
            skip_resources,
            identifier
        }
    }

    ///
    /// Returns a list of tooling that this MCP server provides.  Note that a tool must live as long
    /// as the connection does.  The MCP connection wrapped in this struct therefore remains alive
    /// for as long as tooling returned by this function does.
    pub(crate) async fn get_tools(&self) -> Result<Vec<McpTool>, Error> {
        Ok(self.running_service.list_all_tools()
            .await.map_err(Error::McpServiceError)?
            .into_iter()
            .map(|x| McpTool::from_mcp_server(x, self.running_service.peer().clone()))
            .collect())
    }

    ///
    /// Returns a list of resolved resources from this MCP server
    pub(crate) async fn get_resources(&self) -> Result<Vec<ResourceContents>, Error> {
        let resource_list = self.running_service.list_all_resources().await
            .map_err(Error::McpServiceError)?;

        let mut resource_content_list = Vec::new();
        for resource in resource_list {
            let contents = self.running_service.read_resource(ReadResourceRequestParam {
                uri: resource.uri.clone(),
            }).await.map_err(Error::McpServiceError)?.contents;

            resource_content_list.extend(contents);
        }

        Ok(resource_content_list)
    }
}