use std::pin::Pin;
use std::task::{Context, Poll};

use axum::response::sse::{Event, Sse};
use futures_util::stream::Stream;
use reqwest::Client;
use std::convert::Infallible;
use tokio::sync::mpsc;

use crate::error::{AppError, Result};
use crate::openai::{ChatCompletionRequest, ChatCompletionResponse, ChatCompletionChunk, ChatMessage, Choice};

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
        
        let user_message = request.messages.last()
            .map(|m| m.content.clone())
            .unwrap_or_default();
        
        let message_response = self.send_message(
            &conversation.default_branch.uuid,
            &user_message,
            &model,
        ).await?;
        
        let content = self.poll_stream(&message_response.stream_url).await?;
        
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

    pub async fn stream_chat(&self, request: ChatCompletionRequest) -> Result<Sse<StreamReader>> {
        let model = request.model.clone();
        
        let conversation = self.create_conversation(&model).await?;
        
        let user_message = request.messages.last()
            .map(|m| m.content.clone())
            .unwrap_or_default();
        
        let message_response = self.send_message(
            &conversation.default_branch.uuid,
            &user_message,
            &model,
        ).await?;
        
        let stream = self.create_stream(&message_response.stream_url).await?;
        Ok(Sse::new(StreamReader::new(stream)))
    }

    async fn create_conversation(&self, model: &str) -> Result<ConversationResponse> {
        let url = format!("{}/api/conversations", self.base_url);
        
        let body = serde_json::json!({
            "model_name": map_model(model)
        });
        
        let request = self.client.post(&url)
            .header("Content-Type", "application/json")
            .header("Cookie", format!("kagi_session={}", self.auth_header.as_ref().unwrap()))
            .json(&body);
        
        let response = request.send().await.map_err(|e| {
            AppError::UpstreamError(format!("Failed to create conversation: {}", e))
        })?;
        
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::UpstreamError(format!("Failed to create conversation: {}", body)));
        }
        
        let resp: ConversationResponse = response.json().await.map_err(|e| {
            AppError::UpstreamError(format!("Failed to parse conversation response: {}", e))
        })?;
        
        Ok(resp)
    }

    async fn send_message(&self, branch_uuid: &str, message: &str, model: &str) -> Result<MessageResponse> {
        let url = format!("{}/api/branches/{}/messages", self.base_url, branch_uuid);
        
        let body = serde_json::json!({
            "message": message,
            "model_name": map_model(model),
            "thinking_preset": null,
            "enable_search": true,
            "personalization": true
        });
        
        let request = self.client.post(&url)
            .header("Content-Type", "application/json")
            .header("Cookie", format!("kagi_session={}", self.auth_header.as_ref().unwrap()))
            .json(&body);
        
        let response = request.send().await.map_err(|e| {
            AppError::UpstreamError(format!("Failed to send message: {}", e))
        })?;
        
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::UpstreamError(format!("Failed to send message: {}", body)));
        }
        
        let resp: MessageResponse = response.json().await.map_err(|e| {
            AppError::UpstreamError(format!("Failed to parse message response: {}", e))
        })?;
        
        Ok(resp)
    }

    async fn poll_stream(&self, stream_url: &str) -> Result<String> {
        let base_url = self.base_url.clone();
        let auth = self.auth_header.clone().unwrap_or_default();
        
        let mut cursor = 0u64;
        
        loop {
            let url = format!("{}{}?cursor={}", base_url, stream_url, cursor);
            
            let response = self.client.get(&url)
                .header("Cookie", format!("kagi_session={}", &auth))
                .send().await.map_err(|e| {
                    AppError::UpstreamError(format!("Failed to poll stream: {}", e))
                })?;
            
            if !response.status().is_success() {
                break;
            }
            
            let body = response.text().await.unwrap_or_default();
            
            for line in body.lines() {
                if line.starts_with("data: ") {
                    let data = line.strip_prefix("data: ").unwrap_or("");
                    
                    if data == "[DONE]" {
                        return Ok(String::new());
                    }
                    
                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(text) = event.get("text").and_then(|v| v.as_str()) {
                            if !text.is_empty() {
                                return Ok(text.to_string());
                            }
                        }
                        if let Some(is_final) = event.get("is_final").and_then(|v| v.as_bool()) {
                            if is_final {
                                if let Some(text) = event.get("text").and_then(|v| v.as_str()) {
                                    return Ok(text.to_string());
                                }
                            }
                        }
                    }
                }
            }
            
            cursor += 1;
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        
        Ok(String::new())
    }

async fn create_stream(&self, stream_url: &str) -> std::result::Result<mpsc::Receiver<std::result::Result<Event, Infallible>>, AppError> {
        let base_url = self.base_url.clone();
        let auth = self.auth_header.clone().unwrap_or_default();
        let client = self.client.clone();
        let stream_url = stream_url.to_string();
        
        let (tx, rx) = mpsc::channel(100);
        
        tokio::spawn(async move {
            let mut cursor = 0u64;
            
            loop {
                let url = format!("{}{}?cursor={}", base_url, stream_url, cursor);
                
                let request = client
                    .get(&url)
                    .header("Cookie", format!("kagi_session={}", &auth));
                
                match request.send().await {
                    Ok(response) if response.status().is_success() => {
                        let body = response.text().await.unwrap_or_default();
                        
                        for line in body.lines() {
                            if line.starts_with("data: ") {
                                let data = line.strip_prefix("data: ").unwrap_or("");
                                
                                if data == "[DONE]" {
                                    let done = ChatCompletionChunk::done("gpt-4o".to_string());
                                    if let Ok(event) = serde_json::to_string(&done) {
                                        let _ = tx.send(Ok(Event::default().data(event))).await;
                                    }
                                    return;
                                }
                                
                                if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                                    if let Some(text) = event.get("text").and_then(|v| v.as_str()) {
                                        if !text.is_empty() {
                                            let chunk = ChatCompletionChunk::new(
                                                "gpt-4o".to_string(),
                                                vec![crate::openai::ChoiceDelta {
                                                    index: 0,
                                                    delta: crate::openai::ChatMessageDelta {
                                                        role: None,
                                                        content: Some(text.to_string()),
                                                        tool_calls: None,
                                                    },
                                                    finish_reason: None,
                                                }],
                                            );
                                            if let Ok(event) = serde_json::to_string(&chunk) {
                                                let _ = tx.send(Ok(Event::default().data(event))).await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(_) | Err(_) => {}
                }
                
                cursor += 1;
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
        
Ok(rx)
    }
}

fn map_model(model: &str) -> &str {
    match model {
        "gpt-4o" => "kimi-k2-6-thinking",
        "gpt-4o-mini" => "kimi-k2-thinking",
        "claude-sonnet" => "claude-sonnet-4-20250514",
        "claude-opus" => "claude-opus-4-20251113",
        _ => "kimi-k2-6-thinking",
    }
}

#[derive(Debug, serde::Deserialize)]
struct ConversationResponse {
    conversation: Conversation,
    #[serde(rename = "default_branch")]
    default_branch: Branch,
}

#[derive(Debug, serde::Deserialize)]
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

#[derive(Debug, serde::Deserialize)]
struct StreamResponse {
    token: Option<String>,
    done: Option<bool>,
    cursor: Option<u64>,
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