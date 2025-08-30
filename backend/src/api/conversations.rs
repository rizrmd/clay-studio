use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use crate::models::*;
use crate::models::tool_usage::ToolUsage;
use crate::utils::AppState;
use crate::utils::AppError;
use crate::utils::middleware::{get_current_client_id, is_current_user_root};
use crate::core::tools::ToolApplicabilityChecker;
use chrono::Utc;
use uuid::Uuid;
use sqlx::Row;
use serde_json;


#[handler]
pub async fn get_conversation_context(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let conversation_id = req.param::<String>("conversation_id")
        .ok_or(AppError::BadRequest("Missing conversation_id".to_string()))?;

    // Fetch conversation from database - calculate actual message count excluding forgotten messages
    let conversation = sqlx::query_as::<_, (String, String, Option<String>, i32, Option<bool>)>(
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
            c.is_title_manually_set 
         FROM conversations c
         WHERE c.id = $1"
    )
    .bind(&conversation_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    .ok_or(AppError::NotFound(format!("Conversation {} not found", conversation_id)))?;

    let project_id = conversation.1;

    // Fetch messages from database (excluding forgotten ones)
    let messages = sqlx::query(
        "SELECT id, content, role, created_at FROM messages 
         WHERE conversation_id = $1 
         AND (is_forgotten = false OR is_forgotten IS NULL)
         ORDER BY created_at ASC 
         LIMIT 50"
    )
    .bind(&conversation_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let mut message_list = Vec::new();
    for row in messages {
        let message_id: String = row.try_get("id")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get id: {}", e)))?;
        let role: String = row.try_get("role")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get role: {}", e)))?;
        let content: String = row.try_get("content")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get content: {}", e)))?;
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get created_at: {}", e)))?;
        
        // Fetch tool usages for this message
        let tool_usages = sqlx::query(
            "SELECT id, tool_name, parameters, output, execution_time_ms, created_at
             FROM tool_usages 
             WHERE message_id = $1
             ORDER BY created_at ASC"
        )
        .bind(&message_id)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error fetching tool_usages: {}", e)))?;
        
        let mut tool_usage_list = Vec::new();
        for tool_row in tool_usages {
            let tool_usage = ToolUsage {
                id: tool_row.try_get::<uuid::Uuid, _>("id")
                    .map_err(|e| AppError::InternalServerError(format!("Failed to get tool_usage id: {}", e)))?,
                message_id: message_id.clone(),
                tool_name: tool_row.try_get("tool_name")
                    .map_err(|e| AppError::InternalServerError(format!("Failed to get tool_name: {}", e)))?,
                tool_use_id: tool_row.try_get("tool_use_id").ok(),
                parameters: tool_row.try_get("parameters").ok(),
                output: tool_row.try_get("output").ok(),
                execution_time_ms: tool_row.try_get("execution_time_ms").ok(),
                created_at: tool_row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                    .map(|dt| dt.to_rfc3339())
                    .ok(),
            };
            tool_usage_list.push(tool_usage);
        }
        
        let mut msg = match role.as_str() {
            "user" => Message::new_user(content),
            "assistant" => Message::new_assistant(content),
            _ => continue,
        };
        
        // Set the message ID and created_at
        msg.id = message_id;
        msg.created_at = Some(created_at.to_rfc3339());
        
        // Add tool usages if any exist
        if !tool_usage_list.is_empty() {
            msg.tool_usages = Some(tool_usage_list);
        }
        
        message_list.push(msg);
    }

    // Fetch data sources for the project
    let data_sources = sqlx::query(
        "SELECT id, name, source_type, connection_config, schema_info, 
         preview_data, table_list, last_tested_at, is_active 
         FROM data_sources WHERE project_id = $1"
    )
    .bind(&project_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let mut data_source_list = Vec::new();
    for row in data_sources {
        data_source_list.push(DataSourceContext {
            id: row.try_get("id")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get id: {}", e)))?,
            name: row.try_get("name")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get name: {}", e)))?,
            source_type: row.try_get("source_type")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get source_type: {}", e)))?,
            connection_config: row.try_get("connection_config")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get connection_config: {}", e)))?,
            schema_info: row.try_get("schema_info").ok(),
            preview_data: row.try_get("preview_data").ok(),
            table_list: row.try_get("table_list").ok(),
            last_tested_at: row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_tested_at")
                .map(|dt_opt| dt_opt.map(|dt| dt.to_rfc3339()))
                .unwrap_or(None),
            is_active: row.try_get("is_active")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get is_active: {}", e)))?,
        });
    }

    // If no data sources exist, use empty list
    if data_source_list.is_empty() {
        data_source_list = vec![];
    }

    // Determine applicable tools based on data sources
    let available_tools = ToolApplicabilityChecker::determine_applicable_tools(&data_source_list);

    // Fetch project settings
    let project_settings = sqlx::query_as::<_, (String, String, Option<serde_json::Value>, Option<serde_json::Value>)>(
        "SELECT id, name, settings, organization_settings FROM projects WHERE id = $1"
    )
    .bind(&project_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    .map(|(id, name, settings, org_settings)| ProjectSettings {
        project_id: id,
        name,
        settings: settings.unwrap_or(serde_json::json!({})),
        organization_settings: org_settings.unwrap_or(serde_json::json!({})),
        default_analysis_preferences: AnalysisPreferences::default(),
    })
    .unwrap_or_else(|| ProjectSettings {
        project_id: project_id.clone(),
        name: "Unknown Project".to_string(),
        settings: serde_json::json!({}),
        organization_settings: serde_json::json!({}),
        default_analysis_preferences: AnalysisPreferences::default(),
    });

    let context = ConversationContext {
        conversation_id: conversation_id.clone(),
        project_id,
        messages: message_list,
        summary: None,
        data_sources: data_source_list,
        available_tools,
        project_settings,
        total_messages: conversation.3,
        context_strategy: ContextStrategy::FullHistory,
    };

    res.render(Json(context));
    Ok(())
}

#[handler]
pub async fn list_conversations(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    
    // Get current user's client_id for filtering
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
                 LIMIT 100"
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
                 JOIN projects p ON c.project_id = p.id
                 WHERE c.project_id = $1 AND p.client_id = $2
                 ORDER BY c.created_at DESC 
                 LIMIT 100"
            )
            .bind(pid)
            .bind(client_id)
            .fetch_all(&state.db_pool)
            .await
        }
    } else {
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
                 ORDER BY c.created_at DESC 
                 LIMIT 100"
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
                 WHERE p.client_id = $1
                 ORDER BY c.created_at DESC 
                 LIMIT 100"
            )
            .bind(client_id)
            .fetch_all(&state.db_pool)
            .await
        }
    }
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let mut conversation_list = Vec::new();
    for row in conversations {
        let id: String = row.try_get("id")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get id: {}", e)))?;
        let message_count: i32 = row.try_get("message_count")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get message_count: {}", e)))?;
        
        // Debug logging
        tracing::debug!("Conversation {} has message_count: {}", id, message_count);
        
        conversation_list.push(Conversation {
            id,
            project_id: row.try_get("project_id")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get project_id: {}", e)))?,
            title: row.try_get("title").ok(),
            created_at: row.try_get("created_at")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get created_at: {}", e)))?,
            updated_at: row.try_get("updated_at")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get updated_at: {}", e)))?,
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
    let state = depot.obtain::<AppState>().unwrap();
    let conversation_id = req.param::<String>("conversation_id")
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
         WHERE c.id = $1"
    )
    .bind(&conversation_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    .ok_or(AppError::NotFound(format!("Conversation {} not found", conversation_id)))?;
    
    let conversation = Conversation {
        id: conversation_row.try_get("id")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get id: {}", e)))?,
        project_id: conversation_row.try_get("project_id")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get project_id: {}", e)))?,
        title: conversation_row.try_get("title").ok(),
        created_at: conversation_row.try_get("created_at")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get created_at: {}", e)))?,
        updated_at: conversation_row.try_get("updated_at")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get updated_at: {}", e)))?,
        message_count: conversation_row.try_get("message_count")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get message_count: {}", e)))?,
        is_title_manually_set: conversation_row.try_get("is_title_manually_set").ok(),
    };

    res.render(Json(conversation));
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    pub project_id: String,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateFromMessageRequest {
    pub project_id: String,
    pub source_conversation_id: Option<String>,
    pub message_id: String,
    pub messages: Vec<MessageForClone>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageForClone {
    pub id: String,
    pub content: String,
    pub role: String,
    pub file_attachments: Option<Vec<serde_json::Value>>,
}

#[handler]
pub async fn create_conversation(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let create_req: CreateConversationRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;
    
    let conversation_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    
    // Use provided title or None (will be set from first message)
    let title = create_req.title.as_ref()
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
    let state = depot.obtain::<AppState>().unwrap();
    let create_req: CreateFromMessageRequest = req.parse_json().await
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
    let mut tx = state.db_pool.begin().await
        .map_err(|e| AppError::InternalServerError(format!("Failed to start transaction: {}", e)))?;
    
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
             VALUES ($1, $2, $3, $4, $5, $6)"
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
                    if let (Some(original_name), Some(file_size)) = 
                        (obj.get("original_name").and_then(|v| v.as_str()),
                         obj.get("file_size").and_then(|v| v.as_i64())) {
                        
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
    tx.commit().await
        .map_err(|e| AppError::InternalServerError(format!("Failed to commit transaction: {}", e)))?;
    
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

#[derive(Debug, Deserialize)]
pub struct UpdateConversationRequest {
    pub title: Option<String>,
}

#[handler]
pub async fn update_conversation(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let conversation_id = req.param::<String>("conversation_id")
        .ok_or(AppError::BadRequest("Missing conversation_id".to_string()))?;
    
    let update_req: UpdateConversationRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;
    
    let now = Utc::now();
    
    // Update in database and mark as manually set if title is provided
    if update_req.title.is_some() {
        sqlx::query(
            "UPDATE conversations 
             SET title = $1, is_title_manually_set = true, updated_at = $2 
             WHERE id = $3"
        )
        .bind(&update_req.title)
        .bind(now)
        .bind(&conversation_id)
    } else {
        sqlx::query(
            "UPDATE conversations 
             SET updated_at = $1 
             WHERE id = $2"
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
         WHERE c.id = $1"
    )
    .bind(&conversation_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let conversation = Conversation {
        id: updated.try_get("id")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get id: {}", e)))?,
        project_id: updated.try_get("project_id")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get project_id: {}", e)))?,
        title: updated.try_get("title").ok(),
        created_at: updated.try_get("created_at")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get created_at: {}", e)))?,
        updated_at: updated.try_get("updated_at")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get updated_at: {}", e)))?,
        message_count: updated.try_get("message_count")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get message_count: {}", e)))?,
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
    let state = depot.obtain::<AppState>().unwrap();
    let conversation_id = req.param::<String>("conversation_id")
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

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub content: String,
    pub role: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub processing_time_ms: Option<i64>,
    pub tool_usages: Option<Vec<crate::models::tool_usage::ToolUsage>>,
}

#[handler]
pub async fn get_conversation_messages(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let conversation_id = req.param::<String>("conversation_id")
        .ok_or(AppError::BadRequest("Missing conversation_id".to_string()))?;
    
    // Fetch messages from database first, filtering out forgotten ones
    // Order by created_at and then by id to ensure stable ordering
    let messages = sqlx::query(
        "SELECT id, content, role, processing_time_ms, created_at 
         FROM messages 
         WHERE conversation_id = $1 
         AND (is_forgotten = false OR is_forgotten IS NULL)
         ORDER BY created_at ASC, id ASC"
    )
    .bind(&conversation_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    tracing::debug!("Found {} messages for conversation {}", messages.len(), conversation_id);
    
    let mut message_responses = Vec::new();
    for row in messages {
        let msg_id: String = row.try_get("id")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get id: {}", e)))?;
        
        // Fetch tool_usages for this message if it's an assistant message
        let msg_role: String = row.try_get("role")
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
            .map_err(|e| AppError::InternalServerError(format!("Failed to fetch tool usages: {}", e)))?;
            
            if !tool_usage_rows.is_empty() {
                let mut usages = Vec::new();
                for usage_row in tool_usage_rows {
                    let created_at: Option<chrono::DateTime<chrono::Utc>> = usage_row.try_get("created_at").ok();
                    usages.push(crate::models::tool_usage::ToolUsage {
                        id: usage_row.try_get("id").unwrap_or_default(),
                        message_id: usage_row.try_get("message_id").unwrap_or_default(),
                        tool_name: usage_row.try_get("tool_name").unwrap_or_default(),
                        tool_use_id: usage_row.try_get("tool_use_id").ok(),
                        parameters: usage_row.try_get("parameters").ok(),
                        output: usage_row.try_get("output").ok(),
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
        
        let msg_created_at = row.try_get::<chrono::DateTime<Utc>, _>("created_at")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get created_at: {}", e)))?;
        
        tracing::trace!("Message order: {} - {} at {}", &msg_id[..8], msg_role, msg_created_at);
        
        message_responses.push(MessageResponse {
            id: msg_id,
            content: row.try_get("content")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get content: {}", e)))?,
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


