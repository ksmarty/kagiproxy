use serde::{Deserialize, Serialize};

use crate::openai::{
    ChatCompletionChunk, ChatMessage, ChatMessageDelta, Choice, ChoiceDelta,
    ChatCompletionResponse, CompletionUsage,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KagiRequest {
    pub model: String,
    pub messages: Vec<KagiMessage>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stop: Option<serde_json::Value>,
    pub stream: bool,
    pub presence_penalty: Option<f64>,
    pub frequency_penalty: Option<f64>,
    pub user: Option<String>,
}

impl KagiRequest {
    pub fn from_openai(req: &crate::openai::ChatCompletionRequest) -> Self {
        Self {
            model: req.model.clone(),
            messages: req.messages.iter().map(KagiMessage::from).collect(),
            temperature: req.temperature,
            top_p: req.top_p,
            max_tokens: req.max_tokens,
            stop: req.stop.clone(),
            stream: req.stream.unwrap_or(false),
            presence_penalty: req.presence_penalty,
            frequency_penalty: req.frequency_penalty,
            user: req.user.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KagiMessage {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl From<&crate::openai::ChatMessage> for KagiMessage {
    fn from(msg: &crate::openai::ChatMessage) -> Self {
        Self {
            role: msg.role.clone(),
            content: msg.content.clone(),
            name: msg.name.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KagiResponse {
    pub id: Option<String>,
    pub choices: Vec<KagiChoice>,
    pub usage: Option<KagiUsage>,
}

impl KagiResponse {
    pub fn into_openai_response(self, model: &str) -> crate::error::Result<ChatCompletionResponse> {
        let choices: Vec<Choice> = self
            .choices
            .into_iter()
            .map(|c| {
                let content = c.message
                    .as_ref()
                    .and_then(|m| m.content.as_deref())
                    .unwrap_or_default()
                    .to_string();
                let role = c.message
                    .as_ref()
                    .and_then(|m| m.role.clone())
                    .unwrap_or_else(|| "assistant".to_string());
                Choice::new(
                    ChatMessage {
                        role,
                        content,
                        name: None,
                        tool_calls: None,
                    },
                    c.finish_reason.as_deref(),
                )
            })
            .collect();

        let usage = self.usage.map(|u| CompletionUsage {
            prompt_tokens: u.prompt_tokens.unwrap_or(0),
            completion_tokens: u.completion_tokens.unwrap_or(0),
            total_tokens: u.total_tokens.unwrap_or(0),
        }).unwrap_or(CompletionUsage::zero());

        Ok(ChatCompletionResponse::new(
            model.to_string(),
            choices,
            usage,
        ))
    }

    pub fn to_openai_chunk(&self) -> ChatCompletionChunk {
        let choices: Vec<ChoiceDelta> = self
            .choices
            .iter()
            .map(|c| {
                let content = c.delta
                    .as_ref()
                    .and_then(|d| d.content.as_deref())
                    .map(|s| s.to_string());
                let role = c.delta
                    .as_ref()
                    .and_then(|d| d.role.clone());
                ChoiceDelta {
                    index: c.index.unwrap_or(0),
                    delta: ChatMessageDelta {
                        role,
                        content,
                        tool_calls: None,
                    },
                    finish_reason: c.finish_reason.clone(),
                }
            })
            .collect();

        ChatCompletionChunk::new("gpt-4o".to_string(), choices)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KagiChoice {
    pub index: Option<u32>,
    #[serde(default)]
    pub message: Option<KagiMessageContent>,
    #[serde(default)]
    pub delta: Option<KagiDelta>,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KagiMessageContent {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KagiDelta {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KagiUsage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}