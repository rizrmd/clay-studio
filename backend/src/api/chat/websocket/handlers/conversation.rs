use crate::utils::AppState;
use sqlx::Row;

// WebSocket conversation management handlers
pub async fn handle_create_conversation(
    project_id: &str,
    title: Option<String>,
    _first_message: Option<String>, // Unused - handled in websocket mod.rs
    _file_ids: Option<Vec<String>>, // Unused - handled in websocket mod.rs
    client_id_str: &str,
    user_id: &str,
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

    // Parse user_id to UUID
    let user_uuid = uuid::Uuid::parse_str(user_id)
        .map_err(|_| crate::utils::AppError::BadRequest("Invalid user ID".to_string()))?;

    // Insert new conversation
    sqlx::query!(
        r#"
        INSERT INTO conversations (id, project_id, title, created_at, updated_at, is_title_manually_set, created_by_user_id, visibility)
        VALUES ($1, $2, $3, NOW(), NOW(), $4, $5, $6)
        "#,
        conversation_id,
        project_id,
        title,
        title.is_some(), // Set manually if title was provided
        user_uuid,
        "private" // Set as private by default
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Failed to create conversation: {}", e)))?;

    // Message handling is done in mod.rs after conversation is created
    let message_count = 0;

    // Return the created conversation
    let is_title_set = title.is_some();

    Ok(crate::models::Conversation {
        id: conversation_id,
        project_id: project_id.to_string(),
        title,
        created_at: now,
        updated_at: now,
        message_count,
        is_title_manually_set: Some(is_title_set),
        created_by_user_id: Some(user_uuid),
        visibility: Some(crate::models::ConversationVisibility::Private),
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
            created_by_user_id: None,
            visibility: Some(crate::models::ConversationVisibility::Private),
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
        created_by_user_id: None,
        visibility: Some(crate::models::ConversationVisibility::Private),
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

    // Optimized: Update and fetch in a single query using RETURNING and CTE
    let updated = if title.is_some() {
        sqlx::query(
            "WITH updated_conv AS (
                UPDATE conversations c
                SET title = $1, is_title_manually_set = true, updated_at = $2
                FROM projects p
                WHERE c.id = $3 AND c.project_id = p.id AND p.client_id = $4
                RETURNING c.id, c.project_id, c.title, c.created_at, c.updated_at, c.is_title_manually_set
            )
            SELECT
                uc.id,
                uc.project_id,
                uc.title,
                (
                    SELECT COUNT(*)::INTEGER
                    FROM messages m
                    WHERE m.conversation_id = uc.id
                    AND (m.is_forgotten = false OR m.is_forgotten IS NULL)
                ) AS message_count,
                uc.created_at,
                uc.updated_at,
                uc.is_title_manually_set
            FROM updated_conv uc",
        )
        .bind(&title)
        .bind(now)
        .bind(conversation_id)
        .bind(client_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?
    } else {
        sqlx::query(
            "WITH updated_conv AS (
                UPDATE conversations c
                SET updated_at = $1
                FROM projects p
                WHERE c.id = $2 AND c.project_id = p.id AND p.client_id = $3
                RETURNING c.id, c.project_id, c.title, c.created_at, c.updated_at, c.is_title_manually_set
            )
            SELECT
                uc.id,
                uc.project_id,
                uc.title,
                (
                    SELECT COUNT(*)::INTEGER
                    FROM messages m
                    WHERE m.conversation_id = uc.id
                    AND (m.is_forgotten = false OR m.is_forgotten IS NULL)
                ) AS message_count,
                uc.created_at,
                uc.updated_at,
                uc.is_title_manually_set
            FROM updated_conv uc",
        )
        .bind(now)
        .bind(conversation_id)
        .bind(client_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?
    };

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
        created_by_user_id: None,
        visibility: Some(crate::models::ConversationVisibility::Private),
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

    // First check if the conversation exists and the user has access to it
    let conversation_exists = sqlx::query(
        "SELECT c.id
         FROM conversations c
         JOIN projects p ON c.project_id = p.id
         WHERE c.id = $1 AND p.client_id = $2"
    )
    .bind(conversation_id)
    .bind(client_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?;

    if conversation_exists.is_none() {
        return Err(crate::utils::AppError::NotFound(format!(
            "Conversation {} not found or access denied",
            conversation_id
        )));
    }

    // Try to get from cache first
    match state.get_conversation_messages(conversation_id).await {
        Ok(messages) => {
            tracing::info!(
                "📦 Returning {} messages from cache for conversation {}",
                messages.len(),
                conversation_id
            );
            for msg in &messages {
                tracing::debug!(
                    "  - {} message: id={}, content_len={}, progress_content_len={}, processing_time={:?}",
                    match msg.role {
                        crate::models::MessageRole::User => "User",
                        crate::models::MessageRole::Assistant => "Assistant",
                        crate::models::MessageRole::System => "System",
                    },
                    &msg.id[..8],
                    msg.content.len(),
                    msg.progress_content.as_ref().map(|p| p.len()).unwrap_or(0),
                    msg.processing_time_ms
                );
            }
            Ok(messages)
        },
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
                    m.progress_content,
                    COALESCE(
                        JSON_AGG(
                            JSON_BUILD_OBJECT(
                                'id', tu.id,
                                'message_id', tu.message_id,
                                'tool_name', tu.tool_name,
                                'tool_use_id', tu.tool_use_id,
                                'parameters', tu.parameters,
                                'output', tu.output,
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
                GROUP BY m.id, m.content, m.role, m.processing_time_ms, m.created_at, m.file_attachments, m.progress_content
                ORDER BY m.created_at ASC"
            )
            .bind(conversation_id)
            .bind(client_id)
            .fetch_all(&state.db_pool)
            .await
            .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?;

            tracing::info!(
                "💾 Fetched {} messages from database for conversation {}",
                message_rows.len(),
                conversation_id
            );

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
                        .and_then(|v| {
                            // Manually parse the tool_usages array because the id field is a string in JSON
                            // but Uuid in the struct
                            if let serde_json::Value::Array(arr) = v {
                                let mut usages = Vec::new();
                                for item in arr {
                                    if let serde_json::Value::Object(obj) = item {
                                        // Parse UUID from string
                                        let id = obj.get("id")
                                            .and_then(|v| v.as_str())
                                            .and_then(|s| uuid::Uuid::parse_str(s).ok())
                                            .unwrap_or_else(uuid::Uuid::new_v4);
                                        
                                        usages.push(crate::models::ToolUsage {
                                            id,
                                            message_id: obj.get("message_id")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or_default()
                                                .to_string(),
                                            tool_name: obj.get("tool_name")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or_default()
                                                .to_string(),
                                            tool_use_id: obj.get("tool_use_id")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                            parameters: obj.get("parameters")
                                                .cloned(),
                                            output: obj.get("output")
                                                .cloned(),
                                            execution_time_ms: obj.get("execution_time_ms")
                                                .and_then(|v| v.as_i64()),
                                            created_at: obj.get("createdAt")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                        });
                                    }
                                }
                                if !usages.is_empty() {
                                    Some(usages)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }),
                    progress_content: row.try_get("progress_content").ok(),
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