use salvo::prelude::*;
use sqlx::Row;

use crate::{
    models::tool_usage::ToolUsage,
    utils::{get_app_state, error::AppError, state::AppState},
};

/// Get all tool usages for a message
#[handler]
pub async fn get_message_tool_usages(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let message_id = req.param::<String>("message_id")
        .ok_or(AppError::BadRequest("Missing message_id parameter".to_string()))?;
    let rows = sqlx::query(
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
    .bind(&message_id)
    .fetch_all(&state.db_pool)
    .await?;

    let tool_usages: Vec<ToolUsage> = rows
        .into_iter()
        .map(|row| {
            let created_at: Option<chrono::DateTime<chrono::Utc>> = row.get("created_at");
            ToolUsage {
                id: row.get("id"),
                message_id: row.get("message_id"),
                tool_name: row.get("tool_name"),
                tool_use_id: row.get("tool_use_id"),
                parameters: row.get("parameters"),
                output: row.get("output"),
                execution_time_ms: row.get("execution_time_ms"),
                created_at: created_at.map(|dt| dt.to_rfc3339()),
            }
        })
        .collect();

    res.render(Json(tool_usages));
    Ok(())
}

/// Get specific tool usage by message ID and tool name
#[handler]
pub async fn get_tool_usage_by_name(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let message_id = req.param::<String>("message_id")
        .ok_or(AppError::BadRequest("Missing message_id parameter".to_string()))?;
    let tool_name = req.param::<String>("tool_name")
        .ok_or(AppError::BadRequest("Missing tool_name parameter".to_string()))?;
    let row = sqlx::query(
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
        WHERE message_id = $1 AND tool_name = $2
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(&message_id)
    .bind(&tool_name)
    .fetch_optional(&state.db_pool)
    .await?;

    match row {
        Some(row) => {
            let created_at: Option<chrono::DateTime<chrono::Utc>> = row.get("created_at");
            let tool_usage = ToolUsage {
                id: row.get("id"),
                message_id: row.get("message_id"),
                tool_name: row.get("tool_name"),
                tool_use_id: row.get("tool_use_id"),
                parameters: row.get("parameters"),
                output: row.get("output"),
                execution_time_ms: row.get("execution_time_ms"),
                created_at: created_at.map(|dt| dt.to_rfc3339()),
            };
            res.render(Json(tool_usage));
            Ok(())
        }
        None => Err(AppError::NotFound("Tool usage not found".to_string())),
    }
}

/// Get tool usage details by ID
#[handler]
pub async fn get_tool_usage_by_id(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let tool_usage_id_str = req.param::<String>("tool_usage_id")
        .ok_or(AppError::BadRequest("Missing tool_usage_id parameter".to_string()))?;
    
    tracing::info!("Fetching tool usage with ID: {}", tool_usage_id_str);
    
    let tool_usage_id = uuid::Uuid::parse_str(&tool_usage_id_str)
        .map_err(|e| {
            tracing::error!("Failed to parse tool usage ID '{}': {}", tool_usage_id_str, e);
            AppError::BadRequest("Invalid tool usage ID format".to_string())
        })?;
    
    // Get current client ID from depot (set by client_scoped middleware)
    let current_client_id = depot.get::<String>("current_client_id")
        .map_err(|_| AppError::Unauthorized("Client context required".to_string()))?;
    
    let current_client_uuid = uuid::Uuid::parse_str(current_client_id)
        .map_err(|_| AppError::InternalServerError("Invalid client ID in context".to_string()))?;

    // Query tool usage with client authorization
    let row = sqlx::query(
        r#"
        SELECT 
            tu.id,
            tu.message_id,
            tu.tool_name,
            tu.tool_use_id,
            tu.parameters,
            tu.output,
            tu.execution_time_ms,
            tu.created_at
        FROM tool_usages tu
        JOIN messages m ON tu.message_id = m.id
        JOIN conversations c ON m.conversation_id = c.id
        JOIN projects p ON c.project_id = p.id
        WHERE tu.id = $1 AND p.client_id = $2
        "#,
    )
    .bind(tool_usage_id)
    .bind(current_client_uuid)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error when fetching tool usage: {}", e);
        AppError::InternalServerError(format!("Database error: {}", e))
    })?;

    match row {
        Some(row) => {
            let created_at: Option<chrono::DateTime<chrono::Utc>> = row.get("created_at");
            let tool_usage = ToolUsage {
                id: row.get("id"),
                message_id: row.get("message_id"),
                tool_name: row.get("tool_name"),
                tool_use_id: row.get("tool_use_id"),
                parameters: row.get("parameters"),
                output: row.get("output"),
                execution_time_ms: row.get("execution_time_ms"),
                created_at: created_at.map(|dt| dt.to_rfc3339()),
            };
            res.render(Json(tool_usage));
            Ok(())
        }
        None => Err(AppError::NotFound("Tool usage not found".to_string())),
    }
}

/// Save tool usage data
#[allow(dead_code)]
pub async fn save_tool_usage(state: &AppState, tool_usage: &ToolUsage) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO tool_usages (id, message_id, tool_name, tool_use_id, parameters, output, execution_time_ms, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (id) DO UPDATE SET
            tool_use_id = EXCLUDED.tool_use_id,
            parameters = EXCLUDED.parameters,
            output = EXCLUDED.output,
            execution_time_ms = EXCLUDED.execution_time_ms
        "#,
    )
    .bind(tool_usage.id)
    .bind(&tool_usage.message_id)
    .bind(&tool_usage.tool_name)
    .bind(&tool_usage.tool_use_id)
    .bind(&tool_usage.parameters)
    .bind(&tool_usage.output)
    .bind(tool_usage.execution_time_ms)
    .bind(
        tool_usage.created_at
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
    )
    .execute(&state.db_pool)
    .await?;

    Ok(())
}
