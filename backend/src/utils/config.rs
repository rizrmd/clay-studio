use anyhow::{Result, Context};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub server_address: String,
    #[allow(dead_code)]
    pub jwt_secret: String,
    pub datasource_pool_warmup: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Check if we're in production mode
        let is_production = env::var("RUST_ENV")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            == "production";

        let database_url = env::var("DATABASE_URL")
            .with_context(|| "DATABASE_URL environment variable must be set")?;

        let server_address = env::var("SERVER_ADDRESS").unwrap_or_else(|_| {
            if is_production {
                "0.0.0.0:7680".to_string()
            } else {
                "127.0.0.1:7680".to_string()
            }
        });

        let jwt_secret = env::var("JWT_SECRET")
            .unwrap_or_else(|_| "development-secret-key".to_string());

        let datasource_pool_warmup = env::var("DATASOURCE_POOL_WARMUP")
            .unwrap_or_else(|_| "true".to_string())
            .to_lowercase()
            == "true";

        Ok(Config {
            database_url,
            server_address,
            jwt_secret,
            datasource_pool_warmup,
        })
    }
}
