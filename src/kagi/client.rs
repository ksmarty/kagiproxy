use std::pin::Pin;
use std::task::{Context, Poll};

use axum::response::sse::{Event, Sse};
use futures_util::stream::{Stream, StreamExt};
use reqwest::Client;
use std::convert::Infallible;
use tokio::sync::mpsc;

use crate::error::{AppError, Result};
use crate::openai::{ChatCompletionRequest, ChatCompletionResponse, ChatCompletionChunk};
use crate::kagi::mapper::{KagiRequest, KagiResponse};

pub struct KagiClient {
    client: Client,
    base_url: String,
    auth_header: Option<(String, String)>,
}

impl KagiClient {
    pub fn new(base_url: String, token: Option<String>, auth_header: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        let auth_header = match (auth_header, token) {
            (Some(name), Some(token)) => Some((name, token)),
            (None, Some(token)) => Some(("Authorization".to_string(), token)),
            _ => None,
        };

        Self {
            client,
            base_url,
            auth_header,
        }
    }

    pub async fn chat(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let kagi_req = KagiRequest::from_openai(&request);
        let kagi_response = self.post("/api/v0/chat", &kagi_req).await?;
        kagi_response.into_openai_response(&request.model)
    }

    pub async fn stream_chat(&self, request: ChatCompletionRequest) -> Result<Sse<StreamReader>> {
        let kagi_req = KagiRequest::from_openai(&request);
        let response = self.stream_post("/api/v0/chat/stream", &kagi_req).await?;
        Ok(Sse::new(StreamReader::new(response)))
    }

    async fn post(&self, path: &str, body: &KagiRequest) -> Result<KagiResponse> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.post(&url).json(body);

        if let Some((name, value)) = &self.auth_header {
            request = request.header(name, value);
        }

        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                AppError::Timeout
            } else if e.is_connect() {
                AppError::UpstreamError("Failed to connect to Kagi".to_string())
            } else {
                AppError::UpstreamError(format!("Request failed: {}", e))
            }
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            if status.as_u16() == 401 {
                return Err(AppError::AuthFailed);
            }
            return Err(AppError::UpstreamError(format!(
                "Kagi returned {}: {}",
                status, body
            )));
        }

        let kagi_resp: KagiResponse = response.json().await.map_err(|e| {
            AppError::UpstreamError(format!("Failed to parse Kagi response: {}", e))
        })?;

        Ok(kagi_resp)
    }

    async fn stream_post(&self, path: &str, body: &KagiRequest) -> Result<mpsc::Receiver<std::result::Result<Event, Infallible>>> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.post(&url).json(body);

        if let Some((name, value)) = &self.auth_header {
            request = request.header(name, value);
        }

        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                AppError::Timeout
            } else if e.is_connect() {
                AppError::UpstreamError("Failed to connect to Kagi".to_string())
            } else {
                AppError::UpstreamError(format!("Request failed: {}", e))
            }
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            if status.as_u16() == 401 {
                return Err(AppError::AuthFailed);
            }
            return Err(AppError::UpstreamError(format!(
                "Kagi returned {}: {}",
                status, body
            )));
        }

        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            let mut stream = response.bytes_stream();

            let mut buffer = String::new();
            let mut in_content = false;

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                            for c in text.chars() {
                                if c == '{' {
                                    in_content = true;
                                    buffer.clear();
                                }
                                if in_content {
                                    buffer.push(c);
                                    if c == '}' {
                                        in_content = false;
                                        if let Ok(kagi_resp) = serde_json::from_str::<KagiResponse>(&buffer) {
                                            let chunk = kagi_resp.to_openai_chunk();
                                            if let Ok(event) = serde_json::to_string(&chunk) {
                                                let _ = tx.send(Ok(Event::default().data(event))).await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Ok(Event::default().data(format!("[ERROR] {}", e)))).await;
                    }
                }
            }

            let done = ChatCompletionChunk::done("gpt-4o".to_string());
            if let Ok(event) = serde_json::to_string(&done) {
                let _ = tx.send(Ok(Event::default().data(event))).await;
            }
        });

        Ok(rx)
    }
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