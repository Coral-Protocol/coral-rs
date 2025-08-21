use std::pin::Pin;
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
    iteration_prompt_provider: Box<dyn LoopPromptProvider>,
    iteration_tool_quota: Option<u32>
}

pub trait LoopPromptProvider: Send + Sync {
    /// Called to generate the prompt for this loop
    fn loop_prompt(&mut self) -> Pin<Box<dyn Future<Output = String> + Send + '_>>;

    /// Called after the prompt is generated, if this returns true, the loop will exit
    fn finished(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>>;
}


pub struct IterationPromptProvider {
    prompt: String,
    iteration_count: u32,
    iteration_max: u32,
    delay: Option<Duration>
}

impl LoopPromptProvider for IterationPromptProvider {
    fn loop_prompt(&mut self) -> Pin<Box<dyn Future<Output = String> + Send + '_>> {
        Box::pin(async move {
            // No delay for the first iteration
            if self.iteration_count > 0 {
                if let Some(delay) = self.delay {
                    tokio::time::sleep(delay).await;
                }
            }

            self.iteration_count += 1;
            self.prompt.clone()
        })
    }

    fn finished(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        Box::pin(async move {
            self.iteration_count >= self.iteration_max
        })
    }
}

impl IterationPromptProvider {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            iteration_count: 0,
            iteration_max: DEFAULT_ITERATIONS,
            delay: Some(DEFAULT_DELAY)
        }
    }

    ///
    /// The number of iterations to loop for
    /// Default is [`DEFAULT_ITERATIONS`]
    pub fn iterations(mut self, iterations: u32) -> Self {
        self.iteration_max = iterations;
        self
    }

    ///
    /// Time to wait between the end of one iteration and the beginning of the next.
    /// The time it took for the last iteration to complete has no bearing on this parameter.
    /// Default is [`DEFAULT_DELAY`]
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    ///
    /// Call this to disable delays between iterations
    pub fn no_delay(mut self) -> Self {
        self.delay = None;
        self
    }
}

impl<M: CompletionModel>  AgentLoop<M> {
    ///
    /// Creates a new Coral agent loop
    pub fn new(agent: Agent<M>, iteration_prompt_provider: impl LoopPromptProvider + 'static) -> Self {
        Self {
            agent,
            iteration_prompt_provider: Box::new(iteration_prompt_provider),
            iteration_tool_quota: DEFAULT_ITERATION_TOOL_QUOTA
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
        loop {
            let prompt = self.iteration_prompt_provider.loop_prompt().await;
            iterations += 1;

            // An iteration should always start with the loop prompt
            messages.push(prompt.clone().into());

            let mut depth = 0;
            loop {
                depth = depth + 1;
                info!("Tool iteration {}/{} [prompt iteration {iterations}]",
                    depth + 1,
                    self.iteration_tool_quota.map_or("unlimited".to_string(), |x| x.to_string()),
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

            if self.iteration_prompt_provider.finished().await {
                break;
            }
        }

        Ok(())
    }
}