use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use crate::models::{Message, MessageRole};
use crate::state::AppState;
use crate::error::AppError;
use crate::claude::{ClaudeManager, QueryOptions, ClaudeMessage};
use chrono::Utc;
use uuid::Uuid;
use sqlx::Row;
use tokio::time::{timeout, Duration};

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

    let _last_message = chat_request.messages.last()
        .ok_or_else(|| AppError::BadRequest("No messages provided".to_string()))?;

    // Get the first active client from the database
    let client_row = sqlx::query(
        "SELECT id, claude_token, install_path FROM clients WHERE status = 'active' AND claude_token IS NOT NULL LIMIT 1"
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let client_id = if let Some(row) = client_row {
        let id: Uuid = row.get("id");
        id
    } else {
        // No active client found, return a helpful error message
        return Err(AppError::ServiceUnavailable(
            "No active Claude client available. Please set up a client first.".to_string()
        ));
    };

    tracing::info!("Using client {} for chat request", client_id);

    let start_time = std::time::Instant::now();
    
    // Build the conversation context from all messages
    let mut conversation = String::new();
    for msg in &chat_request.messages {
        match msg.role {
            MessageRole::User => conversation.push_str(&format!("User: {}\n", msg.content)),
            MessageRole::Assistant => conversation.push_str(&format!("Assistant: {}\n", msg.content)),
            MessageRole::System => conversation.push_str(&format!("System: {}\n", msg.content)),
        }
    }
    
    // Configure query options (you can customize these based on your needs)
    let options = QueryOptions {
        system_prompt: Some("You are a helpful AI assistant integrated into Clay Studio.".to_string()),
        max_turns: Some(1),
        allowed_tools: None, // Allow all tools by default
        permission_mode: None,
        resume_session_id: None,
        output_format: None,
    };
    
    // Execute the Claude query
    let mut response_content = String::new();
    let mut tools_used = Vec::new();
    
    match ClaudeManager::query_claude(client_id, conversation.clone(), Some(options)).await {
        Ok(mut receiver) => {
            // Process messages with a timeout
            let timeout_duration = Duration::from_secs(30);
            
            while let Ok(Some(message)) = timeout(timeout_duration, receiver.recv()).await {
                match message {
                    ClaudeMessage::Result { result } => {
                        response_content = result;
                        break;
                    }
                    ClaudeMessage::Progress { content } => {
                        // You could stream these to the client if needed
                        tracing::debug!("Progress: {}", content);
                    }
                    ClaudeMessage::ToolUse { tool, .. } => {
                        tools_used.push(tool);
                    }
                    ClaudeMessage::Error { error } => {
                        return Err(AppError::InternalServerError(
                            format!("Claude error: {}", error)
                        ));
                    }
                    _ => continue,
                }
            }
            
            if response_content.is_empty() {
                // Fallback if no result was received
                response_content = "I'm processing your request. Please try again if you don't see a response.".to_string();
            }
        }
        Err(e) => {
            tracing::error!("Failed to query Claude: {}", e);
            
            // Fallback response on error
            response_content = format!(
                "I apologize, but I'm having trouble processing your request. Error: {}",
                e
            );
        }
    }
    
    let processing_time_ms = start_time.elapsed().as_millis() as i64;

    let response = ChatResponse {
        id: Uuid::new_v4().to_string(),
        content: response_content,
        role: MessageRole::Assistant,
        created_at: Utc::now().to_rfc3339(),
        clay_tools_used: if tools_used.is_empty() { None } else { Some(tools_used) },
        processing_time_ms: Some(processing_time_ms),
    };

    res.render(Json(response));
    Ok(())
}