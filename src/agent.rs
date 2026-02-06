use crate::claim_manager::ClaimManager;
use crate::completion_evaluated_prompt::CompletionEvaluatedPrompt;
use crate::error::Error;
use crate::mcp_server::McpServerConnection;
use rig::OneOrMany;
use rig::completion::{AssistantContent, Completion, CompletionModel, Message};
use rig::message::UserContent;
use rig::tool::server::{ToolServer, ToolServerHandle};
use std::collections::HashSet;
use tracing::warn;

pub struct Agent<M: CompletionModel> {
    completion_agent: rig::agent::Agent<M>,
    mcp_connections: Vec<ValidatedMcpServerConnection>,
    revalidating_tooling: HashSet<String>,
    agent_name: String,
    agent_version: String,
    system_text: CompletionEvaluatedPrompt,
    claim_manager: Option<ClaimManager>,
}

struct ValidatedMcpServerConnection {
    connection: McpServerConnection,
    tools_validated: bool,
}

pub struct CompletionResult {
    /// Entire message history
    pub messages: Vec<Message>,

    /// The texts returned by the completion agent.  It is possible for this to be empty
    pub texts: Vec<String>,

    /// Quantity of tools used. If this is non-zero, it is likely texts are empty.
    pub tools_used: u32,
}

impl<M: CompletionModel> Agent<M> {
    ///
    /// Creates a new Coral agent using an underlying completion agent.
    pub fn new(
        completion_agent: rig::agent::Agent<M>,
        system_text: CompletionEvaluatedPrompt,
    ) -> Self {
        Self {
            completion_agent,
            mcp_connections: Vec::new(),
            revalidating_tooling: HashSet::new(),
            agent_name: env!("CARGO_PKG_NAME").to_string(),
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
            system_text,
            claim_manager: None,
        }
    }

    ///
    /// Agent name.  Used to identify this agent in MCP servers.
    pub fn agent_name(mut self, name: impl Into<String>) -> Self {
        self.agent_name = name.into();
        self
    }

    ///
    /// Agent version. Used to identify this agent in MCP servers.
    pub fn agent_version(mut self, version: impl Into<String>) -> Self {
        self.agent_version = version.into();
        self
    }

    ///
    /// Adds an MCP server to the Agent.  MCP server tools will be evaluated before requests are
    /// made
    pub fn mcp_server(mut self, connection: McpServerConnection) -> Self {
        self.mcp_connections.push(ValidatedMcpServerConnection {
            connection,
            tools_validated: false,
        });
        self
    }

    ///
    /// Sets the claim manager to use it with this Agent.  If no claim manager is set, no claims
    /// will be made for this agent.  If you plan to export an agent, you must claim from the agent.
    pub fn claim_manager(mut self, claim_manager: ClaimManager) -> Self {
        self.claim_manager = Some(claim_manager);
        self
    }

    // ///
    // /// This function is responsible for making sure every [`McpServerConnection`] provided to this
    // /// agent has their tools validated as requested by the connection for a completion request.
    // ///
    // /// A single [`McpServerConnection`] may choose:
    // /// - To have tooling skipped
    // /// - To have tooling evaluated once
    // /// - To have tooling evaluated before every completion
    // async fn validate_mcp_tooling(&mut self) -> Result<ToolServerHandle, Error> {
    //     let mut tool_server = ToolServer::new();
    //
    //     for mcp in self.mcp_connections.iter_mut() {
    //         let tools = mcp.connection.get_tools().await?;
    //         for (tool, peer) in tools.iter() {
    //             tool_server = tool_server.rmcp_tool(tool.clone(), peer.to_owned())
    //         }
    //     }
    //
    //     // // Remove any tooling that revalidates
    //     // self.revalidating_tooling.retain(|mcp_tool_name| {
    //     //     // self.completion_agent
    //     //     //     .tools
    //     //     //     .retain(|tool_name| tool_name != mcp_tool_name);
    //     //     // self.completion_agent.tools.delete_tool(mcp_tool_name);
    //     //     false
    //     // });
    //     //
    //     // let mut tools = Vec::new();
    //     // for mcp in self.mcp_connections.iter_mut() {
    //     //     if (mcp.tools_validated && !mcp.connection.revalidate_tooling)
    //     //         || mcp.connection.skip_tooling
    //     //     {
    //     //         continue;
    //     //     }
    //     //
    //     //     let mcp_tools = mcp.connection.get_tools().await?;
    //     //     if !mcp.tools_validated {
    //     //         for tool in mcp_tools.iter() {
    //     //             info!(
    //     //                 "adding tool \"{}\" from mcp server \"{}\"",
    //     //                 tool.name(),
    //     //                 mcp.connection.identifier
    //     //             );
    //     //         }
    //     //     }
    //     //
    //     //     mcp.tools_validated = true;
    //     //
    //     //     // If this MCP connection revalidates tooling, the list of tools that are revalidated
    //     //     // needs to be recorded so that it can be removed from the completion agent on the next
    //     //     // time this function is called
    //     //     if mcp.connection.revalidate_tooling {
    //     //         self.revalidating_tooling
    //     //             .extend(mcp_tools.iter().map(|tool| tool.name().clone()))
    //     //     }
    //     //
    //     //     tools.extend(mcp_tools);
    //     // }
    //     //
    //     // // Add new or revalidated tooling to the completion agent's tooling
    //     // let agent_tools = std::mem::take(&mut self.completion_agent.tools);
    //     // self.completion_agent
    //     // self.completion_agent
    //     //     .static_tools
    //     //     .extend(tools.iter().map(|tool| tool.name().clone()));
    //     // self.completion_agent.tools = tools.into_iter().fold(agent_tools, |mut toolset, tool| {
    //     //     toolset.add_tool(tool);
    //     //     toolset
    //     // });
    //     //
    //     // Ok(())
    //
    //     Ok(ToolServer::new().run())
    // }

