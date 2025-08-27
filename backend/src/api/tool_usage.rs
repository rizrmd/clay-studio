use salvo::prelude::*;
use sqlx::Row;

use crate::{
    models::tool_usage::ToolUsage,
    utils::{error::AppError, state::AppState},
};

/// Get all tool usages for a message
#[handler]
pub async fn get_message_tool_usages(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let message_id = req.param::<String>("message_id").unwrap();
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
    let state = depot.obtain::<AppState>().unwrap();
    let message_id = req.param::<String>("message_id").unwrap();
    let tool_name = req.param::<String>("tool_name").unwrap();
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
pub async fn save_tool_usage(
    state: &AppState,
    tool_usage: &ToolUsage,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO tool_usages (id, message_id, tool_name, parameters, output, execution_time_ms, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (id) DO UPDATE SET
            parameters = EXCLUDED.parameters,
            output = EXCLUDED.output,
            execution_time_ms = EXCLUDED.execution_time_ms
        "#,
    )
    .bind(&tool_usage.id)
    .bind(&tool_usage.message_id)
    .bind(&tool_usage.tool_name)
    .bind(&tool_usage.parameters)
    .bind(&tool_usage.output)
    .bind(&tool_usage.execution_time_ms)
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