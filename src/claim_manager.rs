use crate::api::generated::Client;
use crate::api::generated::types::{
    AgentClaimAmount as ClaimAmount, AgentClaimAmount, AgentPaymentClaimRequest, McpToolName,
};
use crate::error::Error;
use rig::completion::Usage;
use std::collections::HashMap;
use std::ops::{Div, Mul};
use tracing::{info, warn};

const MICRO_CORAL_TO_CORAL: f64 = 1_000_000.0;

///
/// When a Coral agent is run in remote mode, it must make "claims".  The agent claims to have
/// performed a certain amount of work for a certain amount of currency.  Claiming is done through
/// the Coral server API.  When a claim is made, the server will return the remaining budget after
/// applying the claim.
///
/// If the remaining budget returned is negative, the server has run out of budget for this agent,
/// and it should (likely) exit.  Note that the server will not allow agents to go over budget, so
/// when a negative budget is returned, this agent was not able to claim the requested amount.  It
/// is the responsibility of the agent to handle this.
///
/// The claim manager can be attached to a [crate::agent::Agent] to simplify the claiming process.
///
/// When a [crate::agent::Agent] has a [ClaimManager] attached, it will use the rules and prices set
/// in the [ClaimManager] automatically.
///
/// Note that the environment variable `CORAL_SEND_CLAIMS` must be set to `1` for a claim manager to
/// send claims.  The Coral server will set this variable during orchestration in remote sessions.
pub struct ClaimManager {
    ///
    /// The base cost of an input token.  Note that token usage is reported OPTIONALLY by the AI the model
    /// provider.  Check that the model provider you are using provides this information if you
    /// intend to use this metric.
    ///
    /// Tokens will be claimed after every completion (tool or prompt iteration)
    input_token_cost: ClaimAmount,

    ///
    /// The base cost of an output token.  See [`input_token_cost`] for more information.
    output_token_cost: ClaimAmount,

    ///
    /// The minimum amount of budget required to continue doing completions.  This can be used to
    /// protect against doing free work.  If you think that one completion/cycle/etc will cost at
    /// minimum 1000, you can set this to 1000 so that the agent will exit when it has a remaining
    /// budget of 500
    ///
    /// The min budget will be evaluated before: tool executions, prompt iterations and tool iterations
    min_budget: ClaimAmount,

    ///
    /// A base cost added for any invocation of a tool.  Note that this will include Coral tooling,
    /// which is required to be used in a Coral agent.
    ///
    /// Tool calls will be claimed after execution
    base_tool_call_cost: ClaimAmount,

    ///
    /// This map can be used to add a custom cost associated with a tool.  The key is the name of
    /// the tool and the value is the cost.  The cost will be added to the base_tool_call_cost
    ///
    /// Tool calls will be claimed after execution
    custom_tool_cost: HashMap<String, ClaimAmount>,

    ///
    /// The cost to perform one iteration.  A single iteration may contain one or more tool
    /// iterations.
    ///
    /// An iteration cost will be claimed after a prompt iteration
    base_iteration_cost: ClaimAmount,

    ///
    /// A tool iteration is an iteration that occurs because of one or more tool calls.  A tool
    /// iteration is always exactly one model completion request.
    ///
    /// A tool iteration cost will be claimed after tool iteration
    base_tool_iteration_cost: ClaimAmount,

    ///
    /// Whether the agent should exit when the budget has been exhausted.  This should almost always
    /// be true, if this value is not true, the agent will perform work for free when the budget has
    /// been exhausted.
    ///
    /// The budget will be evaluated after a claim is made.
    exit_on_budget_exhausted: bool,

    ///
    /// API url from CORAL_API_URL
    api_url: String,

    ///
    /// Session ID for this agent that must be used in API claims
    remote_session_id: String,
}

