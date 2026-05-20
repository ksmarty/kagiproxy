use axum::{extract::State, Json};
use std::sync::Arc;

use crate::openai::ModelList;
use crate::routes::chat::AppState;

pub async fn list_models(State(_state): State<Arc<AppState>>) -> Json<ModelList> {
    Json(ModelList::default_models())
}