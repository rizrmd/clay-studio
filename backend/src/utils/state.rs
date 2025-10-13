use crate::core::analysis::AnalysisService;
use crate::core::sessions::PostgresSessionStore;
use crate::models::{client::Client, tool_usage::ToolUsage, Message};
use crate::utils::db;
use crate::utils::Config;
use chrono::{DateTime, Utc};
use sea_orm::DatabaseConnection;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::path::PathBuf;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;
use salvo::Depot;

#[derive(Clone, Debug, serde::Serialize)]
#[allow(dead_code)]
pub enum ToolStatus {
    Executing,
    Completed,
}

#[derive(Clone, Debug, serde::Serialize)]
#[allow(dead_code)]
pub struct ToolExecution {
    pub tool_name: String,
    pub tool_usage_id: Uuid,
    pub status: ToolStatus,
    pub execution_time_ms: Option<i64>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug)]
pub struct StreamingState {
    /// The ID of the message being streamed
    pub message_id: String,

    /// Accumulated text content from all progress events (for display)
    pub partial_content: String,

    /// Currently executing or completed tools with their status
    pub active_tools: Vec<ToolExecution>,

    /// Complete history of all events (progress, tool_use, tool_complete)
    /// stored in order to replay them exactly when WebSocket reconnects
    pub progress_events: Vec<serde_json::Value>,

    /// Completed tool usages during streaming (not yet saved to database)
    pub completed_tool_usages: Vec<ToolUsage>,
}

#[derive(Clone, Debug)]
pub struct ConversationCache {
    /// All messages in the conversation (excluding forgotten ones)
    pub messages: Vec<Message>,

    /// Set of WebSocket client IDs currently subscribed to this conversation
    pub subscribers: HashSet<String>,

    /// Last time this cache was accessed
    pub last_accessed: DateTime<Utc>,
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
    pub conversation_cache: Arc<RwLock<HashMap<String, ConversationCache>>>,
    pub session_store: PostgresSessionStore,
    pub analysis_service: AnalysisService,
}

impl AppState {
    pub async fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        // Connect to database with SeaORM (handles its own connection pool)
        let db = db::connect(&config.database_url).await?;

        // Create SQLx connection pool with comprehensive logging
        info!("🔌 Creating SQLx PostgreSQL connection pool...");
        let db_pool = Self::create_sqlx_pool(&config.database_url).await?;

        let db_arc = Arc::new(db);
        let session_store = PostgresSessionStore::new(db_arc.clone());

        // Initialize analysis service
        let data_dir = PathBuf::from("./analysis_data");
        tokio::fs::create_dir_all(&data_dir).await?;
        let analysis_service = AnalysisService::new(db_pool.clone());

        let state = AppState {
            db: db_arc,
            db_pool,
            config: Arc::new(config.clone()),
            clients: Arc::new(RwLock::new(HashMap::new())),
            active_claude_streams: Arc::new(RwLock::new(HashMap::new())),
            conversation_cache: Arc::new(RwLock::new(HashMap::new())),
            session_store,
            analysis_service,
        };

        // Start pool health monitor
        state.start_pool_health_monitor();

