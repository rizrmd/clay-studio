use crate::utils::AppState;
use sqlx::Row;

// WebSocket conversation management handlers
pub async fn handle_create_conversation(
    project_id: &str,
    title: Option<String>,
    client_id_str: &str,
    state: &AppState,
) -> Result<crate::models::Conversation, crate::utils::AppError> {
    let client_id = uuid::Uuid::parse_str(client_id_str)
        .map_err(|_| crate::utils::AppError::BadRequest("Invalid client ID".to_string()))?;

    // Verify project exists and belongs to client
    let project_exists = sqlx::query!(
        "SELECT id FROM projects WHERE id = $1 AND client_id = $2",
        project_id,
        client_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?;

    if project_exists.is_none() {
        return Err(crate::utils::AppError::NotFound(format!(
            "Project {} not found or access denied",
            project_id
        )));
    }

    // Generate new conversation ID
    let conversation_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    // Insert new conversation
    sqlx::query!(
        r#"
        INSERT INTO conversations (id, project_id, title, created_at, updated_at, is_title_manually_set)
        VALUES ($1, $2, $3, NOW(), NOW(), $4)
        "#,
        conversation_id,
        project_id,
        title,
        title.is_some() // Set manually if title was provided
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Failed to create conversation: {}", e)))?;

    // Return the created conversation
    let is_title_set = title.is_some();
    Ok(crate::models::Conversation {
        id: conversation_id,
        project_id: project_id.to_string(),
        title,
        created_at: now,
        updated_at: now,
        message_count: 0, // New conversation has no messages
        is_title_manually_set: Some(is_title_set),
    })
}

pub async fn handle_list_conversations(
    project_id: &str,
    client_id_str: &str,
    state: &AppState,
) -> Result<Vec<crate::models::Conversation>, crate::utils::AppError> {
    let client_id = uuid::Uuid::parse_str(client_id_str)
        .map_err(|_| crate::utils::AppError::BadRequest("Invalid client ID".to_string()))?;

    let conversations = sqlx::query(
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
         WHERE c.project_id = $1 AND p.client_id = $2
         ORDER BY c.created_at DESC 
         LIMIT 100",
    )
    .bind(project_id)
    .bind(client_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?;

    let mut conversation_list = Vec::new();
    for row in conversations {
        conversation_list.push(crate::models::Conversation {
            id: row.try_get("id").map_err(|e| {
                crate::utils::AppError::InternalServerError(format!("Failed to get id: {}", e))
            })?,
            project_id: row.try_get("project_id").map_err(|e| {
                crate::utils::AppError::InternalServerError(format!(
                    "Failed to get project_id: {}",
                    e
                ))
            })?,
            title: row.try_get("title").ok(),
            created_at: row.try_get("created_at").map_err(|e| {
                crate::utils::AppError::InternalServerError(format!(
                    "Failed to get created_at: {}",
                    e
                ))
            })?,
            updated_at: row.try_get("updated_at").map_err(|e| {
                crate::utils::AppError::InternalServerError(format!(
                    "Failed to get updated_at: {}",
                    e
                ))
            })?,
            message_count: row.try_get("message_count").map_err(|e| {
                crate::utils::AppError::InternalServerError(format!(
                    "Failed to get message_count: {}",
                    e
                ))
            })?,
            is_title_manually_set: row.try_get("is_title_manually_set").ok(),
        });
    }

    Ok(conversation_list)
}

pub async fn handle_get_conversation(
    conversation_id: &str,
    client_id_str: &str,
    state: &AppState,
) -> Result<crate::models::Conversation, crate::utils::AppError> {
    let client_id = uuid::Uuid::parse_str(client_id_str)
        .map_err(|_| crate::utils::AppError::BadRequest("Invalid client ID".to_string()))?;

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
         JOIN projects p ON c.project_id = p.id
         WHERE c.id = $1 AND p.client_id = $2",
    )
    .bind(conversation_id)
    .bind(client_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?
    .ok_or(crate::utils::AppError::NotFound(format!(
        "Conversation {} not found or access denied",
        conversation_id
    )))?;

    Ok(crate::models::Conversation {
        id: conversation_row.try_get("id").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!("Failed to get id: {}", e))
        })?,
        project_id: conversation_row.try_get("project_id").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!("Failed to get project_id: {}", e))
        })?,
        title: conversation_row.try_get("title").ok(),
        created_at: conversation_row.try_get("created_at").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!("Failed to get created_at: {}", e))
        })?,
        updated_at: conversation_row.try_get("updated_at").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!("Failed to get updated_at: {}", e))
        })?,
        message_count: conversation_row.try_get("message_count").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!(
                "Failed to get message_count: {}",
                e
            ))
        })?,
        is_title_manually_set: conversation_row.try_get("is_title_manually_set").ok(),
    })
}

