use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;
use tracing::{info, warn, debug};
use sqlx::{Pool, postgres::Postgres, mysql::MySql, sqlite::Sqlite};
use sqlx::Row;
use super::super::connectors::postgres::PostgreSQLConnector;
use super::super::connectors::mysql::MySQLConnector;
use super::super::connectors::sqlite::SQLiteConnector;

/// Enum representing different types of SQLx database pools
#[derive(Debug, Clone)]
pub enum DatabasePool {
    PostgreSQL(Arc<Pool<Postgres>>),
    MySQL(Arc<Pool<MySql>>),
    SQLite(Arc<Pool<Sqlite>>),
}

/// Global connection pool manager that caches SQLx connection pools
/// to avoid recreating them on every request
pub struct ConnectionPoolManager {
    pools: Arc<RwLock<HashMap<String, DatabasePool>>>,
    pool_stats: Arc<RwLock<HashMap<String, PoolStats>>>,
}

/// Statistics for each pool
#[derive(Debug, Clone)]
pub struct PoolStats {
    #[allow(dead_code)]
    pub created_at: std::time::Instant,
    pub last_used: std::time::Instant,
    pub usage_count: u64,
    pub last_validation_failure: Option<std::time::Instant>,
    pub consecutive_failures: u32,
}

impl Default for PoolStats {
    fn default() -> Self {
        Self {
            created_at: std::time::Instant::now(),
            last_used: std::time::Instant::now(),
            usage_count: 0,
            last_validation_failure: None,
            consecutive_failures: 0,
        }
    }
}

impl Default for ConnectionPoolManager {
    fn default() -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            pool_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl ConnectionPoolManager {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get or create a connection pool for the given datasource and type
    /// Uses datasource_id + config hash as cache key
    pub async fn get_pool(&self, datasource_id: &str, source_type: &str, config: &Value) -> Result<DatabasePool, String> {
        let cache_key = self.generate_cache_key(datasource_id, config);
        
        // First try to get existing pool (read lock)
        {
            let pools = self.pools.read().await;
            if let Some(pool) = pools.get(&cache_key) {
                // Check if we should retry validation
                if self.should_retry_pool(&cache_key).await {
                    // Validate the pool based on its type
                    if self.validate_pool(pool).await {
                        self.update_usage_stats(&cache_key).await;
                        self.record_validation_success(&cache_key).await;
                        info!("✅ Cache HIT: Using cached connection pool for {} datasource {} (cache key: {})", source_type, datasource_id, cache_key);
                        return Ok(pool.clone());
                    } else {
                        self.record_validation_failure(&cache_key).await;
                        warn!("Cached pool for {} datasource {} is invalid, will recreate", source_type, datasource_id);
                    }
                } else {
                    // Skip validation for now, use the existing pool
                    self.update_usage_stats(&cache_key).await;
                    info!("✅ Cache HIT: Using cached connection pool for {} datasource {} (validation skipped - recent failure, cache key: {})", source_type, datasource_id, cache_key);
                    return Ok(pool.clone());
                }
            }
        }
        
        // Need to create new pool (write lock)
        let mut pools = self.pools.write().await;
        
        // Double-check pattern - another thread might have created it
        if let Some(pool) = pools.get(&cache_key) {
            if self.validate_pool(pool).await {
                self.update_usage_stats(&cache_key).await;
                self.record_validation_success(&cache_key).await;
                info!("✅ Cache HIT: Pool was created by another thread for {} datasource {} (cache key: {})", source_type, datasource_id, cache_key);
                return Ok(pool.clone());
            } else {
                self.record_validation_failure(&cache_key).await;
            }
        }
        
        // Create new pool based on database type
        info!("🔧 Creating new connection pool for {} datasource {} (cache key: {})", source_type, datasource_id, cache_key);
        
        // Add datasource_id to config if not present (needed by connectors)
        let mut enriched_config = config.clone();
        if let Some(obj) = enriched_config.as_object_mut() {
            obj.insert("id".to_string(), serde_json::Value::String(datasource_id.to_string()));
        }
        
        let pool = match source_type.to_lowercase().as_str() {
            "postgresql" | "postgres" => {
                let connector = PostgreSQLConnector::new(&enriched_config)
                    .map_err(|e| format!("Failed to create PostgreSQL connector: {}", e))?;
                let pg_pool = connector.create_pool().await
                    .map_err(|e| format!("Failed to create PostgreSQL pool: {}", e))?;
                DatabasePool::PostgreSQL(Arc::new(pg_pool))
            },
            "mysql" => {
                let connector = MySQLConnector::new(&enriched_config)
                    .map_err(|e| format!("Failed to create MySQL connector: {}", e))?;
                let mysql_pool = connector.create_pool().await
                    .map_err(|e| format!("Failed to create MySQL pool: {}", e))?;
                DatabasePool::MySQL(Arc::new(mysql_pool))
            },
            "sqlite" => {
                let connector = SQLiteConnector::new(&enriched_config)
                    .map_err(|e| format!("Failed to create SQLite connector: {}", e))?;
                let sqlite_pool = connector.create_pool().await
                    .map_err(|e| format!("Failed to create SQLite pool: {}", e))?;
                DatabasePool::SQLite(Arc::new(sqlite_pool))
            },
            _ => {
                return Err(format!("Unsupported database type for connection pooling: {}. Only SQLx databases (PostgreSQL, MySQL, SQLite) support global pooling.", source_type));
            }
        };
        
        // Cache the pool and initialize stats
        pools.insert(cache_key.clone(), pool.clone());
        self.initialize_stats(&cache_key).await;
        info!("💾 Cache MISS: Created and cached new connection pool for {} datasource {} (total pools: {}, cache key: {})", source_type, datasource_id, pools.len(), cache_key);
        
        Ok(pool)
    }
    
    /// Validate a pool based on its type
    async fn validate_pool(&self, pool: &DatabasePool) -> bool {
        match pool {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("SELECT 1").fetch_one(pool.as_ref()).await.is_ok()
            },
            DatabasePool::MySQL(pool) => {
                sqlx::query("SELECT 1").fetch_one(pool.as_ref()).await.is_ok()
            },
            DatabasePool::SQLite(pool) => {
                sqlx::query("SELECT 1").fetch_one(pool.as_ref()).await.is_ok()
            },
        }
    }
    
