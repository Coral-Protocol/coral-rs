use crate::completion_evaluated_prompt::CompletionEvaluatedPrompt;
use futures::{Stream, stream};
use std::time::Duration;
use tokio::time::sleep;

pub fn repeating_prompt_stream(
    prompt: impl Into<CompletionEvaluatedPrompt>,
    delay: Option<Duration>,
    max_reps: usize,
) -> impl Stream<Item = CompletionEvaluatedPrompt> {
    stream::unfold(
        (prompt.into(), delay, max_reps, 0),
        |(prompt, delay, max_reps, reps)| {
            Box::pin(async move {
                if reps >= max_reps {
                    return None;
                }

                if reps > 0 {
                    if let Some(delay_duration) = delay {
                        sleep(delay_duration).await;
                    }
                }

                Some((prompt.clone(), (prompt, delay, max_reps, reps + 1)))
            })
        },
    )
}
