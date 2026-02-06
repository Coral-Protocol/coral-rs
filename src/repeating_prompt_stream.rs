use crate::completion_evaluated_prompt::CompletionEvaluatedPrompt;
use futures::{Stream, stream};
use std::time::Duration;
use tokio::time::sleep;

pub fn repeating_prompt_stream(
    initial_prompt: impl Into<CompletionEvaluatedPrompt>,
    repeating_prompt: impl Into<CompletionEvaluatedPrompt>,
    delay: Option<Duration>,
    max_reps: usize,
) -> impl Stream<Item = CompletionEvaluatedPrompt> {
    stream::unfold(
        (
            initial_prompt.into(),
            repeating_prompt.into(),
            delay,
            max_reps,
            0,
        ),
        |(initial_prompt, repeating_prompt, delay, max_reps, reps)| {
            Box::pin(async move {
                if reps >= max_reps {
                    return None;
                }

                if reps > 0 {
                    if let Some(delay_duration) = delay {
                        sleep(delay_duration).await;
                    }
                }

                let prompt = if reps == 0 {
                    initial_prompt.clone()
                } else {
                    repeating_prompt.clone()
                };

                Some((
                    prompt,
                    (initial_prompt, repeating_prompt, delay, max_reps, reps + 1),
                ))
            })
        },
    )
}
