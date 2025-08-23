#[allow(renamed_and_removed_lints)]
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/api_v1.rs"));
}

use generated::types;

impl From<rig::message::Message> for types::GenericMessage {
    fn from(value: rig::message::Message) -> Self {
        match value {
            rig::message::Message::User { content } => {
                types::GenericMessage::User {
                    content: content.into_iter().map(|x| x.into()).collect()
                }
            }
            rig::message::Message::Assistant { id, content } => {
                types::GenericMessage::Assistant {
                    id,
                    content: content.into_iter().map(|x| x.into()).collect()
                }
            }
        }
    }
}

impl From<rig::message::UserContent> for types::GenericUserContent {
    fn from(value: rig::message::UserContent) -> Self {
        match value {
            rig::message::UserContent::Text(text) => {
                types::GenericUserContent::Text { text: text.text }
            },
            rig::message::UserContent::ToolResult(tool_result) => {
                types::GenericUserContent::ToolResult {
                    id: tool_result.id,
                    call_id: tool_result.call_id,
                    content: tool_result.content
                        .into_iter()
                        .map(|x| x.into())
                        .collect(),
                }
            },
            rig::message::UserContent::Image(image) => {
                types::GenericUserContent::Image {
                    data: image.data,
                    detail: image.detail.map(Into::into),
                    format: image.format.map(Into::into),
                    media_type: image.media_type.map(Into::into),
                }
            },
            rig::message::UserContent::Audio(audio) => {
                types::GenericUserContent::Audio {
                    data: audio.data,
                    format: audio.format.map(Into::into),
                    media_type: audio.media_type.map(Into::into),
                }
            },
            rig::message::UserContent::Document(doc) => {
                types::GenericUserContent::Document {
                    data: doc.data,
                    format: doc.format.map(Into::into),
                    media_type: doc.media_type.map(Into::into)
                }
            },
            rig::message::UserContent::Video(video) => {
                types::GenericUserContent::Video {
                    data: video.data,
                    format: video.format.map(Into::into),
                    media_type: video.media_type.map(Into::into),
                }
            }
        }
    }
}

impl From<rig::message::AssistantContent> for types::GenericAssistantContent {
    fn from(value: rig::message::AssistantContent) -> Self {
        match value {
            rig::message::AssistantContent::Text(text) => {
                types::GenericAssistantContent::AssistantText {
                    text: text.text
                }
            },
            rig::message::AssistantContent::ToolCall(tool_call) => {
                types::GenericAssistantContent::AssistantToolCall {
                    call_id: tool_call.call_id,
                    function: tool_call.function.into(),
                    id: tool_call.id,
                }
            },
            rig::message::AssistantContent::Reasoning(reasoning) => {
                types::GenericAssistantContent::AssistantReasoning {
                    reasoning: reasoning.reasoning,
                }
            }
        }
    }
}

impl From<rig::message::ToolResultContent> for types::GenericToolResultContent {
    fn from(value: rig::message::ToolResultContent) -> Self {
        match value {
            rig::message::ToolResultContent::Text(text) => {
                types::GenericToolResultContent::ToolText { text: text.text }
            },
            rig::message::ToolResultContent::Image(image) => {
                types::GenericToolResultContent::ToolImage {
                    data: image.data,
                    detail: image.detail.map(Into::into),
                    format: image.format.map(Into::into),
                    media_type: image.media_type.map(Into::into),
                }
            }
        }
    }
}

impl From<rig::message::ImageDetail> for types::ImageDetail {
    fn from(value: rig::message::ImageDetail) -> Self {
        match value {
            rig::message::ImageDetail::Low => types::ImageDetail::Low,
            rig::message::ImageDetail::High => types::ImageDetail::High,
            rig::message::ImageDetail::Auto => types::ImageDetail::Auto,
        }
    }
}

impl From<rig::message::ContentFormat> for types::ContentFormat {
    fn from(value: rig::message::ContentFormat) -> Self {
        match value {
            rig::message::ContentFormat::String => types::ContentFormat::String,
            rig::message::ContentFormat::Base64 => types::ContentFormat::Base64
        }
    }
}

