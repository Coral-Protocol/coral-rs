pub mod error;
pub mod mcp_server;
pub mod agent_loop;
pub mod agent;
pub mod telemetry;
pub mod api;
pub mod repeating_prompt_stream;
pub mod completion_evaluated_prompt;
pub mod claim_manager;

use std::io;
pub use rig;
pub use serde;
pub use rmcp;
use tracing::Level;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::{SubscriberInitExt, TryInitError};

///
/// Helper function that omits verbose logging information when CORAL_ORCHESTRATION_RUNTIME is set.
/// This is useful for developing Coral agents because during dev-mode development, extra logging
/// information is desired, but when the agents are being orchestrated (during application
/// development), the extra information is duplicated with the server's logging information
pub fn init_tracing() -> Result<(), TryInitError> {
    if std::env::var("CORAL_ORCHESTRATION_RUNTIME").is_ok() {
        let stderr = tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_level(false)
            .without_time()
            .with_writer(io::stderr
                .with_min_level(Level::ERROR)
                .with_max_level(Level::WARN));

        let stdout = tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_level(false)
            .without_time()
            .with_writer(io::stdout
                .with_min_level(Level::INFO)
                .with_max_level(Level::INFO));

        tracing_subscriber::registry()
            .with(stdout)
            .with(stderr)
            .try_init()
    }
    else {
        let stderr = tracing_subscriber::fmt::layer()
            .with_writer(io::stderr
                .with_min_level(Level::ERROR)
                .with_max_level(Level::WARN));

        let stdout = tracing_subscriber::fmt::layer()
            .with_writer(io::stdout
                .with_min_level(Level::INFO)
                .with_max_level(Level::INFO));

        tracing_subscriber::registry()
            .with(stdout)
            .with(stderr)
            .try_init()
    }
}