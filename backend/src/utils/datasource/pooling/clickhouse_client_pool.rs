//! ClickHouse Client Pool Manager
//! 
//! ClickHouse uses HTTP connections, so we pool the ClickHouse client instances
//! which internally handle HTTP connection pooling through the underlying HTTP client

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;
use tracing::{info, warn, debug};
use clickhouse::Client;

/// ClickHouse client pool entry
#[derive(Clone)]
pub struct ClickHousePoolEntry {
    pub client: Arc<Client>,
    #[allow(dead_code)]
    pub created_at: std::time::Instant,
    pub last_used: std::time::Instant,
    pub usage_count: u64,
}

/// Manager for ClickHouse client pooling
pub struct ClickHouseClientPool {
    clients: Arc<RwLock<HashMap<String, ClickHousePoolEntry>>>,
}

impl Default for ClickHouseClientPool {
    fn default() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl ClickHouseClientPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a ClickHouse client for the given datasource
    pub async fn get_client(&self, datasource_id: &str, config: &Value) -> Result<Arc<Client>, String> {
        let cache_key = self.generate_cache_key(datasource_id, config);
        
        // First try to get existing client (read lock)
        {
            let clients = self.clients.read().await;
            if let Some(entry) = clients.get(&cache_key) {
                // Test the client to make sure it's still valid
                let client_clone = entry.client.clone();
                if self.validate_client(&client_clone).await {
                    // Update usage stats without upgrading to write lock
                    drop(clients);
                    self.update_usage_stats(&cache_key).await;
                    debug!("Using cached ClickHouse client for datasource {}", datasource_id);
                    return Ok(client_clone);
                } else {
                    warn!("Cached ClickHouse client for datasource {} is invalid, will recreate", datasource_id);
                }
            }
        }
        
        // Need to create new client (write lock)
        let mut clients = self.clients.write().await;
        
        // Double-check pattern - another thread might have created it
        if let Some(entry) = clients.get(&cache_key) {
            let client_clone = entry.client.clone();
            if self.validate_client(&client_clone).await {
                // Update stats and return
                drop(clients);
                self.update_usage_stats(&cache_key).await;
                debug!("ClickHouse client was created by another thread for datasource {}", datasource_id);
                return Ok(client_clone);
            }
        }
        
        // Create new ClickHouse client
        info!("Creating new ClickHouse client for datasource {}", datasource_id);
        let client = self.create_clickhouse_client(config)?;
        
        let entry = ClickHousePoolEntry {
            client: Arc::new(client),
            created_at: std::time::Instant::now(),
            last_used: std::time::Instant::now(),
            usage_count: 1,
        };
        
        // Cache the client
        clients.insert(cache_key, entry.clone());
        info!("Cached new ClickHouse client for datasource {} (total clients: {})", datasource_id, clients.len());
        
        Ok(entry.client)
    }

    /// Create a new ClickHouse client from configuration
    fn create_clickhouse_client(&self, config: &Value) -> Result<Client, String> {
        let url = if let Some(url) = config.get("url").and_then(|v| v.as_str()) {
            url.to_string()
        } else {
            // Construct URL from individual components
            let host = config
                .get("host")
                .and_then(|v| v.as_str())
                .unwrap_or("localhost");
            let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(8123);
            let database = config
                .get("database")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            let username = config
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            let password = config
                .get("password")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if password.is_empty() {
                format!("http://{}@{}:{}/{}", username, host, port, database)
            } else {
                format!(
                    "http://{}:{}@{}:{}/{}",
                    username, password, host, port, database
                )
            }
        };

        debug!("Creating ClickHouse client with URL: {}", Self::mask_url(&url));
        
        // Create client with optimized HTTP client settings for connection pooling
        let client = Client::default()
            .with_url(url);
            // TODO: Add custom HTTP client with better pooling settings
            // .with_http_client(custom_http_client)
        
        Ok(client)
    }

    /// Validate that a client is still functional
    async fn validate_client(&self, client: &Client) -> bool {
        match tokio::time::timeout(
            std::time::Duration::from_secs(3),
            client.query("SELECT 1").fetch_one::<u8>(),
        ).await {
            Ok(Ok(_)) => true,
            Ok(Err(_)) => false,
            Err(_) => false, // Timeout
        }
    }

