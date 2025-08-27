#[cfg(test)]
mod agent_tests {
    use coral_rs::agent::Agent;
    use coral_rs::agent_loop::AgentLoop;
    use coral_rs::mcp_server::McpConnectionBuilder;
    use coral_rs::repeating_prompt_stream::repeating_prompt_stream;
    use coral_rs::telemetry::TelemetryMode;
    use rig::client::{CompletionClient, ProviderClient};
    use rig::providers::openai::GPT_4_1_MINI;
    use rig::providers::*;
    use std::time::Duration;

    #[tokio::test]
    #[ignore]
    async fn test_openai() {
        let model = GPT_4_1_MINI;

        let subscriber = tracing_subscriber::FmtSubscriber::new();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");

        let coral = McpConnectionBuilder::from_coral_env()
            .connect()
            .await.expect("Failed to connect to the Coral server");

        let completion_agent = openai::Client::from_env()
            .agent(model)
            .preamble("You are a unit test.")
            .temperature(0.97)
            .max_tokens(512)
            .build();

        let agent = Agent::new(completion_agent)
            .telemetry(TelemetryMode::OpenAI, model)
            .mcp_server(coral);

        let prompt_stream = repeating_prompt_stream(
            "create a thread and send 3 random messages in it",
            Some(Duration::from_secs(1)),
            10
        );

        AgentLoop::new(agent, prompt_stream)
            .execute()
            .await
            .expect("Agent loop failed");
    }
}