impl From<rig::message::VideoMediaType> for types::VideoMediaType {
    fn from(value: rig::message::VideoMediaType) -> Self {
        match value {
            rig::message::VideoMediaType::AVI => types::VideoMediaType::Avi,
            rig::message::VideoMediaType::MP4 => types::VideoMediaType::Mp4,
            rig::message::VideoMediaType::MPEG => types::VideoMediaType::Mpeg,
        }
    }
}

impl From<rig::message::ImageMediaType> for types::ImageMediaType {
    fn from(value: rig::message::ImageMediaType) -> Self {
        match value {
            rig::message::ImageMediaType::JPEG => types::ImageMediaType::Jpeg,
            rig::message::ImageMediaType::PNG => types::ImageMediaType::Png,
            rig::message::ImageMediaType::GIF => types::ImageMediaType::Gif,
            rig::message::ImageMediaType::WEBP => types::ImageMediaType::Webp,
            rig::message::ImageMediaType::HEIC => types::ImageMediaType::Heic,
            rig::message::ImageMediaType::HEIF => types::ImageMediaType::Heif,
            rig::message::ImageMediaType::SVG => types::ImageMediaType::Svg,
        }
    }
}

impl From<rig::message::AudioMediaType> for types::AudioMediaType {
    fn from(value: rig::message::AudioMediaType) -> Self {
        match value {
            rig::message::AudioMediaType::WAV => types::AudioMediaType::Wav,
            rig::message::AudioMediaType::MP3 => types::AudioMediaType::Mp3,
            rig::message::AudioMediaType::AIFF => types::AudioMediaType::Aiff,
            rig::message::AudioMediaType::AAC => types::AudioMediaType::Aac,
            rig::message::AudioMediaType::OGG => types::AudioMediaType::Ogg,
            rig::message::AudioMediaType::FLAC => types::AudioMediaType::Flac,
        }
    }
}

impl From<rig::message::DocumentMediaType> for types::DocumentMediaType {
    fn from(value: rig::message::DocumentMediaType) -> Self {
        match value {
            rig::message::DocumentMediaType::PDF => types::DocumentMediaType::Pdf,
            rig::message::DocumentMediaType::TXT => types::DocumentMediaType::Txt,
            rig::message::DocumentMediaType::RTF => types::DocumentMediaType::Rtf,
            rig::message::DocumentMediaType::HTML => types::DocumentMediaType::Html,
            rig::message::DocumentMediaType::CSS => types::DocumentMediaType::Css,
            rig::message::DocumentMediaType::MARKDOWN => types::DocumentMediaType::Markdown,
            rig::message::DocumentMediaType::CSV => types::DocumentMediaType::Csv,
            rig::message::DocumentMediaType::XML => types::DocumentMediaType::Xml,
            rig::message::DocumentMediaType::Javascript => types::DocumentMediaType::Javascript,
            rig::message::DocumentMediaType::Python => types::DocumentMediaType::Python,
        }
    }
}

impl From<rig::completion::message::ToolFunction> for types::ToolFunction {
    fn from(value: rig::completion::message::ToolFunction) -> Self {
        types::ToolFunction {
            arguments: value.arguments.to_string(),
            name: value.name,
        }
    }
}

impl From<rig::providers::openai::Message> for types::OpenAiMessage {
    fn from(value: rig::providers::openai::Message) -> Self {
        match value {
            rig::providers::openai::Message::System {
                content,
                name
            } => {
                types::OpenAiMessage::Developer {
                    content: content.into_iter().map(Into::into).collect(),
                    name,
                }
            },
            rig::providers::openai::Message::User {
                content,
                name
            } => {
                types::OpenAiMessage::User {
                    content: content.into_iter().map(Into::into).collect(),
                    name
                }
            },
            rig::providers::openai::Message::Assistant {
                content,
                refusal,
                audio,
                name,
                tool_calls
            } => {
                types::OpenAiMessage::Assistant {
                    content: content.into_iter().map(Into::into).collect(),
                    refusal,
                    audio: audio.map(Into::into),
                    name,
                    tool_calls: tool_calls.into_iter().map(Into::into).collect(),
                }
            },
            rig::providers::openai::Message::ToolResult {
                tool_call_id,
                content
            } => {
                types::OpenAiMessage::Tool {
                    tool_call_id,
                    content: content.into_iter().map(Into::into).collect(),
                }
            }
        }
    }
}

