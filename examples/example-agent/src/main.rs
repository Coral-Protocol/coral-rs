use std::time::Duration;
use coral_rs::rig::client::CompletionClient;
use coral_rs::rig::client::ProviderClient;
use coral_rs::agent::Agent;
use coral_rs::agent_loop::{AgentLoop};
use coral_rs::mcp_server::McpConnectionBuilder;
use coral_rs::repeating_prompt_stream::repeating_prompt_stream;
use coral_rs::rig::providers::openai::GPT_4_1_MINI;
use coral_rs::telemetry::TelemetryMode;
use coral_rs::rig::providers::openai;


#[tokio::main]
async fn main() {
    let model = GPT_4_1_MINI;

    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    let coral_mcp = McpConnectionBuilder::from_coral_env()
        .connect()
        .await.expect("Failed to connect to the Coral server");

    let completion_agent = openai::Client::from_env()
        .agent(model)
        .preamble("You are a unit test.")
        .temperature(0.97)
        .max_tokens(512)
        .build();

    let prompt = coral_mcp.prompt_with_resources("Repeat to me the Coral instruction set");

    let agent = Agent::new(completion_agent)
        .telemetry(TelemetryMode::OpenAI, model)
        .mcp_server(coral_mcp.clone());
    
    let prompt_stream = repeating_prompt_stream(
        prompt,
        Some(Duration::from_secs(1)),
        1
    );

    AgentLoop::new(agent, prompt_stream)
        .execute()
        .await
        .expect("Agent loop failed");
}
