use async_session::Session;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use tracing::{debug, info};

/// Cached session entry with expiration tracking
#[derive(Clone, Debug)]
pub struct CachedSession {
    pub session: Session,
    pub expires_at: DateTime<Utc>,
    pub cached_at: DateTime<Utc>,
}

impl CachedSession {
    pub fn new(session: Session, expires_at: DateTime<Utc>) -> Self {
        Self {
            session,
            expires_at,
            cached_at: Utc::now(),
        }
    }

    /// Check if this cached session has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if cache entry is stale (older than 5 minutes)
    /// This ensures we periodically refresh from database
    pub fn is_stale(&self) -> bool {
        Utc::now() > self.cached_at + chrono::Duration::minutes(5)
    }
}

/// High-performance in-memory session cache to avoid database hits
#[derive(Clone, Debug)]
pub struct SessionCache {
    cache: Arc<RwLock<HashMap<String, CachedSession>>>,
    /// Statistics for monitoring performance
    stats: Arc<RwLock<CacheStats>>,
}

#[derive(Debug, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl CacheStats {
    #[allow(dead_code)]
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}

impl SessionCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Get a session from cache, returns None if not cached or expired
    pub async fn get(&self, session_id: &str) -> Option<Session> {
        let cache = self.cache.read().await;
        let mut stats = self.stats.write().await;
        
        if let Some(cached) = cache.get(session_id) {
            if cached.is_expired() {
                stats.misses += 1;
                debug!("Session {} found in cache but expired", session_id);
                None
            } else if cached.is_stale() {
                stats.misses += 1;
                debug!("Session {} found in cache but stale, will refresh", session_id);
                None
            } else {
                stats.hits += 1;
                debug!("Session {} cache hit", session_id);
                Some(cached.session.clone())
            }
        } else {
            stats.misses += 1;
            debug!("Session {} cache miss", session_id);
            None
        }
    }

    /// Cache a session with its expiration time
    pub async fn set(&self, session_id: String, session: Session, expires_at: DateTime<Utc>) {
        let mut cache = self.cache.write().await;
        let cached_session = CachedSession::new(session, expires_at);
        cache.insert(session_id.clone(), cached_session);
        debug!("Cached session {}", session_id);
        
        // Clean up expired entries periodically (10% chance)
        if rand::random::<f32>() < 0.1 {
            self.cleanup_expired_internal(&mut cache).await;
        }
    }

    /// Remove a session from cache
    pub async fn remove(&self, session_id: &str) {
        let mut cache = self.cache.write().await;
        if cache.remove(session_id).is_some() {
            debug!("Removed session {} from cache", session_id);
        }
    }

    /// Clean up expired sessions from cache
    pub async fn cleanup_expired(&self) -> usize {
        let mut cache = self.cache.write().await;
        self.cleanup_expired_internal(&mut cache).await
    }

    async fn cleanup_expired_internal(&self, cache: &mut HashMap<String, CachedSession>) -> usize {
        let initial_count = cache.len();
        cache.retain(|session_id, cached| {
            let keep = !cached.is_expired();
            if !keep {
                debug!("Evicting expired session {} from cache", session_id);
            }
            keep
        });
        
        let removed = initial_count - cache.len();
        if removed > 0 {
            let mut stats = self.stats.write().await;
            stats.evictions += removed as u64;
            info!("Evicted {} expired sessions from cache", removed);
        }
        removed
    }

    /// Get cache statistics
    #[allow(dead_code)]
    pub async fn stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        CacheStats {
            hits: stats.hits,
            misses: stats.misses,
            evictions: stats.evictions,
        }
    }

    /// Get current cache size
    #[allow(dead_code)]
    pub async fn size(&self) -> usize {
        self.cache.read().await.len()
    }

    /// Clear all cached sessions (useful for testing)
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        let count = cache.len();
        cache.clear();
        info!("Cleared {} sessions from cache", count);
    }
}

/// Global session cache instance
static SESSION_CACHE: tokio::sync::OnceCell<SessionCache> = tokio::sync::OnceCell::const_new();

/// Get the global session cache
pub async fn get_session_cache() -> &'static SessionCache {
    SESSION_CACHE.get_or_init(|| async {
        info!("🔥 Initializing global session cache");
        SessionCache::new()
    }).await
}