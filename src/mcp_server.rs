use crate::completion_evaluated_prompt::CompletionEvaluatedPrompt;
use crate::error::Error;
use reqwest::header::HeaderMap;
use rig::tool::rmcp::McpTool;
use rmcp::model::{
    ClientInfo, Implementation, ProtocolVersion, ReadResourceRequestParam, ResourceContents, Tool,
};
use rmcp::service::RunningService;
use rmcp::transport::sse_client::SseClientConfig;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::{
    ConfigureCommandExt, SseClientTransport, StreamableHttpClientTransport, TokioChildProcess,
};
use rmcp::{Peer, RoleClient, ServiceExt};
use std::ffi::OsStr;
use std::sync::Arc;
use tokio::process::Command;

pub struct McpConnectionBuilder {
    client_info: ClientInfo,
    revalidate_tooling: bool,
    skip_tooling: bool,
}

impl McpConnectionBuilder {
    pub fn builder() -> Self {
        Self {
            client_info: ClientInfo {
                protocol_version: Default::default(),
                capabilities: Default::default(),
                client_info: Implementation::from_build_env(),
            },
            revalidate_tooling: false,
            skip_tooling: false,
        }
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
    /// Helper function to build a connection to the Coral server.  This uses the Coral-provided
    /// CORAL_CONNECTION_URL environment variable and therefore only works when this is set (this
    /// is automatically set for agents launched by the Coral server).
    pub async fn build_coral_sse() -> Result<McpServerConnection, Error> {
        Self::builder()
            .revalidate_tooling(false)
            .build_sse(std::env::var("CORAL_CONNECTION_URL").expect("CORAL_CONNECTION_URL not set"))
            .await
    }

    ///
    /// Builds a basic MCP server connection using an SSE transport to the specified [url]
    pub async fn build_sse(self, url: impl Into<String>) -> Result<McpServerConnection, Error> {
        self.build_sse_with_headers(url, HeaderMap::new()).await
    }

    ///
    /// Builds a new MCP connection builder using an SSE transport, allowing headers to be passed
    /// (usually used for authorization)
    pub async fn build_sse_with_headers(
        self,
        url: impl Into<String>,
        headers: impl Into<HeaderMap>,
    ) -> Result<McpServerConnection, Error> {
        let url = url.into();
        let transport = self
            .client_info
            .serve(
                SseClientTransport::start_with_client(
                    reqwest::ClientBuilder::new()
                        .default_headers(headers.into())
                        .build()
                        .map_err(|e| Error::McpSseError(e.into()))?,
                    SseClientConfig {
                        sse_endpoint: url.clone().into(),
                        ..Default::default()
                    },
                )
                .await
                .map_err(Error::McpSseError)?,
            )
            .await
            .map_err(Error::McpClientError)?;

        Ok(McpServerConnection::new(
            transport,
            self.revalidate_tooling,
            self.skip_tooling,
            url,
        ))
    }

    ///
    /// Builds an MCP connection from a child process' stdio stream
    pub async fn build_stdio<I, S>(
        self,
        executable: impl Into<String>,
        arguments: I,
        identifier: impl Into<String>,
    ) -> Result<McpServerConnection, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let transport = self
            .client_info
            .clone()
            .serve(
                TokioChildProcess::new(Command::new(executable.into()).configure(|c| {
                    c.args::<I, S>(arguments);
                }))
                .map_err(Error::McpStdioError)?,
            )
            .await
            .map_err(Error::McpClientError)?;

        Ok(self.build(transport, identifier))
    }

    ///
    /// Builds an MCP connection from a streamable HTTP URI.  Helper function for [Self::build_streamable_http_with_headers]
    pub async fn build_streamable_http(
        self,
        uri: impl Into<String>,
    ) -> Result<McpServerConnection, Error> {
        self.build_streamable_http_with_headers(uri, HeaderMap::new())
            .await
    }

    ///
    /// Builds an MCP connection from a streamable HTTP URI.  This function allows headers to be
    /// passed through for authorization.
    pub async fn build_streamable_http_with_headers(
        self,
        uri: impl Into<String>,
        headers: impl Into<HeaderMap>,
    ) -> Result<McpServerConnection, Error> {
        let uri = uri.into();
        let transport = self
            .client_info
            .serve(StreamableHttpClientTransport::with_client(
                reqwest::ClientBuilder::new()
                    .default_headers(headers.into())
                    .build()
                    .map_err(|e| Error::McpSseError(e.into()))?,
                StreamableHttpClientTransportConfig {
                    uri: uri.clone().into(),
                    ..Default::default()
                },
            ))
            .await
            .map_err(Error::McpClientError)?;

        Ok(McpServerConnection::new(
            transport,
            self.revalidate_tooling,
            self.skip_tooling,
            uri,
        ))
    }

    ///
    /// Builds a [McpServerConnection] from a given [transport] and [identifier]
    pub fn build(
        self,
        transport: RunningService<RoleClient, ClientInfo>,
        identifier: impl Into<String>,
    ) -> McpServerConnection {
        McpServerConnection::new(
            transport,
            self.revalidate_tooling,
            self.skip_tooling,
            identifier.into(),
        )
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
    pub(crate) async fn get_tools(&self) -> Result<Vec<(Tool, Peer<RoleClient>)>, Error> {
        Ok(self
            .running_service
            .list_all_tools()
            .await
            .map_err(Error::McpServiceError)?
            .into_iter()
            .map(|x| (x, self.running_service.peer().clone()))
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
