use std::sync::Arc;
use std::collections::HashMap;
use sea_orm::DatabaseConnection;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::time::Duration;
use tracing::{info, warn, error};
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
        // Connect to database with SeaORM (handles its own connection pool)
        let db = db::connect(&config.database_url).await?;
        
        // Create SQLx connection pool with comprehensive logging
        info!("🔌 Creating SQLx PostgreSQL connection pool...");
        let db_pool = Self::create_sqlx_pool(&config.database_url).await?;
        
        Ok(AppState {
            db: Arc::new(db),
            db_pool,
            config: Arc::new(config.clone()),
            clients: Arc::new(RwLock::new(HashMap::new())),
            active_claude_streams: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    async fn create_sqlx_pool(database_url: &str) -> Result<PgPool, Box<dyn std::error::Error>> {
        info!("📊 SQLx Pool configuration:");
        info!("  - Max connections: 15");
        info!("  - Min connections: 2");
        info!("  - Connect timeout: 30s");
        info!("  - Idle timeout: 600s");
        info!("  - Max lifetime: 1800s");
        
        let pool_options = PgPoolOptions::new()
            .max_connections(15)
            .min_connections(2)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Some(Duration::from_secs(600)))
            .max_lifetime(Some(Duration::from_secs(1800)))
            .test_before_acquire(true);
        
        // Attempt connection with retry logic
        let mut attempts = 0;
        const MAX_ATTEMPTS: u8 = 3;
        
        let pool = loop {
            attempts += 1;
            info!("🔄 SQLx pool connection attempt {}/{}", attempts, MAX_ATTEMPTS);
            
            match pool_options.clone().connect(database_url).await {
                Ok(pool) => {
                    info!("✅ SQLx PostgreSQL pool created successfully");
                    break pool;
                }
                Err(e) => {
                    error!("❌ SQLx pool creation attempt {} failed: {}", attempts, e);
                    
                    // Log specific error diagnostics
                    let error_msg = e.to_string();
                    if error_msg.contains("Connection refused") {
                        error!("🚨 PostgreSQL server unreachable");
                    } else if error_msg.contains("authentication") || error_msg.contains("password") {
                        error!("🚨 PostgreSQL authentication failed");
                    } else if error_msg.contains("timeout") {
                        error!("🚨 PostgreSQL connection timeout");
                    }
                    
                    if attempts >= MAX_ATTEMPTS {
                        error!("💥 All SQLx pool creation attempts exhausted");
                        return Err(e.into());
                    }
                    
                    warn!("⏳ Retrying SQLx pool creation in 2 seconds...");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        };
        
        // Test the pool
        info!("🧪 Testing SQLx pool connection...");
        match sqlx::query("SELECT 1 as test").fetch_one(&pool).await {
            Ok(_) => {
                info!("✅ SQLx pool connection test successful");
                Self::log_pool_stats_static(&pool, "Initial").await;
            }
            Err(e) => {
                error!("❌ SQLx pool connection test failed: {}", e);
                return Err(e.into());
            }
        }
        
        Ok(pool)
    }
    
    /// Log connection pool statistics
    pub async fn log_pool_stats(&self, context: &str) {
        Self::log_pool_stats_static(&self.db_pool, context).await;
    }
    
    async fn log_pool_stats_static(pool: &PgPool, context: &str) {
        let stats = pool.size();
        let idle = pool.num_idle() as u32;
        info!("📈 [{}] Pool Stats - Total: {}, Idle: {}, Active: {}", 
              context, stats, idle, stats.saturating_sub(idle));
              
        // Warn if pool usage is high
        if stats > 10 {
            warn!("⚠️  High database connection usage: {}/15 connections active", stats);
        }
        
        if idle == 0 && stats > 5 {
            warn!("⚠️  No idle database connections available - potential bottleneck");
        }
    }
    
    /// Health check for database connections
    pub async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Test SeaORM connection  
        match sqlx::query("SELECT 1 as test").fetch_one(&self.db_pool).await {
            Ok(_) => info!("✅ SeaORM connection healthy"),
            Err(e) => {
                error!("❌ SeaORM connection unhealthy: {}", e);
                return Err(e.into());
            }
        }
        
        // Test SQLx connection
        match sqlx::query("SELECT 1 as test").fetch_one(&self.db_pool).await {
            Ok(_) => {
                info!("✅ SQLx pool healthy");
                self.log_pool_stats("Health Check").await;
            }
            Err(e) => {
                error!("❌ SQLx pool unhealthy: {}", e);
                return Err(e.into());
            }
        }
        
        Ok(())
    }
}