    /// Update usage statistics for a client
    async fn update_usage_stats(&self, cache_key: &str) {
        let mut clients = self.clients.write().await;
        if let Some(entry) = clients.get_mut(cache_key) {
            entry.last_used = std::time::Instant::now();
            entry.usage_count += 1;
        }
    }

    /// Remove a client from cache (useful when datasource config changes)
    #[allow(dead_code)]
    pub async fn remove_client(&self, datasource_id: &str, config: &Value) {
        let cache_key = self.generate_cache_key(datasource_id, config);
        let mut clients = self.clients.write().await;
        
        if clients.remove(&cache_key).is_some() {
            info!("Removed cached ClickHouse client for datasource {}", datasource_id);
        }
    }

    /// Clear all cached clients
    #[allow(dead_code)]
    pub async fn clear_all(&self) {
        let mut clients = self.clients.write().await;
        let count = clients.len();
        clients.clear();
        info!("Cleared all {} cached ClickHouse clients", count);
    }

    /// Cleanup stale clients that haven't been used for a while
    #[allow(dead_code)]
    pub async fn cleanup_stale_clients(&self, max_idle_time: std::time::Duration) -> usize {
        let mut clients = self.clients.write().await;
        let now = std::time::Instant::now();
        let mut removed_count = 0;
        
        let stale_keys: Vec<String> = clients.iter()
            .filter(|(_, entry)| now.duration_since(entry.last_used) > max_idle_time)
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in stale_keys {
            clients.remove(&key);
            removed_count += 1;
            debug!("Removed stale ClickHouse client for key: {}", key);
        }
        
        if removed_count > 0 {
            info!("Cleaned up {} stale ClickHouse clients", removed_count);
        }
        
        removed_count
    }

    /// Generate a cache key based on datasource ID and config
    fn generate_cache_key(&self, datasource_id: &str, config: &Value) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        datasource_id.hash(&mut hasher);
        config.to_string().hash(&mut hasher);
        let hash = hasher.finish();
        
        format!("ch_{}_{:x}", datasource_id, hash)
    }

    /// Mask sensitive information in URLs for logging
    fn mask_url(url: &str) -> String {
        if url.contains('@') {
            let parts: Vec<&str> = url.splitn(2, "://").collect();
            if parts.len() == 2 {
                let auth_and_rest: Vec<&str> = parts[1].splitn(2, '@').collect();
                if auth_and_rest.len() == 2 {
                    let auth_parts: Vec<&str> = auth_and_rest[0].splitn(2, ':').collect();
                    if auth_parts.len() == 2 {
                        format!("{}://{}:***@{}", parts[0], auth_parts[0], auth_and_rest[1])
                    } else {
                        format!("{}://{}@{}", parts[0], auth_parts[0], auth_and_rest[1])
                    }
                } else {
                    url.to_string()
                }
            } else {
                url.to_string()
            }
        } else {
            url.to_string()
        }
    }

    /// Get statistics about cached clients
    #[allow(dead_code)]
    pub async fn get_stats(&self) -> serde_json::Value {
        let clients = self.clients.read().await;
        let total_clients = clients.len();
        let total_usage: u64 = clients.values().map(|entry| entry.usage_count).sum();
        let oldest_client_age = clients.values()
            .map(|entry| entry.created_at.elapsed().as_secs())
            .min()
            .unwrap_or(0);

        serde_json::json!({
            "total_clients": total_clients,
            "total_usage_count": total_usage,
            "oldest_client_age_seconds": oldest_client_age
        })
    }
}

/// Global singleton instance
static CLICKHOUSE_CLIENT_POOL: tokio::sync::OnceCell<ClickHouseClientPool> = tokio::sync::OnceCell::const_new();

/// Get the global ClickHouse client pool
pub async fn get_clickhouse_client_pool() -> &'static ClickHouseClientPool {
    CLICKHOUSE_CLIENT_POOL.get_or_init(|| async {
        info!("Initializing global ClickHouse client pool");
        ClickHouseClientPool::new()
    }).await
}