        Ok(state)
    }

    async fn create_sqlx_pool(database_url: &str) -> Result<PgPool, Box<dyn std::error::Error>> {
        info!("📊 SQLx Pool configuration:");
        info!("  - Max connections: 20");
        info!("  - Min connections: 3");
        info!("  - Acquire timeout: 10s");
        info!("  - Idle timeout: 300s");
        info!("  - Max lifetime: 900s");

        let pool_options = PgPoolOptions::new()
            .max_connections(20)
            .min_connections(3)
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Some(Duration::from_secs(300)))
            .max_lifetime(Some(Duration::from_secs(900)))
            .test_before_acquire(true)
            .before_acquire(|conn, _meta| Box::pin(async move {
                // Force close connections that might be hung
                let result = sqlx::query("SELECT 1").fetch_one(conn).await;
                Ok(result.is_ok())
            }));

        // Attempt connection with retry logic
        let mut attempts = 0;
        const MAX_ATTEMPTS: u8 = 3;

        let pool = loop {
            attempts += 1;
            info!(
                "🔄 SQLx pool connection attempt {}/{}",
                attempts, MAX_ATTEMPTS
            );

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
                    } else if error_msg.contains("authentication") || error_msg.contains("password")
                    {
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
        info!(
            "📈 [{}] Pool Stats - Total: {}, Idle: {}, Active: {}",
            context,
            stats,
            idle,
            stats.saturating_sub(idle)
        );

        // Warn if pool usage is high
        if stats > 10 {
            warn!(
                "⚠️  High database connection usage: {}/15 connections active",
                stats
            );
        }

        if idle == 0 && stats > 5 {
            warn!("⚠️  No idle database connections available - potential bottleneck");
        }
    }

    /// Health check for database connections
    pub async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Test SeaORM connection
        match sqlx::query("SELECT 1 as test")
            .fetch_one(&self.db_pool)
            .await
        {
            Ok(_) => info!("✅ SeaORM connection healthy"),
            Err(e) => {
                error!("❌ SeaORM connection unhealthy: {}", e);
                return Err(e.into());
            }
        }

        // Test SQLx connection
        match sqlx::query("SELECT 1 as test")
            .fetch_one(&self.db_pool)
            .await
        {
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

    /// Load conversation messages from database and cache them
    pub async fn load_conversation_cache(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
        // Fetch messages from database
        let messages = sqlx::query(
            "SELECT id, content, role, processing_time_ms, created_at, progress_content 
             FROM messages 
             WHERE conversation_id = $1 
             AND (is_forgotten = false OR is_forgotten IS NULL)
             ORDER BY created_at ASC, id ASC",
        )
        .bind(conversation_id)
        .fetch_all(&self.db_pool)
        .await?;

        let mut cached_messages = Vec::new();

        for row in messages {
            let msg_id: String = row.get("id");
            let role_str: String = row.get("role");
            let role = match role_str.as_str() {
                "user" => crate::models::MessageRole::User,
                "assistant" => crate::models::MessageRole::Assistant,
                "system" => crate::models::MessageRole::System,
                _ => continue,
            };

            // Fetch tool usages if it's an assistant message
            let tool_usages = if role == crate::models::MessageRole::Assistant {
                let tool_rows = sqlx::query(
                    "SELECT id, message_id, tool_name, tool_use_id, parameters, output, execution_time_ms, created_at
                     FROM tool_usages 
                     WHERE message_id = $1 
                     ORDER BY created_at ASC"
                )
                .bind(&msg_id)
                .fetch_all(&self.db_pool)
                .await?;

                if !tool_rows.is_empty() {
                    let mut usages = Vec::new();
                    for tool_row in tool_rows {
                        usages.push(ToolUsage {
                            id: tool_row.get("id"),
                            message_id: tool_row.get("message_id"),
                            tool_name: tool_row.get("tool_name"),
                            tool_use_id: tool_row.get("tool_use_id"),
                            parameters: tool_row.get("parameters"),
                            output: tool_row.get("output"),
                            execution_time_ms: tool_row.get("execution_time_ms"),
                            created_at: tool_row
                                .get::<Option<DateTime<Utc>>, _>("created_at")
                                .map(|dt| dt.to_rfc3339()),
                        });
                    }
                    Some(usages)
                } else {
                    None
                }
            } else {
                None
            };

            let progress_content: Option<String> = row.get("progress_content");

            let message = Message {
                id: msg_id.clone(),
                content: row.get("content"),
                role,
                processing_time_ms: row.get("processing_time_ms"),
                created_at: row
                    .get::<DateTime<Utc>, _>("created_at")
                    .to_rfc3339()
                    .into(),
                file_attachments: None,
                tool_usages,
                progress_content: progress_content.clone(),
            };

            // Debug log for messages with progress_content
            if progress_content.is_some() {
                info!(
                    "  📝 Message {} has progress_content (len: {})",
                    &msg_id[..8],
                    progress_content.as_ref().unwrap().len()
                );
            }

            cached_messages.push(message);
        }

        // Update cache
        let mut cache = self.conversation_cache.write().await;
        cache.insert(
            conversation_id.to_string(),
            ConversationCache {
                messages: cached_messages.clone(),
                subscribers: HashSet::new(),
                last_accessed: Utc::now(),
            },
        );

        info!(
            "📝 Cached {} messages for conversation {}",
            cached_messages.len(),
            conversation_id
        );

        Ok(cached_messages)
    }

    /// Get cached messages or load from database
    pub async fn get_conversation_messages(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
        // Check cache first
        {
            let cache = self.conversation_cache.read().await;
            if let Some(cached) = cache.get(conversation_id) {
                // If messages are not empty, return them
                if !cached.messages.is_empty() {
                    // Update last accessed time
                    drop(cache);
                    let mut cache_write = self.conversation_cache.write().await;
                    if let Some(cached_mut) = cache_write.get_mut(conversation_id) {
                        cached_mut.last_accessed = Utc::now();
                        return Ok(cached_mut.messages.clone());
                    }
                }
                // Messages are empty, need to load from database
            }
        }

        // Not in cache or cache is empty, load from database
        self.load_conversation_cache(conversation_id).await
    }

    /// Add a subscriber to a conversation
    pub async fn add_conversation_subscriber(&self, conversation_id: &str, client_id: &str) {
        let mut cache = self.conversation_cache.write().await;

        if let Some(cached) = cache.get_mut(conversation_id) {
            cached.subscribers.insert(client_id.to_string());
            cached.last_accessed = Utc::now();
            info!(
                "➕ Added subscriber {} to conversation {} (total: {})",
                client_id,
                conversation_id,
                cached.subscribers.len()
            );
        } else {
            // Create new cache entry with this subscriber
            let mut subscribers = HashSet::new();
            subscribers.insert(client_id.to_string());

            cache.insert(
                conversation_id.to_string(),
                ConversationCache {
                    messages: Vec::new(), // Will be loaded on first access
                    subscribers,
                    last_accessed: Utc::now(),
                },
            );
            info!(
                "➕ Created cache for conversation {} with subscriber {}",
                conversation_id, client_id
            );
        }
    }

    /// Remove a subscriber from a conversation
    pub async fn remove_conversation_subscriber(&self, conversation_id: &str, client_id: &str) {
        let mut cache = self.conversation_cache.write().await;

        if let Some(cached) = cache.get_mut(conversation_id) {
            cached.subscribers.remove(client_id);
            info!(
                "➖ Removed subscriber {} from conversation {} (remaining: {})",
                client_id,
                conversation_id,
                cached.subscribers.len()
            );

            // Remove cache entry if no subscribers remain
            if cached.subscribers.is_empty() {
                cache.remove(conversation_id);
                info!(
                    "🗑️ Removed cache for conversation {} (no subscribers)",
                    conversation_id
                );
            }
        }
    }

    /// Add or update a message in the cache
    pub async fn update_conversation_cache(&self, conversation_id: &str, message: Message) {
        let mut cache = self.conversation_cache.write().await;

        if let Some(cached) = cache.get_mut(conversation_id) {
            // Check if message already exists (update) or is new (add)
            if let Some(existing) = cached.messages.iter_mut().find(|m| m.id == message.id) {
                *existing = message;
            } else {
                cached.messages.push(message);
            }
            cached.last_accessed = Utc::now();
        }
    }

    /// Invalidate and reload conversation cache (used when forget/restore operations change message visibility)
    pub async fn invalidate_conversation_cache(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
        // Remove from cache first
        {
            let mut cache = self.conversation_cache.write().await;
            cache.remove(conversation_id);
        }

        // Reload from database (this will create a fresh cache entry)
        self.load_conversation_cache(conversation_id).await
    }

    /// Start background task to monitor pool health and attempt recovery
    fn start_pool_health_monitor(&self) {
        let pool = self.db_pool.clone();
        let db = self.db.clone();

        tokio::spawn(async move {
            let mut check_interval = tokio::time::interval(Duration::from_secs(30));
            let mut consecutive_failures = 0u32;

            loop {
                check_interval.tick().await;

                // Health check with timeout (increased to 10s to avoid false positives under load)
                let health_check = tokio::time::timeout(
                    Duration::from_secs(10),
                    sqlx::query("SELECT 1 as health_check").fetch_one(&pool)
                ).await;

                match health_check {
                    Ok(Ok(_)) => {
                        if consecutive_failures > 0 {
                            info!("✅ Database pool recovered after {} failures", consecutive_failures);
                            consecutive_failures = 0;
                        }
                    }
                    Ok(Err(e)) => {
                        consecutive_failures += 1;
                        error!("❌ Pool health check failed (attempt {}): {}", consecutive_failures, e);

                        // Attempt recovery by testing connections
                        if consecutive_failures >= 3 {
                            warn!("🔄 Attempting pool recovery - checking database connectivity...");

                            // Test SeaORM connection
                            match db.ping().await {
                                Ok(_) => {
                                    info!("✅ SeaORM connection still healthy");
                                }
                                Err(e) => {
                                    error!("❌ SeaORM connection also failed: {}", e);
                                }
                            }

                            // Log pool state for diagnostics
                            let stats = pool.size();
                            let idle = pool.num_idle();
                            warn!(
                                "📊 Pool state during failure - Total: {}, Idle: {}, Active: {}",
                                stats,
                                idle,
                                stats.saturating_sub(idle as u32)
                            );
                        }

                        if consecutive_failures >= 5 {
                            error!("🚨 CRITICAL: Pool has failed {} consecutive health checks", consecutive_failures);
                            error!("💡 This may be due to backend live reload. Consider implementing connection pool persistence.");
                        }
                    }
                    Err(_) => {
                        consecutive_failures += 1;
                        error!("❌ Pool health check timed out (attempt {})", consecutive_failures);

                        if consecutive_failures >= 5 {
                            error!("🚨 CRITICAL: Pool has failed {} consecutive health checks - timeout", consecutive_failures);
                        }
                    }
                }

                // Log pool stats periodically
                if consecutive_failures == 0 {
                    let stats = pool.size();
                    let idle = pool.num_idle() as u32;
                    info!(
                        "📊 Pool Health - Total: {}, Idle: {}, Active: {}",
                        stats,
                        idle,
                        stats.saturating_sub(idle)
                    );
                }
            }
        });
    }
}

/// Helper function to safely extract AppState from Depot
/// This prevents panics from unwrap() calls throughout the codebase
pub fn get_app_state(depot: &Depot) -> Result<&AppState, salvo::http::StatusError> {
    depot
        .obtain::<AppState>()
        .map_err(|_| salvo::http::StatusError::internal_server_error())
}
