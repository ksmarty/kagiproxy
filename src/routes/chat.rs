use std::sync::{Arc, Mutex};

use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};

use crate::error::Result;
use crate::kagi::{KagiClient, KagiModel};
use crate::openai::ChatCompletionRequest;

pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatCompletionRequest>,
) -> Result<axum::response::Response> {
    if req.messages.is_empty() {
        return Err(crate::error::AppError::MissingParam("messages".to_string()));
    }

    for msg in &req.messages {
        if msg.content.is_empty() {
            return Err(crate::error::AppError::InvalidRequest("message content cannot be empty".to_string()));
        }
    }

    let stream = req.stream.unwrap_or(false);

    if stream {
        let sse = state.kagi_client.stream_chat(req).await?;
        Ok(sse.into_response())
    } else {
        let response = state.kagi_client.chat(req).await?;
        Ok(Json(response).into_response())
    }
}

pub struct AppState {
    pub kagi_client: Arc<KagiClient>,
    pub kagi_models: Mutex<Vec<KagiModel>>,
}
