use coral_rs::agent::Agent;
use coral_rs::agent_loop::AgentLoop;
use coral_rs::completion_evaluated_prompt::CompletionEvaluatedPrompt;
use coral_rs::init_tracing;
use coral_rs::mcp_server::McpConnectionBuilder;
use coral_rs::repeating_prompt_stream::repeating_prompt_stream;
use coral_rs::rig::client::CompletionClient;
use coral_rs::rig::client::ProviderClient;
use coral_rs::rig::message::ToolChoice;
use coral_rs::rig::providers::anthropic;

#[tokio::main]
async fn main() {
    init_tracing().expect("setting default subscriber failed");

    let coral_mcp = McpConnectionBuilder::build_coral_streamable_http()
        .await
        .expect("Failed to connect to the Coral server");

    let completion_agent = anthropic::Client::from_env()
        .agent("claude-sonnet-4-5")
        .max_tokens(4096)
        .tool_choice(ToolChoice::Required)
        .build();

    let agent = Agent::new(
        completion_agent,
        CompletionEvaluatedPrompt::new()
            .all_resources(coral_mcp.clone())
            .string("You are the Replicate agent, you must use replicate tooling to assist other agents"),
    ).mcp_server(coral_mcp);

    let initial_user_prompt = CompletionEvaluatedPrompt::new()
        .string("[automated message] You are an autonomous agent designed to assist users by collaborating with other agents.")
        .string("If no instructions are provided, consider waiting for mentions until another agent provides further direction.")
        .string("Remember that 'I' am not the user, who is not directly reachable. Use tools to interact with other agents as necessary to fulfil the users needs. You will receive further automated messages this way.");

    let followup_user_prompt = CompletionEvaluatedPrompt::from_string(
        "[automated message] Continue fulfilling your responsibilities collaboratively to the best of your ability.",
    );

    let prompt_stream = repeating_prompt_stream(initial_user_prompt, followup_user_prompt, None, 1);

    AgentLoop::new(agent, prompt_stream)
        .execute()
        .await
        .expect("Agent loop failed");
}
