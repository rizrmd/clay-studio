use super::types::MessageResponse;
use crate::utils::AppError;
use crate::utils::get_app_state;
use chrono::Utc;
use salvo::prelude::*;
use sqlx::Row;

#[handler]
pub async fn get_conversation_messages(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let conversation_id = req
        .param::<String>("conversation_id")
        .ok_or(AppError::BadRequest("Missing conversation_id".to_string()))?;

    // Fetch messages from database first, filtering out forgotten ones
    // Order by created_at and then by id to ensure stable ordering
    let messages = sqlx::query(
        "SELECT id, content, role, processing_time_ms, created_at 
         FROM messages 
         WHERE conversation_id = $1 
         AND (is_forgotten = false OR is_forgotten IS NULL)
         ORDER BY created_at ASC, id ASC",
    )
    .bind(&conversation_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    tracing::debug!(
        "Found {} messages for conversation {}",
        messages.len(),
        conversation_id
    );

    let mut message_responses = Vec::new();
    for row in messages {
        let msg_id: String = row
            .try_get("id")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get id: {}", e)))?;

        // Fetch tool_usages for this message if it's an assistant message
        let msg_role: String = row
            .try_get("role")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get role: {}", e)))?;

        let tool_usages = if msg_role == "assistant" {
            // Fetch tool usages for this message
            let tool_usage_rows = sqlx::query(
                r#"
                SELECT 
                    id,
                    message_id,
                    tool_name,
                    parameters,
                    output,
                    execution_time_ms,
                    created_at
                FROM tool_usages 
                WHERE message_id = $1 
                ORDER BY created_at ASC
                "#,
            )
            .bind(&msg_id)
            .fetch_all(&state.db_pool)
            .await
            .map_err(|e| {
                AppError::InternalServerError(format!("Failed to fetch tool usages: {}", e))
            })?;

            if !tool_usage_rows.is_empty() {
                let mut usages = Vec::new();
                for usage_row in tool_usage_rows {
                    let created_at: Option<chrono::DateTime<chrono::Utc>> =
                        usage_row.try_get("created_at").ok();
                    usages.push(crate::models::tool_usage::ToolUsage {
                        id: usage_row.try_get("id").unwrap_or_default(),
                        message_id: usage_row.try_get("message_id").unwrap_or_default(),
                        tool_name: usage_row.try_get("tool_name").unwrap_or_default(),
                        tool_use_id: usage_row.try_get("tool_use_id").ok(),
                        parameters: None, // Exclude parameters from conversation messages
                        output: None, // Exclude output from conversation messages
                        execution_time_ms: usage_row.try_get("execution_time_ms").ok(),
                        created_at: created_at.map(|dt| dt.to_rfc3339()),
                    });
                }
                Some(usages)
            } else {
                None
            }
        } else {
            None
        };

        let msg_created_at = row
            .try_get::<chrono::DateTime<Utc>, _>("created_at")
            .map_err(|e| {
                AppError::InternalServerError(format!("Failed to get created_at: {}", e))
            })?;

        tracing::trace!(
            "Message order: {} - {} at {}",
            &msg_id[..8],
            msg_role,
            msg_created_at
        );

        message_responses.push(MessageResponse {
            id: msg_id,
            content: row.try_get("content").map_err(|e| {
                AppError::InternalServerError(format!("Failed to get content: {}", e))
            })?,
            role: msg_role,
            created_at: msg_created_at.to_rfc3339(),
            processing_time_ms: row.try_get("processing_time_ms").ok(),
            tool_usages,
        });
    }

    // Check if there's really an active stream that needs resuming
    // Only mark as active if:
    // 1. Last message is from user OR last assistant message is empty AND
    // 2. Stream exists in active_streams (for real-time streaming detection)
    let has_active_stream = {
        // First check the message state
        let needs_streaming = if let Some(last_msg) = message_responses.last() {
            match last_msg.role.as_str() {
                "user" => true, // Last message is user, assistant hasn't responded
                "assistant" => last_msg.content.is_empty(), // Assistant message is empty (stub)
                _ => false,
            }
        } else {
            false // No messages
        };

        // Only check active_streams if we actually need streaming
        if needs_streaming {
            let streams = state.active_claude_streams.read().await;
            streams.contains_key(&conversation_id)
        } else {
            false // Assistant already has content, no need to stream
        }
    };

    // Include streaming state in response
    let response = serde_json::json!({
        "messages": message_responses,
        "has_active_stream": has_active_stream,
    });

    res.render(Json(response));
    Ok(())
}
