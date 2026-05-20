use axum::{extract::State, Json};
use std::sync::Arc;

use crate::openai::ModelList;
use crate::routes::chat::AppState;

pub async fn list_models(State(state): State<Arc<AppState>>) -> Json<ModelList> {
    let models = state.kagi_models.lock().unwrap();
    Json(ModelList::from_kagi_models(&models))
}
