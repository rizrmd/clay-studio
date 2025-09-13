use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use sqlx::postgres::PgPool;
use sqlx::Row;
use serde_json::Value;
use tracing::{info, warn, debug};
use super::postgres::PostgreSQLConnector;

/// Global connection pool manager that caches PostgreSQL connection pools
/// to avoid recreating them on every request
pub struct ConnectionPoolManager {
    pools: Arc<RwLock<HashMap<String, Arc<PgPool>>>>,
}

impl ConnectionPoolManager {
    pub fn new() -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get or create a connection pool for the given datasource config
    /// Uses datasource_id + config hash as cache key
    pub async fn get_pool(&self, datasource_id: &str, config: &Value) -> Result<Arc<PgPool>, String> {
        let cache_key = self.generate_cache_key(datasource_id, config);
        
        // First try to get existing pool (read lock)
        {
            let pools = self.pools.read().await;
            if let Some(pool) = pools.get(&cache_key) {
                // Quick validation - if this fails, we'll recreate the pool
                if sqlx::query("SELECT 1").fetch_one(pool.as_ref()).await.is_ok() {
                    debug!("Using cached connection pool for datasource {}", datasource_id);
                    return Ok(Arc::clone(pool));
                } else {
                    warn!("Cached pool for datasource {} is invalid, will recreate", datasource_id);
                }
            }
        }
        
        // Need to create new pool (write lock)
        let mut pools = self.pools.write().await;
        
        // Double-check pattern - another thread might have created it
        if let Some(pool) = pools.get(&cache_key) {
            if sqlx::query("SELECT 1").fetch_one(pool.as_ref()).await.is_ok() {
                debug!("Pool was created by another thread for datasource {}", datasource_id);
                return Ok(Arc::clone(pool));
            }
        }
        
        // Create new pool
        info!("Creating new connection pool for datasource {}", datasource_id);
        let connector = PostgreSQLConnector::new(config)
            .map_err(|e| format!("Failed to create connector: {}", e))?;
        let pool = connector.create_pool().await
            .map_err(|e| format!("Failed to create pool: {}", e))?;
        let arc_pool = Arc::new(pool);
        
        // Cache the pool
        pools.insert(cache_key, Arc::clone(&arc_pool));
        info!("Cached new connection pool for datasource {} (total pools: {})", datasource_id, pools.len());
        
        Ok(arc_pool)
    }
    
    /// Remove a pool from cache (useful when datasource config changes)
    #[allow(dead_code)]
    pub async fn remove_pool(&self, datasource_id: &str, config: &Value) {
        let cache_key = self.generate_cache_key(datasource_id, config);
        let mut pools = self.pools.write().await;
        if pools.remove(&cache_key).is_some() {
            info!("Removed cached pool for datasource {}", datasource_id);
        }
    }
    
    /// Clear all cached pools (useful for testing or config changes)
    #[allow(dead_code)]
    pub async fn clear_all(&self) {
        let mut pools = self.pools.write().await;
        let count = pools.len();
        pools.clear();
        info!("Cleared all {} cached connection pools", count);
    }

    /// Warm up connection pools for all active datasources
    /// This should be called on application startup to avoid slow first requests
    pub async fn warm_up_pools(&self, app_db_pool: &sqlx::PgPool) -> Result<usize, String> {
        info!("🔥 Starting connection pool warm-up...");
        
        // Get all active datasources from the application database
        let datasources = sqlx::query(
            "SELECT id, connection_config, source_type 
             FROM data_sources 
             WHERE deleted_at IS NULL AND source_type = 'postgresql'"
        )
        .fetch_all(app_db_pool)
        .await
        .map_err(|e| format!("Failed to fetch datasources: {}", e))?;

        let mut success_count = 0;
        let mut error_count = 0;

        for datasource in datasources {
            let datasource_id: String = datasource.try_get("id")
                .map_err(|e| format!("Failed to get datasource ID: {}", e))?;
            let config: serde_json::Value = datasource.try_get("connection_config")
                .map_err(|e| format!("Failed to get config: {}", e))?;
            
            // Config is already a JSON value, no need to parse

            // Attempt to warm up the pool
            match self.get_pool(&datasource_id, &config).await {
                Ok(_) => {
                    success_count += 1;
                    info!("✅ Warmed up pool for datasource {}", datasource_id);
                }
                Err(e) => {
                    error_count += 1;
                    warn!("❌ Failed to warm up pool for datasource {}: {}", datasource_id, e);
                }
            }
        }

        info!("🔥 Pool warm-up complete: {} successful, {} failed", success_count, error_count);
        Ok(success_count)
    }
    
    /// Generate a cache key based on datasource ID and config
    fn generate_cache_key(&self, datasource_id: &str, config: &Value) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        datasource_id.hash(&mut hasher);
        config.to_string().hash(&mut hasher);
        let hash = hasher.finish();
        
        format!("{}_{:x}", datasource_id, hash)
    }
    
    /// Get stats about cached pools
    #[allow(dead_code)]
    pub async fn get_stats(&self) -> HashMap<String, usize> {
        let pools = self.pools.read().await;
        let mut stats = HashMap::new();
        stats.insert("total_pools".to_string(), pools.len());
        stats
    }
}

/// Global singleton instance
static POOL_MANAGER: tokio::sync::OnceCell<ConnectionPoolManager> = tokio::sync::OnceCell::const_new();

/// Get the global connection pool manager
pub async fn get_pool_manager() -> &'static ConnectionPoolManager {
    POOL_MANAGER.get_or_init(|| async {
        info!("Initializing global connection pool manager");
        ConnectionPoolManager::new()
    }).await
}