pub async fn handle_update_conversation(
    conversation_id: &str,
    title: Option<String>,
    client_id_str: &str,
    state: &AppState,
) -> Result<crate::models::Conversation, crate::utils::AppError> {
    let client_id = uuid::Uuid::parse_str(client_id_str)
        .map_err(|_| crate::utils::AppError::BadRequest("Invalid client ID".to_string()))?;

    let now = chrono::Utc::now();

    // Update in database and mark as manually set if title is provided
    // Include authorization check
    if title.is_some() {
        sqlx::query(
            "UPDATE conversations 
             SET title = $1, is_title_manually_set = true, updated_at = $2 
             FROM projects p
             WHERE conversations.id = $3 AND conversations.project_id = p.id AND p.client_id = $4",
        )
        .bind(&title)
        .bind(now)
        .bind(conversation_id)
        .bind(client_id)
    } else {
        sqlx::query(
            "UPDATE conversations 
             SET updated_at = $1 
             FROM projects p
             WHERE conversations.id = $2 AND conversations.project_id = p.id AND p.client_id = $3",
        )
        .bind(now)
        .bind(conversation_id)
        .bind(client_id)
    }
    .execute(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?;

    // Fetch updated conversation
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
    .bind(conversation_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?;

    Ok(crate::models::Conversation {
        id: updated.try_get("id").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!("Failed to get id: {}", e))
        })?,
        project_id: updated.try_get("project_id").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!("Failed to get project_id: {}", e))
        })?,
        title: updated.try_get("title").ok(),
        created_at: updated.try_get("created_at").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!("Failed to get created_at: {}", e))
        })?,
        updated_at: updated.try_get("updated_at").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!("Failed to get updated_at: {}", e))
        })?,
        message_count: updated.try_get("message_count").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!(
                "Failed to get message_count: {}",
                e
            ))
        })?,
        is_title_manually_set: updated.try_get("is_title_manually_set").ok(),
    })
}

