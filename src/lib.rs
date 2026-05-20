pub mod config;
pub mod error;
pub mod health;
pub mod kagi;
pub mod openai;
pub mod routes;

use std::sync::{Arc, Mutex};

use axum::{routing::get, Router};
use config::Config;
use kagi::KagiClient;
use routes::chat::AppState;

pub async fn create_app(config: Config) -> Router {
    let kagi_client = Arc::new(KagiClient::new(
        config.kagi_base_url,
        config.kagi_session_token,
        config.kagi_auth_header,
    ));

    let kagi_models = kagi_client
        .fetch_models()
        .await
        .unwrap_or_default();

    let state = Arc::new(AppState {
        kagi_client,
        kagi_models: Mutex::new(kagi_models),
    });

    Router::new()
        .route("/health", get(health::health))
        .route("/v1/models", get(routes::models::list_models))
        .route("/v1/chat/completions", axum::routing::post(routes::chat::chat_completions))
        .with_state(state)
}
