use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use crate::models::{Message, MessageRole};
use crate::state::AppState;
use crate::error::AppError;
use chrono::Utc;
use uuid::Uuid;
use sqlx::Row;

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
    #[allow(dead_code)]
    pub project_id: String,
    #[allow(dead_code)]
    pub conversation_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub id: String,
    pub content: String,
    pub role: MessageRole,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub clay_tools_used: Option<Vec<String>>,
    pub processing_time_ms: Option<i64>,
}

#[handler]
pub async fn handle_chat(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let chat_request: ChatRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;

    let last_message = chat_request.messages.last()
        .ok_or_else(|| AppError::BadRequest("No messages provided".to_string()))?;

    // Get the first active client from the database
    let client_row = sqlx::query(
        "SELECT id, claude_token, install_path FROM clients WHERE status = 'active' AND claude_token IS NOT NULL LIMIT 1"
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let (client_id, _oauth_token, install_path) = if let Some(row) = client_row {
        let id: Uuid = row.get("id");
        let token: String = row.get("claude_token");
        let path: String = row.get("install_path");
        (id, token, path)
    } else {
        // No active client found, return a helpful error message
        return Err(AppError::ServiceUnavailable(
            "No active Claude client available. Please set up a client first.".to_string()
        ));
    };

    tracing::info!("Using client {} for chat request", client_id);

    // Execute Claude CLI to get a response
    let claude_bin = format!("{}/node_modules/@anthropic-ai/claude-code/cli.js", install_path);
    
    // Check if the Claude CLI exists
    if !std::path::Path::new(&claude_bin).exists() {
        return Err(AppError::ServiceUnavailable(
            format!("Claude CLI not found for client {}. Please reinstall the client.", client_id)
        ));
    }

    // For now, use a simple echo implementation
    // In production, you would use the Claude API properly
    let start_time = std::time::Instant::now();
    
    // Create a simple response using the Claude CLI or API
    // This is a placeholder - in production you'd properly integrate with Claude's API
    let response_content = format!(
        "I received your message: '{}'. (Processed by client: {})",
        last_message.content,
        client_id
    );
    
    let processing_time_ms = start_time.elapsed().as_millis() as i64;

    let response = ChatResponse {
        id: Uuid::new_v4().to_string(),
        content: response_content,
        role: MessageRole::Assistant,
        created_at: Utc::now().to_rfc3339(),
        clay_tools_used: Some(vec!["claude_api".to_string()]),
        processing_time_ms: Some(processing_time_ms),
    };

    res.render(Json(response));
    Ok(())
}