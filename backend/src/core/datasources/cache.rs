use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct CachedDatasource {
    pub id: String,
    pub datasource_type: String,
    pub connection_config: Value,
    pub user_id: Uuid,
    pub cached_at: Instant,
}

#[derive(Debug, Default)]
pub struct DatasourceCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

pub struct DatasourceCache {
    cache: Arc<RwLock<HashMap<String, CachedDatasource>>>,
    stats: Arc<RwLock<DatasourceCacheStats>>,
    ttl: Duration,
}

impl DatasourceCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(DatasourceCacheStats::default())),
            ttl: Duration::from_secs(300), // 5 minutes TTL
        }
    }

    pub async fn get(&self, datasource_id: &str, user_id: &str) -> Option<CachedDatasource> {
        let cache_key = format!("{}:{}", datasource_id, user_id);
        let cache = self.cache.read().await;
        
        if let Some(cached) = cache.get(&cache_key) {
            // Check if still valid
            if cached.cached_at.elapsed() < self.ttl {
                let mut stats = self.stats.write().await;
                stats.hits += 1;
                return Some(cached.clone());
            }
        }
        
        let mut stats = self.stats.write().await;
        stats.misses += 1;
        None
    }

    pub async fn set(&self, datasource: CachedDatasource) {
        let cache_key = format!("{}:{}", datasource.id, datasource.user_id.to_string());
        let mut cache = self.cache.write().await;
        cache.insert(cache_key, datasource);
    }

    pub async fn invalidate(&self, datasource_id: &str, user_id: Option<&str>) {
        let mut cache = self.cache.write().await;
        
        if let Some(uid) = user_id {
            // Invalidate specific user's cache for this datasource
            let cache_key = format!("{}:{}", datasource_id, uid);
            cache.remove(&cache_key);
        } else {
            // Invalidate all cached entries for this datasource across all users
            cache.retain(|key, _| !key.starts_with(&format!("{}:", datasource_id)));
        }
        
        let mut stats = self.stats.write().await;
        stats.evictions += 1;
    }

}

// Global cache instance
static DATASOURCE_CACHE: tokio::sync::OnceCell<DatasourceCache> = tokio::sync::OnceCell::const_new();

pub async fn get_datasource_cache() -> &'static DatasourceCache {
    DATASOURCE_CACHE.get_or_init(|| async {
        info!("🏗️ Initializing datasource cache");
        DatasourceCache::new()
    }).await
}