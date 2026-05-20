use std::net::SocketAddr;
use std::str::FromStr;

use dotenvy::dotenv;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub kagi_base_url: String,
    pub kagi_session_token: Option<String>,
    pub kagi_auth_header: Option<String>,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenv().ok();

        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .map_err(|_| ConfigError::InvalidPort)?;

        let kagi_base_url = std::env::var("KAGI_BASE_URL")
            .unwrap_or_else(|_| "https://kagi.com".to_string());

        let kagi_session_token = std::env::var("KAGI_SESSION_TOKEN").ok();
        let kagi_auth_header = std::env::var("KAGI_AUTH_HEADER").ok();

        let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

        Ok(Self {
            port,
            kagi_base_url,
            kagi_session_token,
            kagi_auth_header,
            log_level,
        })
    }

    pub fn address(&self) -> SocketAddr {
        SocketAddr::from_str(&format!("0.0.0.0:{}", self.port)).unwrap()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid port number")]
    InvalidPort,
}