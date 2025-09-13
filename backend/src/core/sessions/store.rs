use crate::models::session;
use super::cache::get_session_cache;
use async_session::{Result, Session, SessionStore};
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::sync::Arc;
use tracing::debug;

#[derive(Clone, Debug)]
pub struct PostgresSessionStore {
    db: Arc<DatabaseConnection>,
}

impl PostgresSessionStore {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    async fn cleanup_expired(&self) -> std::result::Result<(), sea_orm::DbErr> {
        use sea_orm::DeleteResult;

        let now = Utc::now();
        let _result: DeleteResult = session::Entity::delete_many()
            .filter(session::Column::ExpiresAt.lt(now))
            .exec(self.db.as_ref())
            .await?;

        Ok(())
    }
}

#[async_trait]
impl SessionStore for PostgresSessionStore {
    async fn load_session(&self, cookie_value: String) -> Result<Option<Session>> {
        // Extract session ID from cookie value
        let session_id = Session::id_from_cookie_value(&cookie_value)?;
        
        // Try to get session from cache first
        let cache = get_session_cache().await;
        if let Some(cached_session) = cache.get(&session_id).await {
            debug!("Session {} loaded from cache", session_id);
            return Ok(Some(cached_session));
        }

        // Cache miss - load from database
        debug!("Session {} cache miss, loading from database", session_id);
        
        // Clean up expired sessions occasionally (1% chance)
        if rand::random::<f32>() < 0.01 {
            let _ = self.cleanup_expired().await;
            let _ = cache.cleanup_expired().await;
        }

        let session_record = session::Entity::find_by_id(&session_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;

        match session_record {
            Some(record) => {
                // Check if session has expired
                if record.expires_at.timestamp() < Utc::now().timestamp() {
                    // Delete expired session and remove from cache
                    session::Entity::delete_by_id(&session_id)
                        .exec(self.db.as_ref())
                        .await
                        .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
                    cache.remove(&session_id).await;
                    Ok(None)
                } else {
                    // Deserialize session data from JSON
                    let session: Session = serde_json::from_value(record.data)
                        .map_err(|e| anyhow::anyhow!("Failed to deserialize session: {}", e))?;

                    // Cache the session for future requests
                    cache.set(session_id.clone(), session.clone(), record.expires_at.into()).await;

                    // Validate session before returning
                    Ok(Session::validate(session))
                }
            }
            None => {
                // Session not found in database, ensure it's not cached
                cache.remove(&session_id).await;
                Ok(None)
            }
        }
    }

    async fn store_session(&self, session: Session) -> Result<Option<String>> {
        let session_id = session.id().to_string();
        let now = Utc::now();

        // Get expiry from session or default to 24 hours
        let expires_at = session
            .expiry()
            .copied()
            .unwrap_or_else(|| now + chrono::Duration::hours(24));

        // Serialize session to JSON
        let session_json = serde_json::to_string(&session)
            .map_err(|e| anyhow::anyhow!("Failed to serialize session: {}", e))?;

        // Check if session exists
        let existing = session::Entity::find_by_id(&session_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;

        let json_data: serde_json::Value = serde_json::from_str(&session_json)
            .map_err(|e| anyhow::anyhow!("Failed to parse session JSON: {}", e))?;

        if existing.is_some() {
            // Update existing session
            let active_model = session::ActiveModel {
                id: Set(session_id.clone()),
                data: Set(json_data),
                expires_at: Set(expires_at.into()),
                updated_at: Set(now.into()),
                ..Default::default()
            };
            active_model
                .update(self.db.as_ref())
                .await
                .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
        } else {
            // Create new session
            let new_session = session::ActiveModel {
                id: Set(session_id.clone()),
                data: Set(json_data),
                expires_at: Set(expires_at.into()),
                created_at: Set(now.into()),
                updated_at: Set(now.into()),
            };
            new_session
                .insert(self.db.as_ref())
                .await
                .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
        }

        // Update cache with the stored session
        let cache = get_session_cache().await;
        cache.set(session_id, session.clone(), expires_at).await;

        // Reset data changed flag and return cookie value
        session.reset_data_changed();
        Ok(session.into_cookie_value())
    }

    async fn destroy_session(&self, session: Session) -> Result {
        let session_id = session.id();

        // Remove from database
        session::Entity::delete_by_id(session_id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;

        // Remove from cache
        let cache = get_session_cache().await;
        cache.remove(session_id).await;

        Ok(())
    }

    async fn clear_store(&self) -> Result {
        // Delete all sessions from database
        session::Entity::delete_many()
            .exec(self.db.as_ref())
            .await
            .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;

        // Clear all sessions from cache
        let cache = get_session_cache().await;
        cache.clear().await;

        Ok(())
    }
}
