use kagi_proxy::{config::Config, create_app};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;

    let app = create_app(config.clone());

    let listener = tokio::net::TcpListener::bind(config.address()).await?;

    tracing::info!("Starting server on {}", config.address());

    axum::serve(listener, app).await?;

    Ok(())
}