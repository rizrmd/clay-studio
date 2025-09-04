use crate::core::tools::ToolApplicabilityChecker;
use crate::models::tool_usage::ToolUsage;
use crate::models::*;
use crate::utils::AppError;
use crate::utils::AppState;
use salvo::prelude::*;
use sqlx::Row;

#[handler]
pub async fn get_conversation_context(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let conversation_id = req
        .param::<String>("conversation_id")
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

    let project_id = conversation.1;

    // Fetch messages from database (excluding forgotten ones)
    let messages = sqlx::query(
        "SELECT id, content, role, created_at FROM messages
         WHERE conversation_id = $1
         AND (is_forgotten = false OR is_forgotten IS NULL)
        ORDER BY created_at ASC
         LIMIT 50",
    )
    .bind(&conversation_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let mut message_list = Vec::new();
    for row in messages {
        let message_id: String = row
            .try_get("id")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get id: {}", e)))?;
        let role: String = row
            .try_get("role")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get role: {}", e)))?;
        let content: String = row
            .try_get("content")
            .map_err(|e| AppError::InternalServerError(format!("Failed to get content: {}", e)))?;
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at").map_err(|e| {
            AppError::InternalServerError(format!("Failed to get created_at: {}", e))
        })?;

        // Fetch tool usages for this message
        let tool_usages = sqlx::query(
            "SELECT id, tool_name, parameters, output, execution_time_ms, created_at
            FROM tool_usages
             WHERE message_id = $1
            ORDER BY created_at ASC",
        )
        .bind(&message_id)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| {
            AppError::InternalServerError(format!("Database error fetching tool_usages: {}", e))
        })?;

        let mut tool_usage_list = Vec::new();
        for tool_row in tool_usages {
            let tool_usage = ToolUsage {
                id: tool_row.try_get::<uuid::Uuid, _>("id").map_err(|e| {
                    AppError::InternalServerError(format!("Failed to get tool_usage id: {}", e))
                })?,
                message_id: message_id.clone(),
                tool_name: tool_row.try_get("tool_name").map_err(|e| {
                    AppError::InternalServerError(format!("Failed to get tool_name: {}", e))
                })?,
                tool_use_id: tool_row.try_get("tool_use_id").ok(),
                parameters: tool_row.try_get("parameters").ok(),
                output: tool_row.try_get("output").ok(),
                execution_time_ms: tool_row.try_get("execution_time_ms").ok(),
                created_at: tool_row
                    .try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
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
         FROM data_sources WHERE project_id = $1",
    )
    .bind(&project_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let mut data_source_list = Vec::new();
    for row in data_sources {
        data_source_list.push(DataSourceContext {
            id: row
                .try_get("id")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get id: {}", e)))?,
            name: row
                .try_get("name")
                .map_err(|e| AppError::InternalServerError(format!("Failed to get name: {}", e)))?,
            source_type: row.try_get("source_type").map_err(|e| {
                AppError::InternalServerError(format!("Failed to get source_type: {}", e))
            })?,
            connection_config: row.try_get("connection_config").map_err(|e| {
                AppError::InternalServerError(format!("Failed to get connection_config: {}", e))
            })?,
            schema_info: row.try_get("schema_info").ok(),
            preview_data: row.try_get("preview_data").ok(),
            table_list: row.try_get("table_list").ok(),
            last_tested_at: row
                .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_tested_at")
                .map(|dt_opt| dt_opt.map(|dt| dt.to_rfc3339()))
                .unwrap_or(None),
            is_active: row.try_get("is_active").map_err(|e| {
                AppError::InternalServerError(format!("Failed to get is_active: {}", e))
            })?,
        });
    }

    // If no data sources exist, use empty list
    if data_source_list.is_empty() {
        data_source_list = vec![];
    }

    // Determine applicable tools based on data sources
    let available_tools = ToolApplicabilityChecker::determine_applicable_tools(&data_source_list);

    // Fetch project settings
    let project_settings =
        sqlx::query_as::<
            _,
            (
                String,
                String,
                Option<serde_json::Value>,
                Option<serde_json::Value>,
            ),
        >("SELECT id, name, settings, organization_settings FROM projects WHERE id = $1")
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