impl From<rig::providers::openai::SystemContent> for types::OpenAiSystemContent {
    fn from(value: rig::providers::openai::SystemContent) -> Self {
        types::OpenAiSystemContent {
            text: value.text,
            type_: value.r#type.into(),
        }
    }
}

impl From<rig::providers::openai::UserContent> for types::OpenAiUserContent {
    fn from(value: rig::providers::openai::UserContent) -> Self {
        match value {
            rig::providers::openai::UserContent::Text { text } => {
                types::OpenAiUserContent::Text { text }
            }
            rig::providers::openai::UserContent::Image { image_url } => {
                types::OpenAiUserContent::ImageUrl {
                    image_url: image_url.into()
                }
            }
            rig::providers::openai::UserContent::Audio { input_audio } => {
                types::OpenAiUserContent::Audio {
                    input_audio: input_audio.into()
                }
            }
        }
    }
}

impl From<rig::providers::openai::AssistantContent> for types::OpenAiAssistantContent {
    fn from(value: rig::providers::openai::AssistantContent) -> Self {
        match value {
            rig::providers::openai::AssistantContent::Text { text } => {
                types::OpenAiAssistantContent::Text { text }
            }
            rig::providers::openai::AssistantContent::Refusal { refusal } => {
                types::OpenAiAssistantContent::Refusal { refusal }
            }
        }
    }
}

impl From<rig::providers::openai::ToolCall> for types::ToolCall {
    fn from(value: rig::providers::openai::ToolCall) -> Self {
        types::ToolCall {
            function: value.function.into(),
            id: value.id,
            type_: value.r#type.into(),
        }
    }
}

impl From<rig::providers::openai::ToolResultContent> for types::OpenAiToolResultContent {
    fn from(value: rig::providers::openai::ToolResultContent) -> Self {
        types::OpenAiToolResultContent {
            text: value.text,
            // Text is the only supported type, and rig's field for this is private
            type_: types::ToolResultContentType::Text,
        }
    }
}

impl From<rig::providers::openai::SystemContentType> for types::SystemContentType {
    fn from(value: rig::providers::openai::SystemContentType) -> Self {
        match value {
            rig::providers::openai::SystemContentType::Text => types::SystemContentType::Text
        }
    }
}

impl From<rig::providers::openai::ImageUrl> for types::ImageUrl {
    fn from(value: rig::providers::openai::ImageUrl) -> Self {
        types::ImageUrl {
            detail: value.detail.into(),
            url: value.url,
        }
    }
}

impl From<rig::providers::openai::InputAudio> for types::InputAudio {
    fn from(value: rig::providers::openai::InputAudio) -> Self {
        types::InputAudio {
            data: value.data,
            format: value.format.into(),
        }
    }
}

impl From<rig::providers::openai::ToolType> for types::ToolType {
    fn from(value: rig::providers::openai::ToolType) -> Self {
        match value {
            rig::providers::openai::ToolType::Function => types::ToolType::Function
        }
    }
}

impl From<rig::providers::openai::Function> for types::Function {
    fn from(value: rig::providers::openai::Function) -> Self {
        types::Function {
            arguments: value.arguments.to_string(),
            name: value.name,
        }
    }
}

impl From<rig::providers::openai::AudioAssistant> for types::AudioAssistant {
    fn from(value: rig::providers::openai::AudioAssistant) -> Self {
        types::AudioAssistant {
            id: value.id,
        }
    }
}