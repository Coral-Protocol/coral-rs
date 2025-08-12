use crate::error::Error;
use rig::tool::rmcp::McpTool;
use rmcp::model::{ClientInfo, Implementation, ProtocolVersion, ReadResourceRequestParam, ResourceContents};
use rmcp::service::RunningService;
use rmcp::transport::SseClientTransport;
use rmcp::{RoleClient, ServiceExt};
use std::sync::Arc;

pub struct McpConnectionBuilder {
    client_info: ClientInfo,
    url: String,
    revalidate_tooling: bool,
    revalidate_resources: bool,
}

impl McpConnectionBuilder {
    pub fn new(url: String) -> Self {
        Self {
            client_info: ClientInfo {
                protocol_version: Default::default(),
                capabilities: Default::default(),
                client_info: Implementation {
                    name: env!("CARGO_PKG_NAME").to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
            },
            url,
            revalidate_tooling: false,
            revalidate_resources: false,
        }
    }

    ///
    /// Helper function to set up a connection with the Coral MCP server.  This is designed to be
    /// used when the agent is orchestrated with Coral.  CORAL_CONNECTION_URL is set by the Coral
    /// server and is required for this function to work.  If CORAL_CONNECTION_URL is not set, this
    /// function will panic.
    pub fn from_coral_env() -> Self {
        Self::new(std::env::var("CORAL_CONNECTION_URL")
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
    /// Builds the connection builder into a connection to an MCP server
    pub async fn connect_sse(self) -> Result<McpServerConnection, Error> {
        let transport = SseClientTransport::start(self.url.clone()).await
            .map_err(Error::McpSseError)?;

        Ok(McpServerConnection::new(
            self.client_info
                .serve(transport)
                .await
                .map_err(Error::McpClientError)?,
            self.revalidate_tooling,
            self.revalidate_resources,
            self.url
            )
        )
    }
}

///
/// Represents a live connection to an MCP server.
pub struct McpServerConnection {
    running_service: Arc<RunningService<RoleClient, ClientInfo>>,
    pub(crate) revalidate_tooling: bool,
    pub(crate) revalidate_resources: bool,
    pub(crate) url: String,
}

impl McpServerConnection {
    fn new(
        running_service: RunningService<RoleClient, ClientInfo>,
        revalidate_tooling: bool,
        revalidate_resources: bool,
        url: String,
    ) -> Self {
        Self {
            running_service: Arc::new(running_service),
            revalidate_tooling,
            revalidate_resources,
            url
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