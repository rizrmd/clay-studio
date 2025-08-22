use std::sync::Arc;
use sea_orm::DatabaseConnection;
use crate::config::Config;
use crate::db;

#[derive(Clone)]
pub struct AppState {
    #[allow(dead_code)]
    pub db: Arc<DatabaseConnection>,
    #[allow(dead_code)]
    pub config: Arc<Config>,
}

impl AppState {
    pub async fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let db = db::connect(&config.database_url).await?;
        
        Ok(AppState {
            db: Arc::new(db),
            config: Arc::new(config.clone()),
        })
    }
}