pub async fn handle_delete_conversation(
    conversation_id: &str,
    client_id_str: &str,
    state: &AppState,
) -> Result<(), crate::utils::AppError> {
    let client_id = uuid::Uuid::parse_str(client_id_str)
        .map_err(|_| crate::utils::AppError::BadRequest("Invalid client ID".to_string()))?;
    // Delete from database with authorization check (messages will cascade delete)
    let result = sqlx::query(
        "DELETE FROM conversations 
         USING projects p 
         WHERE conversations.id = $1 AND conversations.project_id = p.id AND p.client_id = $2",
    )
    .bind(conversation_id)
    .bind(client_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?;

    // Check if any rows were affected (conversation existed and was deleted)
    if result.rows_affected() == 0 {
        return Err(crate::utils::AppError::NotFound(format!(
            "Conversation {} not found or access denied",
            conversation_id
        )));
    }

    Ok(())
}

pub async fn handle_get_conversation_messages(
    conversation_id: &str,
    client_id_str: &str,
    state: &AppState,
) -> Result<Vec<crate::models::Message>, crate::utils::AppError> {
    let client_id = uuid::Uuid::parse_str(client_id_str)
        .map_err(|_| crate::utils::AppError::BadRequest("Invalid client ID".to_string()))?;
    // Try to get from cache first
    match state.get_conversation_messages(conversation_id).await {
        Ok(messages) => Ok(messages),
        Err(_) => {
            // Fall back to direct database query with authorization check
            let message_rows = sqlx::query(
                "SELECT 
                    m.id, 
                    m.content, 
                    m.role, 
                    m.processing_time_ms,
                    m.created_at,
                    m.file_attachments,
                    COALESCE(
                        JSON_AGG(
                            JSON_BUILD_OBJECT(
                                'id', tu.id,
                                'message_id', tu.message_id,
                                'tool_name', tu.tool_name,
                                'tool_use_id', tu.tool_use_id,
                                'execution_time_ms', tu.execution_time_ms,
                                'createdAt', tu.created_at
                            )
                        ) FILTER (WHERE tu.id IS NOT NULL),
                        '[]'::json
                    ) as tool_usages
                FROM messages m
                LEFT JOIN tool_usages tu ON m.id = tu.message_id
                JOIN conversations c ON m.conversation_id = c.id
                JOIN projects p ON c.project_id = p.id
                WHERE m.conversation_id = $1 AND p.client_id = $2
                AND (m.is_forgotten = false OR m.is_forgotten IS NULL)
                GROUP BY m.id, m.content, m.role, m.processing_time_ms, m.created_at, m.file_attachments
                ORDER BY m.created_at ASC"
            )
            .bind(conversation_id)
            .bind(client_id)
            .fetch_all(&state.db_pool)
            .await
            .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?;

            let mut messages = Vec::new();
            for row in message_rows {
                messages.push(crate::models::Message {
                    id: row.try_get("id").map_err(|e| {
                        crate::utils::AppError::InternalServerError(format!(
                            "Failed to get id: {}",
                            e
                        ))
                    })?,
                    content: row.try_get("content").map_err(|e| {
                        crate::utils::AppError::InternalServerError(format!(
                            "Failed to get content: {}",
                            e
                        ))
                    })?,
                    role: match row
                        .try_get::<String, _>("role")
                        .map_err(|e| {
                            crate::utils::AppError::InternalServerError(format!(
                                "Failed to get role: {}",
                                e
                            ))
                        })?
                        .as_str()
                    {
                        "user" => crate::models::MessageRole::User,
                        "assistant" => crate::models::MessageRole::Assistant,
                        "system" => crate::models::MessageRole::System,
                        _ => crate::models::MessageRole::User,
                    },
                    processing_time_ms: row.try_get("processing_time_ms").ok(),
                    created_at: row
                        .try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                        .ok()
                        .map(|dt| dt.to_rfc3339()),
                    file_attachments: row
                        .try_get::<Option<serde_json::Value>, _>("file_attachments")
                        .ok()
                        .flatten()
                        .and_then(|v| serde_json::from_value(v).ok()),
                    tool_usages: row
                        .try_get::<serde_json::Value, _>("tool_usages")
                        .ok()
                        .and_then(|v| serde_json::from_value(v).ok()),
                });
            }

            Ok(messages)
        }
    }
}

// Store ask_user response in the database
pub async fn store_ask_user_response(
    state: &AppState,
    conversation_id: &str,
    interaction_id: &str,
    response: &serde_json::Value,
) -> Result<(), crate::utils::AppError> {
    // For now, store in a simple JSON column in messages table
    // In production, you might want a dedicated interaction_responses table

    let response_json = serde_json::to_string(response).map_err(|e| {
        crate::utils::AppError::InternalServerError(format!("Failed to serialize response: {}", e))
    })?;

    // Store as a system message with the interaction response
    let message_content = format!(
        "User response to interaction {}:\n{}",
        interaction_id, response_json
    );

    sqlx::query!(
        r#"
        INSERT INTO messages (id, conversation_id, role, content, created_at)
        VALUES ($1, $2, 'system', $3, NOW())
        "#,
        uuid::Uuid::new_v4().to_string(),
        conversation_id,
        message_content
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?;

    Ok(())
}