    async fn build_tool_server(&mut self) -> Result<ToolServerHandle, Error> {
        let mut tool_server = ToolServer::new();

        for mcp in self.mcp_connections.iter_mut() {
            let tools = mcp.connection.get_tools().await?;
            for (tool, peer) in tools.iter() {
                tool_server = tool_server.rmcp_tool(tool.clone(), peer.to_owned())
            }
        }

        Ok(tool_server.run())
    }

    /// Performs a completion request
    ///
    /// This function, in order:
    /// 1. Validates all tooling and documents on any connected MCP server (that require validation)
    /// 2. Performs one completion request to the underlying completion agent
    /// 3. Runs any tool calls that came back from the request
    /// 4. Appends all messages in the response and any tool call results to the message history
    ///
    /// If telemetry is enabled, the last step of this function will be to post telemetry data
    ///  to the Coral server.
    ///
    /// # Arguments
    /// * `messages` - The full message history for this completion request.  It is assumed that
    /// this contains the necessary prompts for the completion.  This function will panic if given
    /// an empty message history.
    ///
    pub async fn run_completion(
        &mut self,
        mut messages: Vec<Message>,
    ) -> Result<CompletionResult, Error> {
        self.completion_agent.tool_server_handle = self.build_tool_server().await?;

        let resp = self
            .completion_agent
            .completion(self.system_text.evaluate().await?, messages.clone())
            .await
            .map_err(Error::CompletionError)?
            .send()
            .await
            .map_err(Error::CompletionError)?;

        if let Some(claim_manager) = &self.claim_manager {
            claim_manager.claim_tokens(&resp.usage).await?;
        }

        let mut tools_used = 0;
        let mut texts = Vec::new();
        for choice in resp.choice {
            match &choice {
                AssistantContent::ToolCall(tool_call) => {
                    tools_used = tools_used + 1;

                    let output = self
                        .completion_agent
                        .tool_server_handle
                        .call_tool(
                            &tool_call.function.name,
                            &*tool_call.function.arguments.to_string(),
                        )
                        .await
                        .unwrap_or_else(|e| {
                            warn!("error calling tool {}: {e}", tool_call.function.name);
                            e.to_string()
                        });

                    if let Some(claim_manager) = &self.claim_manager {
                        claim_manager
                            .claim_tool_call(tool_call.function.name.clone())
                            .await?;
                    }

                    messages.push(Message::Assistant {
                        id: None,
                        content: OneOrMany::one(choice.clone()),
                    });
                    messages.push(if let Some(call_id) = tool_call.call_id.clone() {
                        UserContent::tool_result_with_call_id(
                            tool_call.id.clone(),
                            call_id,
                            OneOrMany::one(output.into()),
                        )
                        .into()
                    } else {
                        UserContent::tool_result(
                            tool_call.id.clone(),
                            OneOrMany::one(output.into()),
                        )
                        .into()
                    })
                }
                AssistantContent::Text(text) => {
                    texts.push(text.text.clone());
                }
                _ => {}
            }
        }

        if let Some(claim_manager) = &self.claim_manager {
            if tools_used == 0 {
                claim_manager.claim_iteration().await?;
            } else {
                claim_manager.claim_tool_iteration().await?;
            }
        }

        Ok(CompletionResult {
            messages,
            texts,
            tools_used,
        })
    }
}
