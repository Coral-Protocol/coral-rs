use crate::agent::Agent;
use crate::completion_evaluated_prompt::CompletionEvaluatedPrompt;
use crate::error::Error;
use futures::{Stream, StreamExt};
use rig::completion::CompletionModel;
use std::pin::Pin;
use tracing::{info, warn};

pub const DEFAULT_ITERATION_TOOL_QUOTA: Option<u32> = Some(64);

pub struct AgentLoop<M: CompletionModel> {
    agent: Agent<M>,
    prompt_stream: Pin<Box<dyn Stream<Item = CompletionEvaluatedPrompt>>>,
    iteration_tool_quota: Option<u32>,
}

impl<M: CompletionModel> AgentLoop<M> {
    ///
    /// Creates a new Coral agent loop
    pub fn new(
        agent: Agent<M>,
        prompt_stream: impl Stream<Item = CompletionEvaluatedPrompt> + 'static,
    ) -> Self {
        Self {
            agent,
            prompt_stream: Box::pin(prompt_stream),
            iteration_tool_quota: DEFAULT_ITERATION_TOOL_QUOTA,
        }
    }

    ///
    /// The maximum number of tools that can be used during one iteration.  If an iteration reaches
    /// this limit, it will move on to the next iteration.  This number should be large enough to
    /// allow the model to use as many tools as it needs to complete a task, but should also be
    /// small enough to catch any bugs that result in infinite tool usage.
    ///
    /// If None is provided, there will be no limit to tool usage.  If there is an infinite tool
    /// usage bug when None is set, tokens will be burned!
    /// Default is [`DEFAULT_ITERATION_TOOL_QUOTA`]
    pub fn iteration_tool_quota(mut self, iteration_tool_quota: Option<u32>) -> Self {
        self.iteration_tool_quota = iteration_tool_quota;
        self
    }

    ///
    /// Executes the loop, consuming self
    pub async fn execute(mut self) -> Result<(), Error> {
        info!("Starting Coral agent loop");

        let mut messages = Vec::new();
        let mut iterations = 0;
        while let Some(prompt) = self.prompt_stream.next().await {
            iterations += 1;

            // An iteration should always start with the loop prompt
            messages.push(prompt.evaluate().await?.into());

            let mut depth = 0;
            loop {
                depth = depth + 1;
                info!(
                    "Tool iteration {}/{} [prompt iteration {iterations}]",
                    depth + 1,
                    self.iteration_tool_quota
                        .map_or("unlimited".to_string(), |x| x.to_string()),
                );

                let res = self.agent.run_completion(messages).await?;
                if !res.texts.is_empty() {
                    info!("\"{}\"", res.texts.join(""));
                }

                messages = res.messages;
                if res.tools_used == 0 {
                    info!("Prompt iteration [{iterations}] finished - no tools used");
                    break;
                }

                if Some(depth) == self.iteration_tool_quota {
                    warn!("Prompt iteration [{iterations}] finished - tool quota reached");
                    break;
                }
            }
        }

        Ok(())
    }
}

