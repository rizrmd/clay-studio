use std::sync::Arc;
use std::collections::HashMap;
use sea_orm::DatabaseConnection;
use sqlx::PgPool;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::utils::Config;
use crate::utils::db;
use crate::models::client::Client;



#[derive(Clone, Debug)]
pub struct StreamingState {
    pub message_id: String,
    pub partial_content: String,
    pub last_updated: DateTime<Utc>,
    pub active_tools: Vec<String>,
}

#[derive(Clone)]
pub struct AppState {
    #[allow(dead_code)]
    pub db: Arc<DatabaseConnection>,
    pub db_pool: PgPool,
    #[allow(dead_code)]
    pub config: Arc<Config>,
    #[allow(dead_code)]
    pub clients: Arc<RwLock<HashMap<Uuid, Client>>>,
    pub active_claude_streams: Arc<RwLock<HashMap<String, StreamingState>>>,
}

impl AppState {
    pub async fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let db = db::connect(&config.database_url).await?;
        let db_pool = PgPool::connect(&config.database_url).await?;
        
        Ok(AppState {
            db: Arc::new(db),
            db_pool,
            config: Arc::new(config.clone()),
            clients: Arc::new(RwLock::new(HashMap::new())),
            active_claude_streams: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
}