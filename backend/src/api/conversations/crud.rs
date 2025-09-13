use super::types::{
    CreateConversationRequest, CreateFromMessageRequest, UpdateConversationRequest,
};
use crate::models::*;
use crate::utils::middleware::{get_current_client_id, get_current_user_id, is_current_user_root};
use crate::utils::{get_app_state, AppError};
use chrono::Utc;
use salvo::prelude::*;
use sqlx::Row;
use uuid::Uuid;

#[handler]
pub async fn list_conversations(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;

    // Get current user's ID for filtering
    let user_id = get_current_user_id(depot)?;
    let client_id = get_current_client_id(depot)?;

    // Optional project_id filter from query params
    let project_id = req.query::<String>("project_id");

    // Build query based on filters - calculate actual message count excluding forgotten messages
    // Include client_id filtering through projects table join
    let conversations = if let Some(pid) = project_id {
        if is_current_user_root(depot) {
            sqlx::query(
                "SELECT 
                    c.id, 
                    c.project_id, 
                    c.title, 
                    (
                        SELECT COUNT(*)::INTEGER 
                        FROM messages m 
                        WHERE m.conversation_id = c.id
                        AND (m.is_forgotten = false OR m.is_forgotten IS NULL)
                    ) AS message_count,
                    c.created_at, 
                    c.updated_at, 
                    c.is_title_manually_set 
                 FROM conversations c
                 WHERE c.project_id = $1 
                 ORDER BY c.created_at DESC 
                 LIMIT 100",
            )
            .bind(pid)
            .fetch_all(&state.db_pool)
            .await
        } else {
            sqlx::query(
                "SELECT 
                    c.id, 
                    c.project_id, 
                    c.title, 
                    (
                        SELECT COUNT(*)::INTEGER 
                        FROM messages m 
                        WHERE m.conversation_id = c.id
                        AND (m.is_forgotten = false OR m.is_forgotten IS NULL)
                    ) AS message_count,
                    c.created_at, 
                    c.updated_at, 
                    c.is_title_manually_set 
                 FROM conversations c
                 WHERE c.project_id = $1
                 ORDER BY c.created_at DESC 
                 LIMIT 100",
            )
            .bind(pid)
            .fetch_all(&state.db_pool)
            .await
        }
    } else if is_current_user_root(depot) {
        sqlx::query(
            "SELECT 
                c.id, 
                c.project_id, 
                c.title, 
                (
                    SELECT COUNT(*)::INTEGER 
                    FROM messages m 
                    WHERE m.conversation_id = c.id
                    AND (m.is_forgotten = false OR m.is_forgotten IS NULL)
                ) AS message_count,
                c.created_at, 
                c.updated_at, 
                c.is_title_manually_set 
             FROM conversations c
             ORDER BY c.created_at DESC 
             LIMIT 100",
        )
        .fetch_all(&state.db_pool)
        .await
    } else {
        sqlx::query(
            "SELECT 
                c.id, 
                c.project_id, 
                c.title, 
                (
                    SELECT COUNT(*)::INTEGER 
                    FROM messages m 
                    WHERE m.conversation_id = c.id
                    AND (m.is_forgotten = false OR m.is_forgotten IS NULL)
                ) AS message_count,
                c.created_at, 
                c.updated_at, 
                c.is_title_manually_set 
             FROM conversations c
             JOIN projects p ON c.project_id = p.id
             WHERE p.user_id = $1
             ORDER BY c.created_at DESC 
             LIMIT 100",
        )
        .bind(user_id)
        .fetch_all(&state.db_pool)
        .await
    }
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let mut conversation_list = Vec::new();
    for row in conversations {
        let id: String = row
            .try_get("id")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get id: {}", e)))?;
        let message_count: i32 = row.try_get("message_count").map_err(|e| {
            AppError::InternalServerError(format!("Failed to get message_count: {}", e))
        })?;

        // Debug logging
        tracing::debug!("Conversation {} has message_count: {}", id, message_count);

        conversation_list.push(Conversation {
            id,
            project_id: row.try_get("project_id").map_err(|e| {
                AppError::InternalServerError(format!("Failed to get project_id: {}", e))
            })?,
            title: row.try_get("title").ok(),
            created_at: row.try_get("created_at").map_err(|e| {
                AppError::InternalServerError(format!("Failed to get created_at: {}", e))
            })?,
            updated_at: row.try_get("updated_at").map_err(|e| {
                AppError::InternalServerError(format!("Failed to get updated_at: {}", e))
            })?,
            message_count,
            is_title_manually_set: row.try_get("is_title_manually_set").ok(),
        });
    }

    res.render(Json(conversation_list));
    Ok(())
}

