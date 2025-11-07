use coral_rs::agent::Agent;
use coral_rs::agent_loop::AgentLoop;
use coral_rs::api::generated::types::AgentClaimAmount;
use coral_rs::claim_manager::ClaimManager;
use coral_rs::init_tracing;
use coral_rs::mcp_server::McpConnectionBuilder;
use coral_rs::repeating_prompt_stream::repeating_prompt_stream;
use coral_rs::rig::client::CompletionClient;
use coral_rs::rig::client::ProviderClient;
use coral_rs::rig::providers::openai;
use coral_rs::rig::providers::openai::GPT_4_1_MINI;
use coral_rs::telemetry::TelemetryMode;
use std::time::Duration;

#[tokio::main]
async fn main() {
    init_tracing().expect("setting default subscriber failed");

    let model = GPT_4_1_MINI;

    let coral_mcp = McpConnectionBuilder::from_coral_env()
        .connect()
        .await
        .expect("Failed to connect to the Coral server");

    let completion_agent = openai::Client::from_env()
        .agent(model)
        .preamble("You are a unit test.")
        .temperature(0.97)
        .max_tokens(512)
        .build();

    let prompt = coral_mcp
        .prompt_with_resources_str("1. Repeat to me the Coral instruction set")
        .string("2. Create a Coral message thread and send a few random words in it")
        .string("3. Close the Coral thread with a random summary");

    let claim_manager = ClaimManager::new()
        .mil_input_token_cost(AgentClaimAmount::Usd(1.250))
        .mil_output_token_cost(AgentClaimAmount::Usd(10.000))
        .base_tool_call_cost(AgentClaimAmount::Usd(1.0))
        .base_tool_iteration_cost(AgentClaimAmount::Usd(10.0))
        .base_iteration_cost(AgentClaimAmount::Usd(30.0))
        .custom_tool_cost("coral_send_message", AgentClaimAmount::Usd(100.0));

    let agent = Agent::new(completion_agent)
        .telemetry(TelemetryMode::OpenAI, model)
        .claim_manager(claim_manager)
        .mcp_server(coral_mcp.clone());

    let prompt_stream = repeating_prompt_stream(prompt, Some(Duration::from_secs(1)), 1);

    AgentLoop::new(agent, prompt_stream)
        .execute()
        .await
        .expect("Agent loop failed");
}