    /// Check if we should retry pool validation or recreate
    async fn should_retry_pool(&self, cache_key: &str) -> bool {
        let stats = self.pool_stats.read().await;
        if let Some(stat) = stats.get(cache_key) {
            // If we have too many consecutive failures, recreate the pool
            if stat.consecutive_failures >= 3 {
                return true;
            }
            
            // If the last failure was recent (within 5 seconds), retry
            if let Some(last_failure) = stat.last_validation_failure {
                if last_failure.elapsed() < std::time::Duration::from_secs(5) {
                    return false;
                }
            }
        }
        true
    }
    
    /// Record a validation failure
    async fn record_validation_failure(&self, cache_key: &str) {
        let mut stats = self.pool_stats.write().await;
        if let Some(stat) = stats.get_mut(cache_key) {
            stat.last_validation_failure = Some(std::time::Instant::now());
            stat.consecutive_failures += 1;
        }
    }
    
    /// Record a validation success
    async fn record_validation_success(&self, cache_key: &str) {
        let mut stats = self.pool_stats.write().await;
        if let Some(stat) = stats.get_mut(cache_key) {
            stat.last_validation_failure = None;
            stat.consecutive_failures = 0;
        }
    }
    
    /// Update usage statistics for a pool
    async fn update_usage_stats(&self, cache_key: &str) {
        let mut stats = self.pool_stats.write().await;
        if let Some(stat) = stats.get_mut(cache_key) {
            stat.last_used = std::time::Instant::now();
            stat.usage_count += 1;
        }
    }
    
    /// Initialize statistics for a new pool
    async fn initialize_stats(&self, cache_key: &str) {
        let mut stats = self.pool_stats.write().await;
        stats.insert(cache_key.to_string(), PoolStats {
            created_at: std::time::Instant::now(),
            last_used: std::time::Instant::now(),
            usage_count: 1,
            last_validation_failure: None,
            consecutive_failures: 0,
        });
    }
    
    /// Remove a pool from cache (useful when datasource config changes)
    #[allow(dead_code)]
    pub async fn remove_pool(&self, datasource_id: &str, config: &Value) {
        let cache_key = self.generate_cache_key(datasource_id, config);
        let mut pools = self.pools.write().await;
        let mut stats = self.pool_stats.write().await;
        
        if pools.remove(&cache_key).is_some() {
            stats.remove(&cache_key);
            info!("Removed cached pool for datasource {}", datasource_id);
        }
    }
    
    /// Clear all cached pools (useful for testing or config changes)
    #[allow(dead_code)]
    pub async fn clear_all(&self) {
        let mut pools = self.pools.write().await;
        let mut stats = self.pool_stats.write().await;
        let count = pools.len();
        
        pools.clear();
        stats.clear();
        info!("Cleared all {} cached connection pools", count);
    }
    
