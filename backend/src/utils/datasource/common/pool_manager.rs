use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Generic pool manager for database connections
#[allow(dead_code)]
pub struct PoolManager<T> {
    pool: Arc<Mutex<Option<PoolWithTimestamp<T>>>>,
    connection_string: String,
    max_lifetime: Duration,
}

#[allow(dead_code)]
struct PoolWithTimestamp<T> {
    pool: T,
    created_at: Instant,
}

#[allow(dead_code)]
impl<T> PoolManager<T> 
where
    T: Clone + Send + Sync + 'static,
{
    pub fn new(connection_string: String, max_lifetime: Duration) -> Self {
        Self {
            pool: Arc::new(Mutex::new(None)),
            connection_string,
            max_lifetime,
        }
    }

    /// Get a pool, creating a new one if needed or if the existing one is stale
    pub async fn get_pool<F, Fut, E>(&self, create_pool: F) -> Result<T, E>
    where
        F: FnOnce(String) -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut pool_guard = self.pool.lock().await;

        // Check if we have a valid, non-expired pool
        if let Some(ref pool_with_ts) = *pool_guard {
            if pool_with_ts.created_at.elapsed() < self.max_lifetime {
                // Pool is still valid and not expired
                if self.test_pool_health(&pool_with_ts.pool).await {
                    debug!("Reusing existing database pool");
                    return Ok(pool_with_ts.pool.clone());
                } else {
                    warn!("Existing database pool failed health check, creating new pool");
                }
            } else {
                debug!("Database pool expired (age: {:?}), creating new pool", pool_with_ts.created_at.elapsed());
            }
        }

        // Create new pool
        debug!("Creating new database pool");
        let new_pool = create_pool(self.connection_string.clone()).await?;
        
        *pool_guard = Some(PoolWithTimestamp {
            pool: new_pool.clone(),
            created_at: Instant::now(),
        });

        Ok(new_pool)
    }

    /// Test if the pool is still healthy (should be implemented per database type)
    async fn test_pool_health(&self, _pool: &T) -> bool {
        // Default implementation - override in specific implementations
        true
    }

    /// Force refresh the pool
    pub async fn refresh_pool<F, Fut, E>(&self, create_pool: F) -> Result<T, E>
    where
        F: FnOnce(String) -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut pool_guard = self.pool.lock().await;
        
        debug!("Force refreshing database pool");
        let new_pool = create_pool(self.connection_string.clone()).await?;
        
        *pool_guard = Some(PoolWithTimestamp {
            pool: new_pool.clone(),
            created_at: Instant::now(),
        });

        Ok(new_pool)
    }

    /// Clear the pool
    pub async fn clear_pool(&self) {
        let mut pool_guard = self.pool.lock().await;
        *pool_guard = None;
        debug!("Database pool cleared");
    }
}

/// Common pool options for database connections
#[allow(dead_code)]
pub struct PoolOptions {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Option<Duration>,
}

impl Default for PoolOptions {
    fn default() -> Self {
        Self {
            max_connections: 5,
            min_connections: 1,
            acquire_timeout: Duration::from_secs(3),
            idle_timeout: Some(Duration::from_secs(30)),
        }
    }
}