impl ClaimManager {
    ///
    /// Creates a new claim manager with every claim value set to zero.  This function will panic if
    /// `CORAL_API_URL` or `CORAL_SESSION_ID` are not set environment variables.
    ///
    /// Claims will not be sent if `CORAL_SEND_CLAIMS` is not equal to `1`
    pub fn new() -> Self {
        Self {
            input_token_cost: ClaimAmount::MicroCoral(0),
            output_token_cost: ClaimAmount::MicroCoral(0),
            min_budget: ClaimAmount::MicroCoral(0),
            base_tool_call_cost: ClaimAmount::MicroCoral(0),
            custom_tool_cost: HashMap::new(),
            base_iteration_cost: ClaimAmount::MicroCoral(0),
            base_tool_iteration_cost: ClaimAmount::MicroCoral(0),
            exit_on_budget_exhausted: true,
            api_url: std::env::var("CORAL_API_URL").expect("CORAL_API_URL not set"),
            remote_session_id: std::env::var("CORAL_SESSION_ID").expect("CORAL_SESSION_ID not set"),
        }
    }

    ///
    /// Sets the cost per singular input token
    pub fn input_token_cost(mut self, input_token_cost: ClaimAmount) -> Self {
        self.input_token_cost = input_token_cost;
        self
    }

    ///
    /// Sets the cost per million input tokens
    pub fn mil_input_token_cost(mut self, input_token_cost: ClaimAmount) -> Self {
        self.input_token_cost = input_token_cost.div(1_000_000);
        self
    }

    ///
    /// Sets the cost per singular output token
    pub fn output_token_cost(mut self, output_token_cost: ClaimAmount) -> Self {
        self.output_token_cost = output_token_cost;
        self
    }

    ///
    /// Sets the cost per million output tokens
    pub fn mil_output_token_cost(mut self, output_token_cost: ClaimAmount) -> Self {
        self.output_token_cost = output_token_cost.div(1_000_000);
        self
    }

    ///
    /// Sets the minimum budget
    pub fn min_budget(mut self, min_budget: ClaimAmount) -> Self {
        self.min_budget = min_budget;
        self
    }

    ///
    /// Sets the base tool call cost
    pub fn base_tool_call_cost(mut self, base_tool_call_cost: ClaimAmount) -> Self {
        self.base_tool_call_cost = base_tool_call_cost;
        self
    }

    ///
    /// Sets the base iteration cost
    pub fn base_iteration_cost(mut self, base_iteration_cost: ClaimAmount) -> Self {
        self.base_iteration_cost = base_iteration_cost;
        self
    }

    ///
    /// Sets the base tool iteration cost
    pub fn base_tool_iteration_cost(mut self, base_tool_iteration_cost: ClaimAmount) -> Self {
        self.base_tool_iteration_cost = base_tool_iteration_cost;
        self
    }
    ///
    /// Sets whether to exit when the budget has been exhausted
    pub fn exit_on_budget_exhausted(mut self, exit_on_budget_exhausted: bool) -> Self {
        self.exit_on_budget_exhausted = exit_on_budget_exhausted;
        self
    }

    ///
    /// Adds a new custom tool cost by name
    pub fn custom_tool_cost(mut self, tool_name: impl Into<String>, cost: ClaimAmount) -> Self {
        self.custom_tool_cost.insert(tool_name.into(), cost);
        self
    }

    ///
    /// Adds a new tool cost for a Coral tool
    pub fn coral_custom_tool_cost(mut self, tool_name: McpToolName, cost: ClaimAmount) -> Self {
        self.custom_tool_cost.insert(tool_name.to_string(), cost);
        self
    }

