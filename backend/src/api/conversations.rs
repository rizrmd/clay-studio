use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use crate::models::*;
use crate::utils::AppState;
use crate::utils::AppError;
use crate::core::tools::ToolApplicabilityChecker;
use chrono::Utc;
use uuid::Uuid;
use sqlx::Row;


#[handler]
pub async fn get_conversation_context(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let conversation_id = req.param::<String>("conversation_id")
        .ok_or(AppError::BadRequest("Missing conversation_id".to_string()))?;

    // Fetch conversation from database
    let conversation = sqlx::query_as::<_, (String, String, Option<String>, i32, Option<bool>)>(
        "SELECT id, project_id, title, message_count, is_title_manually_set FROM conversations WHERE id = $1"
    )
    .bind(&conversation_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    .ok_or(AppError::NotFound(format!("Conversation {} not found", conversation_id)))?;

    let project_id = conversation.1;

    // Fetch messages from database
    let messages = sqlx::query(
        "SELECT id, content, role FROM messages 
         WHERE conversation_id = $1 
         ORDER BY created_at ASC 
         LIMIT 50"
    )
    .bind(&conversation_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let mut message_list = Vec::new();
    for row in messages {
        let role: String = row.try_get("role")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get role: {}", e)))?;
        let content: String = row.try_get("content")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get content: {}", e)))?;
        let msg = match role.as_str() {
            "user" => Message::new_user(content),
            "assistant" => Message::new_assistant(content),
            _ => continue,
        };
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
    
    // Optional project_id filter from query params
    let project_id = req.query::<String>("project_id");
    
    // Build query based on filters
    let conversations = if let Some(pid) = project_id {
        sqlx::query(
            "SELECT id, project_id, title, message_count, created_at, updated_at, is_title_manually_set 
             FROM conversations 
             WHERE project_id = $1 
             ORDER BY updated_at DESC 
             LIMIT 100"
        )
        .bind(pid)
        .fetch_all(&state.db_pool)
        .await
    } else {
        sqlx::query(
            "SELECT id, project_id, title, message_count, created_at, updated_at, is_title_manually_set 
             FROM conversations 
             ORDER BY updated_at DESC 
             LIMIT 100"
        )
        .fetch_all(&state.db_pool)
        .await
    }
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let mut conversation_list = Vec::new();
    for row in conversations {
        conversation_list.push(Conversation {
            id: row.try_get("id")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get id: {}", e)))?,
            project_id: row.try_get("project_id")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get project_id: {}", e)))?,
            title: row.try_get("title").ok(),
            created_at: row.try_get("created_at")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get created_at: {}", e)))?,
            updated_at: row.try_get("updated_at")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get updated_at: {}", e)))?,
            message_count: row.try_get("message_count")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get message_count: {}", e)))?,
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
        "SELECT id, project_id, title, message_count, created_at, updated_at, is_title_manually_set 
         FROM conversations 
         WHERE id = $1"
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
    
    // Fetch updated conversation
    let updated = sqlx::query(
        "SELECT id, project_id, title, message_count, created_at, updated_at, is_title_manually_set 
         FROM conversations WHERE id = $1"
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
    pub clay_tools_used: Option<Vec<String>>,
    pub processing_time_ms: Option<i64>,
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
    
    // First, check if there's a forgotten_after_message_id
    let forgotten_after = sqlx::query_scalar::<_, Option<String>>(
        "SELECT forgotten_after_message_id FROM conversations WHERE id = $1"
    )
    .bind(&conversation_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    // Fetch messages from database, filtering if there's a forgotten_after point
    let messages = if let Some(forgotten_after_id) = forgotten_after {
        sqlx::query(
            "SELECT m.id, m.content, m.role, m.clay_tools_used, m.processing_time_ms, m.created_at 
             FROM messages m
             WHERE m.conversation_id = $1 
             AND m.created_at <= (SELECT created_at FROM messages WHERE id = $2)
             ORDER BY m.created_at ASC"
        )
        .bind(&conversation_id)
        .bind(&forgotten_after_id)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    } else {
        sqlx::query(
            "SELECT id, content, role, clay_tools_used, processing_time_ms, created_at 
             FROM messages 
             WHERE conversation_id = $1 
             ORDER BY created_at ASC"
        )
        .bind(&conversation_id)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    };
    
    let mut message_responses = Vec::new();
    for row in messages {
        let tools_json: Option<serde_json::Value> = row.try_get("clay_tools_used").ok();
        let tools = tools_json.and_then(|v| {
            if v.is_array() {
                v.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|item| item.as_str().map(String::from))
                        .collect()
                })
            } else {
                None
            }
        });
        
        message_responses.push(MessageResponse {
            id: row.try_get("id")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get id: {}", e)))?,
            content: row.try_get("content")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get content: {}", e)))?,
            role: row.try_get("role")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get role: {}", e)))?,
            created_at: row.try_get::<chrono::DateTime<Utc>, _>("created_at")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get created_at: {}", e)))?
                .to_rfc3339(),
            clay_tools_used: tools,
            processing_time_ms: row.try_get("processing_time_ms").ok(),
        });
    }
    
    res.render(Json(message_responses));
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct ForgetMessagesRequest {
    pub message_id: String,
}

#[handler]
pub async fn forget_messages_after(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let conversation_id = req.param::<String>("conversation_id")
        .ok_or(AppError::BadRequest("Missing conversation_id".to_string()))?;
    
    let forget_request: ForgetMessagesRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;
    
    // Update the conversation to set forgotten_after_message_id
    sqlx::query(
        "UPDATE conversations 
         SET forgotten_after_message_id = $1, updated_at = NOW() 
         WHERE id = $2"
    )
    .bind(&forget_request.message_id)
    .bind(&conversation_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    // Get count of forgotten messages
    let count_result = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM messages 
         WHERE conversation_id = $1 
         AND created_at > (SELECT created_at FROM messages WHERE id = $2)"
    )
    .bind(&conversation_id)
    .bind(&forget_request.message_id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(0);
    
    res.render(Json(serde_json::json!({
        "success": true,
        "forgotten_count": count_result,
        "forgotten_after_message_id": forget_request.message_id
    })));
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
        .ok_or(AppError::BadRequest("Missing conversation_id".to_string()))?;
    
    // Clear the forgotten_after_message_id
    sqlx::query(
        "UPDATE conversations 
         SET forgotten_after_message_id = NULL, updated_at = NOW() 
         WHERE id = $1"
    )
    .bind(&conversation_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    res.render(Json(serde_json::json!({
        "success": true,
        "message": "All messages restored"
    })));
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
        .ok_or(AppError::BadRequest("Missing conversation_id".to_string()))?;
    
    // Get the forgotten_after_message_id from the conversation
    let forgotten_after = sqlx::query_scalar::<_, Option<String>>(
        "SELECT forgotten_after_message_id FROM conversations WHERE id = $1"
    )
    .bind(&conversation_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let mut response = serde_json::json!({
        "has_forgotten": forgotten_after.is_some(),
        "forgotten_after_message_id": forgotten_after
    });
    
    if let Some(message_id) = forgotten_after {
        // Get count of forgotten messages
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM messages 
             WHERE conversation_id = $1 
             AND created_at > (SELECT created_at FROM messages WHERE id = $2)"
        )
        .bind(&conversation_id)
        .bind(&message_id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(0);
        
        response["forgotten_count"] = serde_json::json!(count);
    }
    
    res.render(Json(response));
    Ok(())
}