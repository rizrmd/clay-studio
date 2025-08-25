use async_trait::async_trait;
use async_session::{Session, SessionStore, Result};
use sea_orm::{DatabaseConnection, EntityTrait, Set, ActiveModelTrait, QueryFilter, ColumnTrait};
use chrono::Utc;
use std::sync::Arc;
use crate::models::session;

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
        // Clean up expired sessions occasionally (1% chance)
        if rand::random::<f32>() < 0.01 {
            let _ = self.cleanup_expired().await;
        }
        
        // Extract session ID from cookie value
        let session_id = Session::id_from_cookie_value(&cookie_value)?;
        
        let session_record = session::Entity::find_by_id(&session_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
        
        match session_record {
            Some(record) => {
                // Check if session has expired
                if record.expires_at.timestamp() < Utc::now().timestamp() {
                    // Delete expired session
                    session::Entity::delete_by_id(&session_id)
                        .exec(self.db.as_ref())
                        .await
                        .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
                    Ok(None)
                } else {
                    // Deserialize session data from JSON
                    let session: Session = serde_json::from_value(record.data)
                        .map_err(|e| anyhow::anyhow!("Failed to deserialize session: {}", e))?;
                    
                    // Validate session before returning
                    Ok(Session::validate(session))
                }
            }
            None => Ok(None)
        }
    }

    async fn store_session(&self, session: Session) -> Result<Option<String>> {
        let session_id = session.id().to_string();
        let now = Utc::now();
        
        // Get expiry from session or default to 24 hours
        let expires_at = session.expiry()
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
            active_model.update(self.db.as_ref()).await
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
            new_session.insert(self.db.as_ref()).await
                .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
        }
        
        // Reset data changed flag and return cookie value
        session.reset_data_changed();
        Ok(session.into_cookie_value())
    }

    async fn destroy_session(&self, session: Session) -> Result {
        let session_id = session.id();
        
        session::Entity::delete_by_id(session_id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
        
        Ok(())
    }

    async fn clear_store(&self) -> Result {
        // Delete all sessions
        session::Entity::delete_many()
            .exec(self.db.as_ref())
            .await
            .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
        
        Ok(())
    }
}