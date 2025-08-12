use crate::agent::Agent;
use crate::error::Error;
use rig::completion::{CompletionModel};
use std::time::Duration;
use tracing::{info, warn};

const DEFAULT_ITERATION_TOOL_QUOTA: Option<u32> = Some(64);
const DEFAULT_ITERATIONS: u32 = 20;
const DEFAULT_DELAY: Duration = Duration::from_secs(10);

pub struct AgentLoop<M: CompletionModel>  {
    agent: Agent<M>,
    prompt: String,
    iterations: u32,
    delay: Duration,
    iteration_tool_quota: Option<u32>
}

impl<M: CompletionModel>  AgentLoop<M> {
    ///
    /// Creates a new Coral agent loop
    pub fn new(agent: Agent<M>, prompt: impl Into<String>) -> Self {
        Self {
            agent,
            prompt: prompt.into(),
            iterations: DEFAULT_ITERATIONS,
            delay: DEFAULT_DELAY,
            iteration_tool_quota: DEFAULT_ITERATION_TOOL_QUOTA
        }
    }

    ///
    /// The number of iterations to loop for
    /// Default is [`DEFAULT_ITERATIONS`]
    pub fn iterations(mut self, iterations: u32) -> Self {
        self.iterations = iterations;
        self
    }

    ///
    /// Time to wait between the end of one iteration and the beginning of the next.
    /// The time it took for the last iteration to complete has no bearing on this parameter.
    /// Default is [`DEFAULT_DELAY`]
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
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
        info!("Prompt: {}", self.prompt);

        let mut messages = Vec::new();
        for i in 0..self.iterations {
            tokio::time::sleep(self.delay).await;

            // An iteration should always start with the loop prompt
            messages.push(self.prompt.clone().into());

            let mut depth = 0;
            loop {
                depth = depth + 1;
                info!("Running iteration {}/{} [tool quota {}/{}]",
                    i + 1,
                    self.iterations,
                    depth + 1,
                    self.iteration_tool_quota.map_or("unlimited".to_string(), |x| x.to_string()),
                );

                let res = self.agent.run_completion(messages).await?;
                if !res.texts.is_empty() {
                    info!("\"{}\"", res.texts.join(""));
                }

                messages = res.messages;
                if res.tools_used == 0 {
                    info!("Iteration {}/{} finished - no tools used", i + 1, self.iterations);
                    break;
                }

                if Some(depth) == self.iteration_tool_quota {
                    warn!("Iteration {}/{} finished - tool quota reached", i + 1, self.iterations);
                    break;
                }
            }
        }

        Ok(())
    }
}