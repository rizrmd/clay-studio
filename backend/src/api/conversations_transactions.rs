use salvo::prelude::*;
use serde::Deserialize;
use crate::models::*;
use crate::utils::AppState;
use crate::utils::AppError;
use chrono::Utc;
use uuid::Uuid;
use sqlx::{Transaction, Postgres};

#[derive(Debug, Deserialize)]
pub struct CreateConversationWithMessageRequest {
    pub project_id: String,
    pub title: Option<String>,
    pub initial_message: Option<String>,
}

/// Create a conversation with an optional initial message using a database transaction
#[handler]
pub async fn create_conversation_with_message(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let create_req: CreateConversationWithMessageRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;
    
    // Start a database transaction
    let mut tx: Transaction<'_, Postgres> = state.db_pool.begin().await
        .map_err(|e| AppError::InternalServerError(format!("Failed to start transaction: {}", e)))?;
    
    let conversation_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    
    // Validate that the project exists
    let project_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE id = $1)"
    )
    .bind(&create_req.project_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    if !project_exists {
        return Err(AppError::BadRequest("Project not found".to_string()));
    }
    
    // Insert conversation into database
    sqlx::query(
        "INSERT INTO conversations (id, project_id, title, message_count, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(&conversation_id)
    .bind(&create_req.project_id)
    .bind(&create_req.title)
    .bind(0i32)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to create conversation: {}", e)))?;
    
    let mut message_count = 0;
    
    // If there's an initial message, add it
    if let Some(initial_message) = create_req.initial_message {
        let message_id = Uuid::new_v4().to_string();
        
        sqlx::query(
            "INSERT INTO messages (id, conversation_id, content, role, created_at) 
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(&message_id)
        .bind(&conversation_id)
        .bind(&initial_message)
        .bind("user")
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to create initial message: {}", e)))?;
        
        // Update conversation message count
        sqlx::query(
            "UPDATE conversations SET message_count = message_count + 1 WHERE id = $1"
        )
        .bind(&conversation_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to update message count: {}", e)))?;
        
        message_count = 1;
    }
    
    // Commit the transaction
    tx.commit().await
        .map_err(|e| AppError::InternalServerError(format!("Failed to commit transaction: {}", e)))?;
    
    let conversation = Conversation {
        id: conversation_id,
        project_id: create_req.project_id,
        title: create_req.title,
        created_at: now,
        updated_at: now,
        message_count,
    };

    res.render(Json(conversation));
    Ok(())
}

/// Delete a conversation and all its messages using a transaction
#[handler]
pub async fn delete_conversation_cascade(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let conversation_id = req.param::<String>("conversation_id")
        .ok_or(AppError::BadRequest("Missing conversation_id".to_string()))?;
    
    // Start a database transaction
    let mut tx: Transaction<'_, Postgres> = state.db_pool.begin().await
        .map_err(|e| AppError::InternalServerError(format!("Failed to start transaction: {}", e)))?;
    
    // Check if conversation exists and get message count
    let conversation_info = sqlx::query_as::<_, (String, i32)>(
        "SELECT id, message_count FROM conversations WHERE id = $1"
    )
    .bind(&conversation_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    if conversation_info.is_none() {
        return Err(AppError::NotFound("Conversation not found".to_string()));
    }
    
    let (_, message_count) = conversation_info.unwrap();
    
    // Delete all messages first (due to foreign key constraint)
    let deleted_messages = sqlx::query(
        "DELETE FROM messages WHERE conversation_id = $1"
    )
    .bind(&conversation_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to delete messages: {}", e)))?;
    
    // Delete the conversation
    sqlx::query(
        "DELETE FROM conversations WHERE id = $1"
    )
    .bind(&conversation_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to delete conversation: {}", e)))?;
    
    // Commit the transaction
    tx.commit().await
        .map_err(|e| AppError::InternalServerError(format!("Failed to commit transaction: {}", e)))?;
    
    res.render(Json(serde_json::json!({
        "success": true,
        "deleted_id": conversation_id,
        "deleted_messages": deleted_messages.rows_affected(),
        "expected_messages": message_count
    })));
    Ok(())
}

/// Batch update multiple conversations within a transaction
#[derive(Debug, Deserialize)]
pub struct BatchUpdateRequest {
    pub conversation_ids: Vec<String>,
    pub update: ConversationUpdate,
}

#[derive(Debug, Deserialize)]
pub struct ConversationUpdate {
    pub title: Option<String>,
    pub project_id: Option<String>,
}

#[handler]
pub async fn batch_update_conversations(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let batch_req: BatchUpdateRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;
    
    if batch_req.conversation_ids.is_empty() {
        return Err(AppError::BadRequest("No conversation IDs provided".to_string()));
    }
    
    // Start a database transaction
    let mut tx: Transaction<'_, Postgres> = state.db_pool.begin().await
        .map_err(|e| AppError::InternalServerError(format!("Failed to start transaction: {}", e)))?;
    
    let now = Utc::now();
    let mut updated_count = 0;
    
    // If updating project_id, validate it exists
    if let Some(ref new_project_id) = batch_req.update.project_id {
        let project_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = $1)"
        )
        .bind(new_project_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
        
        if !project_exists {
            return Err(AppError::BadRequest("Target project not found".to_string()));
        }
    }
    
    // Update each conversation
    for conversation_id in &batch_req.conversation_ids {
        let mut query_parts = vec!["UPDATE conversations SET updated_at = $1"];
        let mut bind_values: Vec<Box<dyn sqlx::Encode<'_, Postgres> + Send + Sync>> = vec![
            Box::new(now),
        ];
        let mut param_count = 2;
        
        if let Some(ref title) = batch_req.update.title {
            query_parts.push(&format!("title = ${}", param_count));
            bind_values.push(Box::new(title.clone()));
            param_count += 1;
        }
        
        if let Some(ref project_id) = batch_req.update.project_id {
            query_parts.push(&format!("project_id = ${}", param_count));
            bind_values.push(Box::new(project_id.clone()));
            param_count += 1;
        }
        
        query_parts.push(&format!("WHERE id = ${}", param_count));
        bind_values.push(Box::new(conversation_id.clone()));
        
        let query = query_parts.join(", ");
        
        // Execute dynamic query
        let result = sqlx::query(&query)
            .bind(now)
            .bind(batch_req.update.title.as_ref())
            .bind(batch_req.update.project_id.as_ref())
            .bind(conversation_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::InternalServerError(format!("Failed to update conversation: {}", e)))?;
        
        updated_count += result.rows_affected();
    }
    
    // Commit the transaction
    tx.commit().await
        .map_err(|e| AppError::InternalServerError(format!("Failed to commit transaction: {}", e)))?;
    
    res.render(Json(serde_json::json!({
        "success": true,
        "updated_count": updated_count,
        "requested_count": batch_req.conversation_ids.len()
    })));
    Ok(())
}