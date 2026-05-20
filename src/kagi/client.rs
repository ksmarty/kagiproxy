use std::pin::Pin;
use std::task::{Context, Poll};

use axum::response::sse::{Event, Sse};
use futures_util::stream::Stream;
use reqwest::Client;
use std::convert::Infallible;
use tokio::sync::mpsc;

use crate::error::{AppError, Result};
use crate::openai::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, ChatMessage,
    ChatMessageDelta, Choice, ChoiceDelta,
};

fn strip_html(html: &str) -> String {
    let mut content = html.to_string();
    if html.contains("<details>") {
        if let Some(end) = content.find("</details>") {
            content = content[end + 10..].to_string();
        } else {
            return String::new();
        }
    }
    let mut result = String::new();
    let mut in_tag = false;
    for c in content.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}

fn extract_stream_content(event: &serde_json::Value) -> Option<String> {
    if let Some(html) = event.get("html_content").and_then(|v| v.as_str()) {
        if !html.is_empty() {
            let stripped = strip_html(html);
            if !stripped.is_empty() {
                return Some(stripped);
            }
        }
    }
    if let Some(text) = event.get("text").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            let stripped = strip_html(text);
            if !stripped.is_empty() {
                return Some(stripped);
            }
        }
    }
    None
}

pub struct KagiClient {
    client: Client,
    base_url: String,
    auth_header: Option<String>,
}