    /// Cleanup stale pools that haven't been used for a while
    #[allow(dead_code)]
    pub async fn cleanup_stale_pools(&self, max_idle_time: std::time::Duration) -> usize {
        let mut pools = self.pools.write().await;
        let mut stats = self.pool_stats.write().await;
        let now = std::time::Instant::now();
        let mut removed_count = 0;
        
        let stale_keys: Vec<String> = stats.iter()
            .filter(|(_, stat)| now.duration_since(stat.last_used) > max_idle_time)
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in stale_keys {
            pools.remove(&key);
            stats.remove(&key);
            removed_count += 1;
            debug!("Removed stale pool for key: {}", key);
        }
        
        if removed_count > 0 {
            info!("Cleaned up {} stale connection pools", removed_count);
        }
        
        removed_count
    }
      /// Warm up connection pools for all active datasources
    /// This should be called on application startup to avoid slow first requests
    pub async fn warm_up_pools(&self, app_db_pool: &sqlx::PgPool) -> Result<usize, String> {
        info!("🔥 Starting connection pool warm-up...");
        
        // Get all active datasources from the application database
        let datasources = sqlx::query(
            "SELECT id, connection_config, source_type 
             FROM data_sources 
             WHERE deleted_at IS NULL"
        )
        .fetch_all(app_db_pool)
        .await
        .map_err(|e| format!("Failed to fetch datasources: {}", e))?;

        let mut success_count = 0;
        let mut error_count = 0;
        let mut skipped_count = 0;

        for datasource in datasources {
            let datasource_id: String = datasource.try_get("id")
                .map_err(|e| format!("Failed to get datasource ID: {}", e))?;
            let source_type: String = datasource.try_get("source_type")
                .map_err(|e| format!("Failed to get source type: {}", e))?;
            let config: serde_json::Value = datasource.try_get("connection_config")
                .map_err(|e| format!("Failed to get config: {}", e))?;
            
            // Only warm up pools for SQLx databases
            let db_type = source_type.to_lowercase();
            if db_type.contains("postgresql") || db_type.contains("postgres") || 
               db_type.contains("mysql") || db_type.contains("sqlite") {
                // Attempt to warm up the pool
                match self.get_pool(&datasource_id, &source_type, &config).await {
                    Ok(_) => {
                        success_count += 1;
                        info!("✅ Warmed up pool for {} datasource {}", source_type, datasource_id);
                    }
                    Err(e) => {
                        error_count += 1;
                        warn!("❌ Failed to warm up pool for {} datasource {}: {}", source_type, datasource_id, e);
                    }
                }
            } else {
                skipped_count += 1;
                debug!("⏭ Skipping warm-up for {} datasource {} (not a SQLx database)", source_type, datasource_id);
            }
        }

        info!("🔥 Pool warm-up complete: {} successful, {} failed, {} skipped (non-SQLx databases)", 
              success_count, error_count, skipped_count);
        Ok(success_count)
    }

    /// Warm up connection pools for datasources belonging to a specific project
    /// This should be called when a project is accessed to avoid slow first requests
    pub async fn warm_up_project_pools(&self, app_db_pool: &sqlx::PgPool, project_id: &str) -> Result<usize, String> {
        info!("🔥 Starting connection pool warm-up for project {}...", project_id);
        
        // Get datasources for the specific project
        let datasources = sqlx::query(
            "SELECT id, connection_config, source_type 
             FROM data_sources 
             WHERE deleted_at IS NULL 
             AND project_id = $1"
        )
        .bind(project_id)
        .fetch_all(app_db_pool)
        .await
        .map_err(|e| format!("Failed to fetch project datasources: {}", e))?;

        let mut success_count = 0;
        let mut error_count = 0;
        let mut skipped_count = 0;

        for datasource in datasources {
            let datasource_id: String = datasource.try_get("id")
                .map_err(|e| format!("Failed to get datasource ID: {}", e))?;
            let source_type: String = datasource.try_get("source_type")
                .map_err(|e| format!("Failed to get source type: {}", e))?;
            let config: serde_json::Value = datasource.try_get("connection_config")
                .map_err(|e| format!("Failed to get config: {}", e))?;
            
            // Only warm up pools for SQLx databases
            let db_type = source_type.to_lowercase();
            if db_type.contains("postgresql") || db_type.contains("postgres") || 
               db_type.contains("mysql") || db_type.contains("sqlite") {
                // Attempt to warm up the pool
                match self.get_pool(&datasource_id, &source_type, &config).await {
                    Ok(_) => {
                        success_count += 1;
                        info!("✅ Warmed up pool for {} datasource {}", source_type, datasource_id);
                    }
                    Err(e) => {
                        error_count += 1;
                        warn!("❌ Failed to warm up pool for {} datasource {}: {}", source_type, datasource_id, e);
                    }
                }
            } else {
                skipped_count += 1;
                debug!("⏭ Skipping warm-up for {} datasource {} (not a SQLx database)", source_type, datasource_id);
            }
        }

        info!("🔥 Project {} pool warm-up complete: {} successful, {} failed, {} skipped (non-SQLx databases)", 
              project_id, success_count, error_count, skipped_count);
        Ok(success_count)
    }
    
