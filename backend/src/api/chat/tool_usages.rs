use crate::models::tool_usage::ToolUsage;
use crate::utils::{get_app_state, AppError};
use crate::utils::middleware::auth::auth_required;
use crate::utils::middleware::client_scoped;
use salvo::prelude::*;
use uuid::Uuid;

#[handler]
pub async fn get_tool_usage(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let tool_usage_id = req
        .param::<String>("id")
        .ok_or(AppError::BadRequest("Missing tool usage id".to_string()))?;

    // Parse UUID
    let tool_usage_uuid = Uuid::parse_str(&tool_usage_id)
        .map_err(|_| AppError::BadRequest("Invalid tool usage id format".to_string()))?;

    // Fetch tool usage from database
    let tool_usage = sqlx::query_as::<_, (
        Uuid,
        String,
        String,
        Option<String>,
        Option<serde_json::Value>,
        Option<serde_json::Value>,
        Option<i64>,
        chrono::DateTime<chrono::Utc>,
    )>(
        "SELECT 
            id,
            message_id,
            tool_name,
            tool_use_id,
            parameters,
            output,
            execution_time_ms,
            created_at
        FROM tool_usages
        WHERE id = $1"
    )
    .bind(tool_usage_uuid)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    .ok_or(AppError::NotFound(format!(
        "Tool usage {} not found",
        tool_usage_id
    )))?;

    let tool_usage_response = ToolUsage {
        id: tool_usage.0,
        message_id: tool_usage.1,
        tool_name: tool_usage.2,
        tool_use_id: tool_usage.3,
        parameters: tool_usage.4,
        output: tool_usage.5,
        execution_time_ms: tool_usage.6,
        created_at: Some(tool_usage.7.to_rfc3339()),
    };

    res.render(Json(tool_usage_response));
    Ok(())
}

pub fn tool_usage_routes() -> Router {
    Router::new()
        .hoop(auth_required)
        .hoop(client_scoped)
        .push(Router::with_path("/tool-usages/{id}")
            .get(get_tool_usage))
}