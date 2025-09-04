use crate::api::generated::types::{McpToolName, McpToolResult, TelemetryTarget};
use crate::error::Error;
use crate::mcp_server::McpServerConnection;
use crate::telemetry::{TelemetryIdentifier, TelemetryMode, TelemetryRequest};
use rig::completion::{AssistantContent, Completion, CompletionModel, Message};
use rig::message::UserContent;
use rig::tool::ToolDyn;
use rig::OneOrMany;
use std::collections::{HashSet};
use tracing::{info, warn};
use crate::completion_evaluated_prompt::CompletionEvaluatedPrompt;

pub struct Agent<M: CompletionModel>  {
    completion_agent: rig::agent::Agent<M>,
    mcp_connections: Vec<ValidatedMcpServerConnection>,
    revalidating_tooling: HashSet<String>,
    agent_name: String,
    agent_version: String,
    telemetry: TelemetryMode,
    telemetry_url: String,
    telemetry_session_id: String,
    telemetry_model_description: String,
    preamble: Option<CompletionEvaluatedPrompt>
}

struct ValidatedMcpServerConnection {
    connection: McpServerConnection,
    tools_validated: bool
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
    pub fn new(completion_agent: rig::agent::Agent<M>) -> Self {
        Self {
            completion_agent,
            mcp_connections: Vec::new(),
            revalidating_tooling: HashSet::new(),
            agent_name: env!("CARGO_PKG_NAME").to_string(),
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
            telemetry: TelemetryMode::None,
            telemetry_url: String::new(),
            telemetry_session_id: String::new(),
            telemetry_model_description: String::new(),
            preamble: None,
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
            tools_validated: false
        });
        self
    }

    ///
    /// Sets the preamble for this agent to a specific [`CompletionEvaluatedPrompt`] instance.  Note
    /// that if this is not set, the default string provided to the inner agent model will be used.
    ///
    /// The preamble will be evaluated in each call to [`run_completion`].
    pub fn preamble(mut self, preamble: CompletionEvaluatedPrompt) -> Self {
        self.preamble = Some(preamble);
        self
    }

    ///
    /// Sets the Telemetry mode for this agent.  The default value is [`TelemetryMode::None`]; in
    /// this mode, no telemetry is sent.
    ///
    /// If the value provided is anything but [`TelemetryMode::None`], the following environment
    /// variables are required (this function will panic if they are not provided):
    /// - CORAL_API_URL
    /// - CORAL_SESSION_ID
    pub fn telemetry(mut self, telemetry: TelemetryMode, model_description: impl Into<String>) -> Self {
        self.telemetry = telemetry;
        self.telemetry_url = std::env::var("CORAL_API_URL")
            .expect("CORAL_API_URL not set");
        self.telemetry_session_id = std::env::var("CORAL_SESSION_ID")
            .expect("CORAL_SESSION_ID not set");
        self.telemetry_model_description = model_description.into();

        self
    }

    ///
    /// This function is responsible for making sure every [`McpServerConnection`] provided to this
    /// agent has their tools validated as requested by the connection for a completion request.
    ///
    /// A single [`McpServerConnection`] may choose:
    /// - To have tooling skipped
    /// - To have tooling evaluated once
    /// - To have tooling evaluated before every completion
    async fn validate_mcp_tooling(&mut self) -> Result<(), Error> {
        // Remove any tooling that revalidates
        self.revalidating_tooling.retain(|mcp_tool_name| {
            self.completion_agent.static_tools.retain(|tool_name| tool_name != mcp_tool_name);
            self.completion_agent.tools.delete_tool(mcp_tool_name);
            false
        });

        let mut tools = Vec::new();
        for mcp in self.mcp_connections.iter_mut() {
            if (mcp.tools_validated && !mcp.connection.revalidate_tooling) ||
                mcp.connection.skip_tooling {
                continue;
            }

            let mcp_tools = mcp.connection.get_tools().await?;
            if !mcp.tools_validated {
                for tool in mcp_tools.iter() {
                    info!("adding tool \"{}\" from mcp server \"{}\"", tool.name(), mcp.connection.identifier);
                }
            }

            mcp.tools_validated = true;

            // If this MCP connection revalidates tooling, the list of tools that are revalidated
            // needs to be recorded so that it can be removed from the completion agent on the next
            // time this function is called
            if mcp.connection.revalidate_tooling {
                self.revalidating_tooling.extend(mcp_tools.iter().map(|tool| tool.name().clone()))
            }


            tools.extend(mcp_tools);
        }

        // Add new or revalidated tooling to the completion agent's tooling
        let agent_tools = std::mem::take(&mut self.completion_agent.tools);
        self.completion_agent.static_tools.extend(tools.iter().map(|tool| tool.name().clone()));
        self.completion_agent.tools = tools.into_iter().fold(agent_tools, |mut toolset, tool| {
            toolset.add_tool(tool);
            toolset
        });

        Ok(())
    }

    ///
    /// If there was a preamble provided to this agent, this function will evaluate it, and if the
    /// evaluation succeeds, the inner model's preamble field will be overwritten to this newly
    /// evaluated prompt.
    ///
    /// If there was no preamble provided to this agent, nothing will happen here.
    ///
    /// If the evaluation of the prompt fails (e.g., failure to locate a resource), this function will
    /// return an error.
    async fn validate_preamble(&mut self) -> Result<(), Error> {
        if let Some(prompt) = &self.preamble {
            match prompt.evaluate().await {
                Ok(prompt) => self.completion_agent.preamble = prompt,
                Err(e) => return Err(e)
            }
        }

        Ok(())
    }

    ///
    /// Sends telemetry data to the Coral server.  The coral server is identified by the
    /// CORAL_API_URL environment variable, which is automatically passed to agents orchestrated by
    /// Coral server
    async fn send_telemetry(
        &self,
        targets: Vec<TelemetryTarget>,
        messages: Vec<Message>
    ) {
        let target_count = targets.len();
        let id = TelemetryIdentifier {
            targets,
            session_id: self.telemetry_session_id.clone(),
        };

        let res = TelemetryRequest::new(
            id,
            self.telemetry_url.clone(),
            &self.completion_agent,
            self.telemetry_model_description.clone(),
            messages,
        )
            .telemetry_mode(self.telemetry.clone())
            .send()
            .await;

        if let Err(e) = res {
            warn!("Error sending telemetry: {e}")
        }
        else {
            info!("Telemetry attached to {target_count} messages");
        }
    }

    ///
    /// Gathers a list of places that telemetry could be attached to when given a tool call (name
    /// and output from tool).
    ///
    /// At the moment, telemetry is only attached to Coral messages.  So this function will return
    /// a TelemetryTarget from a Coral message if passed a call to [`McpTooling::CoralSendMessage`]
    fn find_telemetry_targets(name: &String, output: &String) -> Vec<TelemetryTarget> {
        let mut telemetry_targets = Vec::new();

        match serde_json::from_str::<McpToolName>(format!("\"{name}\"").as_str()) {
            Ok(McpToolName::CoralSendMessage) => {
                match serde_json::from_str::<McpToolResult>(output) {
                    Ok(McpToolResult::SendMessageSuccess { message }) => {
                        telemetry_targets.push(TelemetryTarget {
                            message_id: message.id,
                            thread_id: message.thread_id,
                        })
                    }
                    Err(e) => {
                        warn!("Identified CoralSendMessage tool call, but couldn't parse the output: {e}");
                    },
                    Ok(other) => {
                        warn!("Identified CoralSendMessage tool call, but got a non SendMessageSuccess return: {other:#?}");
                    }
                }
            }
            _ => {}
        }

        telemetry_targets
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
        mut messages: Vec<Message>
    ) -> Result<CompletionResult, Error> {
        self.validate_mcp_tooling().await?;
        self.validate_preamble().await?;

        // Take the last message from the stack as a prompt
        let prompt = messages
            .pop()
            .expect("cannot send completion with no messages");

        let resp = self.completion_agent
            .completion(prompt.clone(), messages.clone())
            .await.map_err(Error::CompletionError)?
            .send()
            .await.map_err(Error::CompletionError)?;

        messages.push(prompt);
        messages.push(Message::Assistant {
            id: None,
            content: resp.choice.clone(),
        });

        let mut tools_used = 0;
        let mut texts = Vec::new();
        let mut telemetry_targets = Vec::new();
        for choice in resp.choice {
            match choice {
                AssistantContent::ToolCall(tool_call) => {
                    tools_used = tools_used + 1;

                    let output = self.completion_agent
                        .tools
                        .call(
                            &tool_call.function.name,
                            tool_call.function.arguments.to_string(),
                        )
                        .await
                        .map_err(Error::ToolsetError)?;

                    telemetry_targets.extend(Self::find_telemetry_targets(&tool_call.function.name, &output));

                    messages.push(
                        if let Some(call_id) = tool_call.call_id {
                            UserContent::tool_result_with_call_id(
                                tool_call.id.clone(),
                                call_id,
                                OneOrMany::one(output.into()),
                            ).into()
                        }
                        else {
                            UserContent::tool_result(
                                tool_call.id.clone(),
                                OneOrMany::one(output.into()),
                            ).into()
                        }
                    )
                },
                AssistantContent::Text(text) => {
                    texts.push(text.text.clone());
                }
                _ => {}
            }
        }

        if !telemetry_targets.is_empty() && !matches!(self.telemetry, TelemetryMode::None) {
            self.send_telemetry(telemetry_targets, messages.clone()).await;
        }

        Ok(CompletionResult {
            messages,
            texts,
            tools_used,
        })
    }
}