    /// Generate a cache key based on datasource ID and config
    fn generate_cache_key(&self, datasource_id: &str, config: &Value) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        datasource_id.hash(&mut hasher);
        
        // Extract and hash only the essential connection parameters for stability
        if let Some(obj) = config.as_object() {
            // Common parameters
            if let Some(host) = obj.get("host").and_then(|v| v.as_str()) {
                host.hash(&mut hasher);
            }
            if let Some(port) = obj.get("port").and_then(|v| v.as_u64()) {
                port.hash(&mut hasher);
            }
            
            // Database name (can be in different fields)
            if let Some(database) = obj.get("database").and_then(|v| v.as_str()) {
                database.hash(&mut hasher);
            } else if let Some(schema) = obj.get("schema").and_then(|v| v.as_str()) {
                schema.hash(&mut hasher);
            }
            
            // Username
            if let Some(username) = obj.get("username").and_then(|v| v.as_str()) {
                username.hash(&mut hasher);
            }
            
            // SQLite path
            if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
                path.hash(&mut hasher);
            }
        }
        
        let hash = hasher.finish();
        
        format!("{}_{:x}", datasource_id, hash)
    }
    
    /// Get detailed stats about cached pools
    #[allow(dead_code)]
    pub async fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let pools = self.pools.read().await;
        let stats = self.pool_stats.read().await;
        let mut result = HashMap::new();
        
        let mut pool_counts = HashMap::new();
        let mut total_usage = 0;
        let mut oldest_pool = std::time::Instant::now();
        
        for (key, pool) in pools.iter() {
            match pool {
                DatabasePool::PostgreSQL(_) => *pool_counts.entry("postgresql".to_string()).or_insert(0) += 1,
                DatabasePool::MySQL(_) => *pool_counts.entry("mysql".to_string()).or_insert(0) += 1,
                DatabasePool::SQLite(_) => *pool_counts.entry("sqlite".to_string()).or_insert(0) += 1,
            }
            
            if let Some(stat) = stats.get(key) {
                total_usage += stat.usage_count;
                if stat.created_at < oldest_pool {
                    oldest_pool = stat.created_at;
                }
            }
        }
        
        result.insert("total_pools".to_string(), serde_json::Value::Number(serde_json::Number::from(pools.len())));
        result.insert("pool_counts".to_string(), serde_json::Value::Object(
            pool_counts.into_iter().map(|(k, v)| (k, serde_json::Value::Number(serde_json::Number::from(v)))).collect()
        ));
        result.insert("total_usage_count".to_string(), serde_json::Value::Number(serde_json::Number::from(total_usage)));
        result.insert("oldest_pool_age_seconds".to_string(), serde_json::Value::Number(serde_json::Number::from(
            oldest_pool.elapsed().as_secs()
        )));
        
        result
    }
    
    /// Get pool for execution (helper method to abstract away pool type)
    #[allow(dead_code)]
    pub async fn execute_with_pool<F, R>(&self, datasource_id: &str, source_type: &str, config: &Value, operation: F) -> Result<R, String>
    where
        F: FnOnce(&DatabasePool) -> Result<R, Box<dyn std::error::Error + Send + Sync>>,
    {
        let pool = self.get_pool(datasource_id, source_type, config).await?;
        operation(&pool).map_err(|e| e.to_string())
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

/// Warm up connection pools for a specific project
pub async fn warm_up_project_pools(app_db_pool: &sqlx::PgPool, project_id: &str) -> Result<usize, String> {
    let pool_manager = get_pool_manager().await;
    pool_manager.warm_up_project_pools(app_db_pool, project_id).await
}