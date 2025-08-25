use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use crate::utils::AppState;
use crate::utils::AppError;
use sqlx::Row;

#[derive(Debug, Deserialize)]
pub struct ForgetAfterRequest {
    pub message_id: String,
}

#[derive(Debug, Serialize)]
pub struct ForgetStatusResponse {
    pub has_forgotten: bool,
    pub forgotten_after_message_id: Option<String>,
    pub forgotten_count: Option<i32>,
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
    
    // Count messages that will be forgotten
    let count_result = sqlx::query(
        "SELECT COUNT(*) as count FROM messages 
         WHERE conversation_id = $1 AND created_at > 
         (SELECT created_at FROM messages WHERE id = $2 AND conversation_id = $1)"
    )
    .bind(&conversation_id)
    .bind(&forget_request.message_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to count messages: {}", e)))?;
    
    let forgotten_count: i64 = count_result.get("count");
    
    // Update conversation with forgotten marker
    sqlx::query(
        "UPDATE conversations 
         SET forgotten_after_message_id = $1, forgotten_count = $2 
         WHERE id = $3"
    )
    .bind(&forget_request.message_id)
    .bind(forgotten_count as i32)
    .bind(&conversation_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to update conversation: {}", e)))?;
    
    res.render(Json(ForgetStatusResponse {
        has_forgotten: true,
        forgotten_after_message_id: Some(forget_request.message_id),
        forgotten_count: Some(forgotten_count as i32),
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
    
    // Clear forgotten marker
    sqlx::query(
        "UPDATE conversations 
         SET forgotten_after_message_id = NULL, forgotten_count = 0 
         WHERE id = $1"
    )
    .bind(&conversation_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to update conversation: {}", e)))?;
    
    res.render(Json(ForgetStatusResponse {
        has_forgotten: false,
        forgotten_after_message_id: None,
        forgotten_count: None,
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
    
    let result = sqlx::query(
        "SELECT forgotten_after_message_id, forgotten_count 
         FROM conversations WHERE id = $1"
    )
    .bind(&conversation_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to query conversation: {}", e)))?;
    
    if let Some(row) = result {
        let forgotten_after_message_id: Option<String> = row.get("forgotten_after_message_id");
        let forgotten_count: Option<i32> = row.get("forgotten_count");
        
        res.render(Json(ForgetStatusResponse {
            has_forgotten: forgotten_after_message_id.is_some(),
            forgotten_after_message_id,
            forgotten_count,
        }));
    } else {
        res.render(Json(ForgetStatusResponse {
            has_forgotten: false,
            forgotten_after_message_id: None,
            forgotten_count: None,
        }));
    }
    
    Ok(())
}