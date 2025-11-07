use crate::api::generated::Client;
use crate::api::generated::types::{
    OpenAiMessage, RouteException, Telemetry, TelemetryMessages, TelemetryPost, TelemetryTarget,
};
use progenitor::progenitor_client::Error as ProgenitorError;
use rig::completion::{CompletionModel, Document};
use serde::Serialize;
use thiserror::Error;
use tracing::warn;

///
/// Telemetry is debugging information attached to Coral messages. The telemetry data should
/// provide all relevant data that influenced a language model's completion response.
pub(crate) struct TelemetryRequest<'a, M: CompletionModel> {
    id: TelemetryIdentifier,
    url: String,
    messages: Vec<rig::completion::Message>,
    telemetry_mode: TelemetryMode,
    agent: &'a rig::agent::Agent<M>,
    model_description: String,
}

#[derive(Serialize, Copy, Clone)]
pub enum TelemetryMode {
    ///
    /// No telemetry
    None,

    ///
    /// The OpenAI format is a familiar message format used by OpenAI and other model providers.
    /// This format does not support every field that Rig does, if a comprehensive format is
    /// required, [`TelemetryMode::Generic`] should be used.
    OpenAI,

    ///
    /// Generic/Rig message format.  This message format supports every field that Rig does and
    /// is generally more portable than [`TelemetryMode::OpenAI`] but is rig-opinionated and
    /// unlikely to be familiar.
    Generic,
}

pub(crate) struct TelemetryIdentifier {
    pub targets: Vec<TelemetryTarget>,
    pub session_id: String,
}

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("failed to send telemetry {0}")]
    Request(ProgenitorError<RouteException>),

    #[error("no targets provided")]
    EmptyTargets,

    #[error("no messages provided")]
    EmptyMessages,
}

impl<'a, M: CompletionModel> TelemetryRequest<'a, M> {
    pub(crate) fn new(
        id: TelemetryIdentifier,
        url: String,
        agent: &'a rig::agent::Agent<M>,
        model_description: impl Into<String>,
        messages: Vec<rig::completion::Message>,
    ) -> Self {
        Self {
            id,
            url,
            messages,
            telemetry_mode: TelemetryMode::OpenAI,
            agent,
            model_description: model_description.into(),
        }
    }

    pub(crate) fn telemetry_mode(mut self, format: TelemetryMode) -> Self {
        self.telemetry_mode = format;
        self
    }

    ///
    /// Formats telemetry messages in OpenAI format.  Note that OpenAI's message type only provides
    /// try_into; a generic -> openai conversion can fail.  Any conversion failure here will result
    /// in this function returning None.
    fn messages_openai(&self) -> Option<Vec<OpenAiMessage>> {
        let mut messages = Vec::new();
        for msg in &self.messages {
            let openai_messages: Vec<rig::providers::openai::Message> =
                msg.clone().try_into().ok()?;
            messages.extend(openai_messages);
        }

        Some(messages.into_iter().map(Into::into).collect())
    }

    ///
    /// Returns messages in API format
    fn messages_generic(self) -> Vec<crate::api::generated::types::GenericMessage> {
        self.messages.into_iter().map(Into::into).collect()
    }

    ///
    /// Converts Rig documents into Telemetry documents.  Ideally identical types, but Telemetry
    /// comes from an OpenAPI spec and is a different rust type.
    fn convert_documents(resources: Vec<Document>) -> Vec<crate::api::generated::types::Document> {
        resources
            .into_iter()
            .map(|x| crate::api::generated::types::Document {
                id: x.id,
                text: x.text,
            })
            .collect()
    }

    ///
    /// Formats the Telemetry struct into data that the Coral server expects
    async fn format(self) -> TelemetryPost {
        TelemetryPost {
            targets: self.id.targets.clone(),
            data: Telemetry {
                // additional_params: self.agent.additional_params.clone(),
                additional_params: Default::default(),
                max_tokens: self.agent.max_tokens.map(|t| t as i64),
                model_description: self.model_description.clone(),
                preamble: Some(self.agent.preamble.clone()),
                resources: Self::convert_documents(self.agent.static_context.clone()),
                temperature: self.agent.temperature.clone(),
                tools: Self::convert_documents(
                    self.agent.tools.documents().await.unwrap_or_default(),
                ),
                messages: match self.telemetry_mode {
                    TelemetryMode::OpenAI => match self.messages_openai() {
                        None => {
                            warn!(
                                "OpenAI message format requested for telemetry but model response could not convert.  Falling back to generic format."
                            );
                            TelemetryMessages::Generic(self.messages_generic())
                        }
                        Some(messages) => TelemetryMessages::OpenAi(messages),
                    },
                    TelemetryMode::Generic => TelemetryMessages::Generic(self.messages_generic()),
                    TelemetryMode::None => panic!("cannot send telemetry in None mode"),
                },
            },
        }
    }

    ///
    /// Serializes the contained telemetry information and sends it to the Coral server
    pub(crate) async fn send(self) -> Result<(), Error> {
        if self.id.targets.is_empty() {
            return Err(Error::EmptyTargets);
        }

        if self.messages.is_empty() {
            return Err(Error::EmptyMessages);
        }

        let url = self.url.clone();
        let session_id = self.id.session_id.clone();
        let data = self.format().await;
        Client::new(url.as_str())
            .add_telemetry(session_id.as_str(), &data)
            .await
            .map_err(Error::Request)?;

        Ok(())
    }
}