impl KagiClient {
    pub fn new(base_url: String, token: Option<String>, _auth_header: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url,
            auth_header: token,
        }
    }

    pub async fn chat(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let model = request.model.clone();

        let conversation = self.create_conversation(&model).await?;

        let user_message = request
            .messages
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let message_response = self
            .send_message(&conversation.default_branch.uuid, &user_message, &model)
            .await?;

        let content = self
            .poll_stream(&message_response.stream_url, &model)
            .await?;

        Ok(ChatCompletionResponse::new(
            model,
            vec![Choice::new(
                ChatMessage {
                    role: "assistant".to_string(),
                    content,
                    name: None,
                    tool_calls: None,
                },
                Some("stop"),
            )],
            crate::openai::CompletionUsage::zero(),
        ))
    }

    pub async fn stream_chat(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<Sse<StreamReader>> {
        let model = request.model.clone();

        let conversation = self.create_conversation(&model).await?;

        let user_message = request
            .messages
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let message_response = self
            .send_message(&conversation.default_branch.uuid, &user_message, &model)
            .await?;

        let stream = self
            .create_stream(&message_response.stream_url, &model)
            .await?;
        Ok(Sse::new(StreamReader::new(stream)))
    }

    async fn create_conversation(&self, model: &str) -> Result<ConversationResponse> {
        let url = format!("{}/api/conversations", self.base_url);

        let body = serde_json::json!({
            "model_name": model
        });

        let request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header(
                "Cookie",
                format!("kagi_session={}", self.auth_header.as_ref().unwrap()),
            )
            .json(&body);

        let response = request.send().await.map_err(|e| {
            AppError::UpstreamError(format!("Failed to create conversation: {}", e))
        })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::UpstreamError(format!(
                "Failed to create conversation: {}",
                body
            )));
        }

        let resp: ConversationResponse = response.json().await.map_err(|e| {
            AppError::UpstreamError(format!("Failed to parse conversation response: {}", e))
        })?;

        Ok(resp)
    }

    async fn send_message(
        &self,
        branch_uuid: &str,
        message: &str,
        model: &str,
    ) -> Result<MessageResponse> {
        let url = format!(
            "{}/api/branches/{}/messages",
            self.base_url, branch_uuid
        );

        let body = serde_json::json!({
            "message": message,
            "model_name": model,
            "thinking_preset": null,
            "enable_search": true,
            "personalization": true
        });

        let request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header(
                "Cookie",
                format!("kagi_session={}", self.auth_header.as_ref().unwrap()),
            )
            .json(&body);

        let response = request.send().await.map_err(|e| {
            AppError::UpstreamError(format!("Failed to send message: {}", e))
        })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::UpstreamError(format!(
                "Failed to send message: {}",
                body
            )));
        }

        let resp: MessageResponse = response.json().await.map_err(|e| {
            AppError::UpstreamError(format!("Failed to parse message response: {}", e))
        })?;

        Ok(resp)
    }

    async fn poll_stream(&self, stream_url: &str, _model: &str) -> Result<String> {
        let base_url = self.base_url.clone();
        let auth = self.auth_header.clone().unwrap_or_default();

        let mut cursor = 0u64;
        let mut accumulated = String::new();

        loop {
            let url = format!("{}{}?cursor={}", base_url, stream_url, cursor);

            let response = self
                .client
                .get(&url)
                .header("Cookie", format!("kagi_session={}", &auth))
                .send()
                .await
                .map_err(|e| AppError::UpstreamError(format!("Failed to poll stream: {}", e)))?;

            if !response.status().is_success() {
                break;
            }

            let body = response.text().await.unwrap_or_default();

            for line in body.lines() {
                if line.starts_with("data: ") {
                    let data = line.strip_prefix("data: ").unwrap_or("");

                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(is_final) =
                            event.get("is_final").and_then(|v| v.as_bool())
                        {
                            if is_final {
                                if !accumulated.is_empty() {
                                    return Ok(accumulated);
                                }
                                if let Some(text) = event.get("text").and_then(|v| v.as_str()) {
                                    if !text.is_empty() {
                                        return Ok(strip_html(text));
                                    }
                                }
                                return Ok(accumulated);
                            }
                        }

                        if let Some(content) = extract_stream_content(&event) {
                            if content.len() > accumulated.len() {
                                accumulated = content;
                            }
                        }

                        if let Some(id) = event.get("cursor").and_then(|v| v.as_u64()) {
                            cursor = id;
                        }
                    }
                } else if line.starts_with("id: ") {
                    if let Ok(id) = line.strip_prefix("id: ").unwrap_or("").parse::<u64>() {
                        if id >= cursor {
                            cursor = id + 1;
                        }
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        Ok(accumulated)
    }

    async fn create_stream(
        &self,
        stream_url: &str,
        model: &str,
    ) -> std::result::Result<mpsc::Receiver<std::result::Result<Event, Infallible>>, AppError>
    {
        let base_url = self.base_url.clone();
        let auth = self.auth_header.clone().unwrap_or_default();
        let client = self.client.clone();
        let stream_url = stream_url.to_string();
        let model = model.to_string();

        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            let mut cursor = 0u64;
            let mut previous_content = String::new();

            loop {
                let url = format!("{}{}?cursor={}", base_url, stream_url, cursor);

                let request = client
                    .get(&url)
                    .header("Cookie", format!("kagi_session={}", &auth));

                match request.send().await {
                    Ok(response) if response.status().is_success() => {
                        let body = response.text().await.unwrap_or_default();

                        if body.trim().is_empty() {
                            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                            continue;
                        }

                        for line in body.lines() {
                            if line.starts_with("id: ") {
                                if let Ok(id) =
                                    line.strip_prefix("id: ").unwrap_or("").parse::<u64>()
                                {
                                    if id >= cursor {
                                        cursor = id + 1;
                                    }
                                }
                            } else if line.starts_with("data: ") {
                                let data = line.strip_prefix("data: ").unwrap_or("");

                                if let Ok(event) =
                                    serde_json::from_str::<serde_json::Value>(data)
                                {
                                    if let Some(true) =
                                        event.get("is_final").and_then(|v| v.as_bool())
                                    {
                                        let done = ChatCompletionChunk::new(
                                            model.clone(),
                                            vec![ChoiceDelta {
                                                index: 0,
                                                delta: ChatMessageDelta {
                                                    role: None,
                                                    content: None,
                                                    tool_calls: None,
                                                },
                                                finish_reason: Some("stop".to_string()),
                                            }],
                                        );
                                        if let Ok(json) = serde_json::to_string(&done) {
                                            let _ = tx
                                                .send(Ok(Event::default().data(json)))
                                                .await;
                                        }
                                        return;
                                    }

                                    if let Some(content) = extract_stream_content(&event) {
                                        if content.len() > previous_content.len() {
                                            let delta = content[previous_content.len()..].to_string();
                                            previous_content = content;

                                            if !delta.is_empty() {
                                                let chunk = ChatCompletionChunk::new(
                                                    model.clone(),
                                                    vec![ChoiceDelta::content(&delta)],
                                                );
                                                if let Ok(json) =
                                                    serde_json::to_string(&chunk)
                                                {
                                                    let _ = tx
                                                        .send(Ok(Event::default().data(json)))
                                                        .await;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(_) | Err(_) => {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
            }
        });

        Ok(rx)
    }

    pub async fn fetch_models(&self) -> Result<Vec<KagiModel>> {
        let auth = match self.auth_header.as_ref() {
            Some(token) => token.clone(),
            None => return Ok(Vec::new()),
        };
        let url = format!("{}/api/init", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("Cookie", format!("kagi_session={}", auth))
            .send()
            .await
            .map_err(|e| AppError::UpstreamError(format!("Failed to fetch models: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::UpstreamError(format!(
                "Failed to fetch models: {}",
                body
            )));
        }

        let resp: KagiInitResponse = response.json().await.map_err(|e| {
            AppError::UpstreamError(format!("Failed to parse init response: {}", e))
        })?;

        Ok(resp
            .models
            .models
            .into_iter()
            .filter(|m| m.supported.unwrap_or(false) && !m.deprecated.unwrap_or(false) && !m.retired.unwrap_or(false))
            .collect())
    }
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct ConversationResponse {
    conversation: Conversation,
    #[serde(rename = "default_branch")]
    default_branch: Branch,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct Conversation {
    uuid: String,
}

#[derive(Debug, serde::Deserialize)]
struct Branch {
    uuid: String,
}

#[derive(Debug, serde::Deserialize)]
struct MessageResponse {
    #[serde(rename = "stream_url")]
    stream_url: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KagiModel {
    pub id: String,
    pub provider: Option<String>,
    #[serde(rename = "provider_label")]
    pub provider_label: Option<String>,
    #[serde(rename = "display_name")]
    pub display_name: Option<String>,
    #[serde(rename = "context_window")]
    pub context_window: Option<u64>,
    pub supported: Option<bool>,
    pub deprecated: Option<bool>,
    pub retired: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
struct KagiModelsSection {
    models: Vec<KagiModel>,
}

#[derive(Debug, serde::Deserialize)]
struct KagiInitResponse {
    models: KagiModelsSection,
}

pub struct StreamReader {
    inner: mpsc::Receiver<std::result::Result<Event, Infallible>>,
}

impl StreamReader {
    fn new(inner: mpsc::Receiver<std::result::Result<Event, Infallible>>) -> Self {
        Self { inner }
    }
}

impl Stream for StreamReader {
    type Item = std::result::Result<Event, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_recv(cx)
    }
}