#[handler]
pub async fn get_conversation(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let conversation_id = req
        .param::<String>("conversation_id")
        .ok_or(AppError::BadRequest("Missing conversation_id".to_string()))?;

    let conversation_row = sqlx::query(
        "SELECT 
            c.id, 
            c.project_id, 
            c.title, 
            (
                SELECT COUNT(*)::INTEGER 
                FROM messages m 
                WHERE m.conversation_id = c.id
                AND (m.is_forgotten = false OR m.is_forgotten IS NULL)
            ) AS message_count,
            c.created_at, 
            c.updated_at, 
            c.is_title_manually_set 
         FROM conversations c
         WHERE c.id = $1",
    )
    .bind(&conversation_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    .ok_or(AppError::NotFound(format!(
        "Conversation {} not found",
        conversation_id
    )))?;

    let conversation = Conversation {
        id: conversation_row
            .try_get("id")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get id: {}", e)))?,
        project_id: conversation_row.try_get("project_id").map_err(|e| {
            AppError::InternalServerError(format!("Failed to get project_id: {}", e))
        })?,
        title: conversation_row.try_get("title").ok(),
        created_at: conversation_row.try_get("created_at").map_err(|e| {
            AppError::InternalServerError(format!("Failed to get created_at: {}", e))
        })?,
        updated_at: conversation_row.try_get("updated_at").map_err(|e| {
            AppError::InternalServerError(format!("Failed to get updated_at: {}", e))
        })?,
        message_count: conversation_row.try_get("message_count").map_err(|e| {
            AppError::InternalServerError(format!("Failed to get message_count: {}", e))
        })?,
        is_title_manually_set: conversation_row.try_get("is_title_manually_set").ok(),
    };

    res.render(Json(conversation));
    Ok(())
}

#[handler]
pub async fn create_conversation(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let create_req: CreateConversationRequest = req
        .parse_json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;

    let conversation_id = Uuid::new_v4().to_string();
    let now = Utc::now();

    // Use provided title or None (will be set from first message)
    let title = create_req
        .title
        .as_ref()
        .filter(|t| !t.trim().is_empty())
        .cloned();

    // If title is explicitly provided, mark it as manually set
    let is_manually_set = title.is_some();

    // Insert into database
    sqlx::query(
        "INSERT INTO conversations (id, project_id, title, message_count, created_at, updated_at, is_title_manually_set) 
         VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind(&conversation_id)
    .bind(&create_req.project_id)
    .bind(&title)
    .bind(0i32)
    .bind(now)
    .bind(now)
    .bind(is_manually_set)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let conversation = Conversation {
        id: conversation_id,
        project_id: create_req.project_id,
        title,
        created_at: now,
        updated_at: now,
        message_count: 0,
        is_title_manually_set: Some(is_manually_set),
    };

    res.render(Json(conversation));
    Ok(())
}

#[handler]
pub async fn create_conversation_from_message(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let create_req: CreateFromMessageRequest = req
        .parse_json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;

    // Generate a title from the first user message or use a default
    let mut generated_title = "New Conversation".to_string();
    for msg in &create_req.messages {
        if msg.role == "user" {
            // Take first 50 chars of first user message as title
            let content = msg.content.trim();
            if !content.is_empty() {
                generated_title = if content.len() > 50 {
                    format!("{}...", &content[..47])
                } else {
                    content.to_string()
                };
                break;
            }
        }
    }

    let conversation_id = Uuid::new_v4().to_string();
    let now = Utc::now();

    // Start a transaction
    let mut tx = state.db_pool.begin().await.map_err(|e| {
        AppError::InternalServerError(format!("Failed to start transaction: {}", e))
    })?;

    // Create the new conversation
    sqlx::query(
        "INSERT INTO conversations (id, project_id, title, message_count, created_at, updated_at, is_title_manually_set) 
         VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind(&conversation_id)
    .bind(&create_req.project_id)
    .bind(&generated_title)
    .bind(create_req.messages.len() as i32)
    .bind(now)
    .bind(now)
    .bind(false) // Auto-generated title
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to create conversation: {}", e)))?;

    // Insert all the messages
    for (index, msg) in create_req.messages.iter().enumerate() {
        let message_id = Uuid::new_v4().to_string();
        let created_at = now + chrono::Duration::milliseconds(index as i64);

        // Insert the message
        sqlx::query(
            "INSERT INTO messages (id, conversation_id, content, role, created_at, is_forgotten) 
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&message_id)
        .bind(&conversation_id)
        .bind(&msg.content)
        .bind(&msg.role)
        .bind(created_at)
        .bind(false)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to insert message: {}", e)))?;

        // If there are file attachments, clone them
        if let Some(attachments) = &msg.file_attachments {
            for attachment in attachments {
                if let Some(obj) = attachment.as_object() {
                    // Extract file attachment details
                    if let (Some(original_name), Some(file_size)) = (
                        obj.get("original_name").and_then(|v| v.as_str()),
                        obj.get("file_size").and_then(|v| v.as_i64()),
                    ) {
                        let attachment_id = Uuid::new_v4().to_string();
                        let mime_type = obj.get("mime_type").and_then(|v| v.as_str());
                        let description = obj.get("description").and_then(|v| v.as_str());
                        let auto_description = obj.get("auto_description").and_then(|v| v.as_str());
                        let file_path = obj.get("file_path").and_then(|v| v.as_str());

                        // Insert file attachment record
                        sqlx::query(
                            "INSERT INTO file_attachments (id, message_id, file_name, original_name, file_path, file_size, mime_type, description, auto_description, created_at) 
                             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
                        )
                        .bind(&attachment_id)
                        .bind(&message_id)
                        .bind(original_name) // Using original_name as file_name for cloned messages
                        .bind(original_name)
                        .bind(file_path)
                        .bind(file_size as i32)
                        .bind(mime_type)
                        .bind(description)
                        .bind(auto_description)
                        .bind(now)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| AppError::InternalServerError(format!("Failed to clone file attachment: {}", e)))?;
                    }
                }
            }
        }
    }

    // Commit the transaction
    tx.commit().await.map_err(|e| {
        AppError::InternalServerError(format!("Failed to commit transaction: {}", e))
    })?;

    // Return the new conversation details
    let response = serde_json::json!({
        "conversation_id": conversation_id,
        "project_id": create_req.project_id,
        "title": generated_title,
        "message_count": create_req.messages.len()
    });

    res.render(Json(response));
    Ok(())
}

#[handler]
pub async fn update_conversation(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let conversation_id = req
        .param::<String>("conversation_id")
        .ok_or(AppError::BadRequest("Missing conversation_id".to_string()))?;

    let update_req: UpdateConversationRequest = req
        .parse_json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;

    let now = Utc::now();

    // Update in database and mark as manually set if title is provided
    if update_req.title.is_some() {
        sqlx::query(
            "UPDATE conversations 
             SET title = $1, is_title_manually_set = true, updated_at = $2 
             WHERE id = $3",
        )
        .bind(&update_req.title)
        .bind(now)
        .bind(&conversation_id)
    } else {
        sqlx::query(
            "UPDATE conversations 
             SET updated_at = $1 
             WHERE id = $2",
        )
        .bind(now)
        .bind(&conversation_id)
    }
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    // Fetch updated conversation - calculate actual message count excluding forgotten messages
    let updated = sqlx::query(
        "SELECT 
            c.id, 
            c.project_id, 
            c.title, 
            (
                SELECT COUNT(*)::INTEGER 
                FROM messages m 
                WHERE m.conversation_id = c.id
                AND (m.is_forgotten = false OR m.is_forgotten IS NULL)
            ) AS message_count,
            c.created_at, 
            c.updated_at, 
            c.is_title_manually_set 
         FROM conversations c
         WHERE c.id = $1",
    )
    .bind(&conversation_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let conversation = Conversation {
        id: updated
            .try_get("id")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get id: {}", e)))?,
        project_id: updated.try_get("project_id").map_err(|e| {
            AppError::InternalServerError(format!("Failed to get project_id: {}", e))
        })?,
        title: updated.try_get("title").ok(),
        created_at: updated.try_get("created_at").map_err(|e| {
            AppError::InternalServerError(format!("Failed to get created_at: {}", e))
        })?,
        updated_at: updated.try_get("updated_at").map_err(|e| {
            AppError::InternalServerError(format!("Failed to get updated_at: {}", e))
        })?,
        message_count: updated.try_get("message_count").map_err(|e| {
            AppError::InternalServerError(format!("Failed to get message_count: {}", e))
        })?,
        is_title_manually_set: updated.try_get("is_title_manually_set").ok(),
    };

    res.render(Json(conversation));
    Ok(())
}

#[handler]
pub async fn delete_conversation(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let conversation_id = req
        .param::<String>("conversation_id")
        .ok_or(AppError::BadRequest("Missing conversation_id".to_string()))?;

    // Delete from database (messages will cascade delete)
    sqlx::query("DELETE FROM conversations WHERE id = $1")
        .bind(&conversation_id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    res.render(Json(serde_json::json!({
        "success": true,
        "deleted_id": conversation_id
    })));
    Ok(())
}
