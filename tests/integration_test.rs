use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use kagi_proxy::{
    config::Config,
    create_app,
    openai::ModelList,
};
use tower::ServiceExt;
use serde_json::json;

#[tokio::test]
async fn test_health() {
    let config = Config {
        port: 3000,
        kagi_base_url: "http://localhost".to_string(),
        kagi_session_token: None,
        kagi_auth_header: None,
        log_level: "info".to_string(),
    };
    let app = create_app(config).await;
    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(&body[..], b"OK");
}

#[tokio::test]
async fn test_list_models() {
    let config = Config {
        port: 3000,
        kagi_base_url: "http://localhost".to_string(),
        kagi_session_token: None,
        kagi_auth_header: None,
        log_level: "info".to_string(),
    };
    let app = create_app(config).await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    let model_list: ModelList = serde_json::from_slice(&body).unwrap();
    assert_eq!(model_list.object, "list");
}

#[tokio::test]
async fn test_chat_completions_missing_messages() {
    let config = Config {
        port: 3000,
        kagi_base_url: "http://localhost".to_string(),
        kagi_session_token: None,
        kagi_auth_header: None,
        log_level: "info".to_string(),
    };
    let app = create_app(config).await;
    let request_body = json!({
        "model": "gpt-4o",
        "messages": []
    });
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/chat/completions")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    let error: kagi_proxy::openai::ErrorResponse = serde_json::from_slice(&body).unwrap();
    assert!(error.error.message.contains("messages"));
}

#[tokio::test]
async fn test_chat_completions_empty_content() {
    let config = Config {
        port: 3000,
        kagi_base_url: "http://localhost".to_string(),
        kagi_session_token: None,
        kagi_auth_header: None,
        log_level: "info".to_string(),
    };
    let app = create_app(config).await;
    let request_body = json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": ""}
        ]
    });
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/chat/completions")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    let error: kagi_proxy::openai::ErrorResponse = serde_json::from_slice(&body).unwrap();
    assert!(error.error.message.contains("empty"));
}
