use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Settings {
    pub database_url: String,
    pub host: String,
    pub port: u16,
}

impl Settings {
    pub fn from_env() -> Result<Self> {
        Ok(Settings {
            database_url: std::env::var("DATABASE_URL")
                .context("DATABASE_URL environment variable must be set")?,
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
        })
    }
}