    ///
    /// Claim for tokens used
    pub(crate) async fn claim_tokens(&self, usage: &Usage) -> Result<(), Error> {
        if self.input_token_cost.is_zero() && self.output_token_cost.is_zero() {
            info!("not claiming tokens because input_token_cost and output_token_cost are zero");
            return Ok(());
        }

        if usage.input_tokens + usage.output_tokens != usage.total_tokens {
            // If the input and output tokens do not combine to the total tokens, it means the
            // provider did provide token usage but only gave it to us as total tokens.  In this
            // case the output token price will be used.  If the claim manager has a cost specified
            // for input tokens, a warning should be generated
            if !self.input_token_cost.is_zero() {
                warn!(
                    "provider only reported total token usage, input_token_cost will be ignored!  token cost will be claimed used output_token_cost"
                )
            }

            info!(
                "claiming {} for {} tokens",
                self.output_token_cost, usage.total_tokens
            );
            return self
                .claim(self.output_token_cost.clone().mul(usage.total_tokens))
                .await;
        } else if usage.total_tokens == 0 {
            warn!("provider reported zero tokens!");
        } else {
            info!(
                "claiming {} for {} input tokens",
                self.input_token_cost, usage.input_tokens
            );
            self.claim(self.input_token_cost.clone().mul(usage.input_tokens))
                .await?;

            info!(
                "claiming {} for {} output tokens",
                self.output_token_cost, usage.output_tokens
            );
            self.claim(self.output_token_cost.clone().mul(usage.output_tokens))
                .await?;
        }

        Ok(())
    }

    ///
    /// Claim for one prompt iteration
    pub(crate) async fn claim_iteration(&self) -> Result<(), Error> {
        if !self.base_iteration_cost.is_zero() {
            info!(
                "claiming {} for one prompt iteration",
                self.base_iteration_cost
            );
            self.claim(self.base_iteration_cost.clone()).await
        } else {
            info!("not claiming prompt iteration because base_iteration_cost is zero");
            Ok(())
        }
    }

    ///
    /// Claim for one tool iteration
    pub(crate) async fn claim_tool_iteration(&self) -> Result<(), Error> {
        if !self.base_tool_iteration_cost.is_zero() {
            info!(
                "claiming {} for one tool iteration",
                self.base_tool_iteration_cost
            );
            self.claim(self.base_tool_iteration_cost.clone()).await
        } else {
            info!("not claiming tool iteration because base_tool_iteration_cost is zero");
            Ok(())
        }
    }

    ///
    /// Claim for one tool call
    pub(crate) async fn claim_tool_call(&self, tool_name: impl Into<String>) -> Result<(), Error> {
        let name = tool_name.into();
        if !self.base_tool_call_cost.is_zero() {
            self.claim(self.base_tool_call_cost.clone()).await?;
            info!(
                "claiming {} as a base cost for tool '{name}'",
                self.base_tool_call_cost
            );
        }

        if let Some(cost) = self.custom_tool_cost.get(name.as_str()) {
            info!("claiming {cost} as an additional cost for tool '{name}'");

            self.claim(cost.clone()).await?;
            self.claim(cost.clone()).await?;
        }

        Ok(())
    }

    ///
    /// Send a claim to the Coral server
    async fn claim(&self, amount: ClaimAmount) -> Result<(), Error> {
        // CORAL_SEND_CLAIMS must be '1' to send claims to the server, if this is not set, it
        // indicates the agent is running in local mode
        if std::env::var("CORAL_SEND_CLAIMS") != Ok("1".to_string()) {
            return Ok(());
        }

        if amount.is_zero() {
            // Don't spam the server with zero claims
            return Ok(());
        }

        let budget = Client::new(self.api_url.as_str())
            .claim_payment(
                self.remote_session_id.as_str(),
                &AgentPaymentClaimRequest { amount },
            )
            .await
            .map_err(Error::ApiError)?
            .into_inner();

        if self.exit_on_budget_exhausted {
            // If the ClaimManager's budget was expressed in USD, we need to use the server-provided
            // conversion rate... Coral server has some warnings about the accuracy of this field,
            // which shouldn't be ignored.  At this point, we have nothing better to use, and it is
            // the only way to provide a reasonable result when USD is given.
            let min_micro = match self.min_budget {
                AgentClaimAmount::Coral(coral) => (coral * MICRO_CORAL_TO_CORAL) as i64,
                AgentClaimAmount::MicroCoral(micro) => micro,
                AgentClaimAmount::Usd(usd) => {
                    ((usd / budget.coral_usd_price) * MICRO_CORAL_TO_CORAL) as i64
                }
            };

            if budget.remaining_budget <= min_micro {
                return Err(Error::BudgetExhausted);
            }
        }

        Ok(())
    }
}

