use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use crate::utils::AppState;
use crate::utils::AppError;
use chrono::Utc;

#[derive(Debug, Deserialize)]
pub struct ForgetAfterRequest {
    pub message_id: String,
}

#[derive(Debug, Serialize)]
pub struct ForgetStatusResponse {
    pub has_forgotten: bool,
    pub forgotten_count: i64,
}

#[handler]
pub async fn forget_messages_after(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let conversation_id = req.param::<String>("conversation_id")
        .ok_or_else(|| AppError::BadRequest("Missing conversation_id".to_string()))?;
    
    let forget_request: ForgetAfterRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;
    
    // Get the timestamp of the specified message
    let message_timestamp = sqlx::query_scalar::<_, chrono::DateTime<Utc>>(
        "SELECT created_at FROM messages WHERE id = $1 AND conversation_id = $2"
    )
    .bind(&forget_request.message_id)
    .bind(&conversation_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    .ok_or(AppError::NotFound("Message not found".to_string()))?;
    
    // Mark all messages after this one as forgotten
    let update_result = sqlx::query(
        "UPDATE messages 
         SET is_forgotten = true 
         WHERE conversation_id = $1 
         AND created_at > $2"
    )
    .bind(&conversation_id)
    .bind(message_timestamp)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to mark messages as forgotten: {}", e)))?;
    
    let forgotten_count = update_result.rows_affected() as i64;
    
    res.render(Json(ForgetStatusResponse {
        has_forgotten: forgotten_count > 0,
        forgotten_count,
    }));
    Ok(())
}

#[handler]
pub async fn restore_forgotten_messages(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let conversation_id = req.param::<String>("conversation_id")
        .ok_or_else(|| AppError::BadRequest("Missing conversation_id".to_string()))?;
    
    // Mark all messages as not forgotten
    let update_result = sqlx::query(
        "UPDATE messages 
         SET is_forgotten = false 
         WHERE conversation_id = $1 
         AND is_forgotten = true"
    )
    .bind(&conversation_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to restore messages: {}", e)))?;
    
    let restored_count = update_result.rows_affected() as i64;
    
    res.render(Json(ForgetStatusResponse {
        has_forgotten: false,
        forgotten_count: restored_count,
    }));
    Ok(())
}

#[handler]
pub async fn get_forgotten_status(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let conversation_id = req.param::<String>("conversation_id")
        .ok_or_else(|| AppError::BadRequest("Missing conversation_id".to_string()))?;
    
    // Count forgotten messages
    let forgotten_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM messages 
         WHERE conversation_id = $1 
         AND is_forgotten = true"
    )
    .bind(&conversation_id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(0);
    
    res.render(Json(ForgetStatusResponse {
        has_forgotten: forgotten_count > 0,
        forgotten_count,
    }));
    
    Ok(())
}