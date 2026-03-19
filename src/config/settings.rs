use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Settings {
    pub database_url: String,
    pub jwt_secret: String,
    pub host: String,
    pub port: u16,
}

impl Settings {
    pub fn from_env() -> Result<Self> {
        Ok(Settings {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://axis_user:axis_password@localhost:5432/axis_core".to_string()),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "default_secret_change_me".to_string()),
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
        })
    }
}
