use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use crate::models::{Message, MessageRole};
use crate::state::AppState;
use crate::error::AppError;
use chrono::Utc;
use uuid::Uuid;

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
    let _state = depot.obtain::<AppState>().unwrap();
    let chat_request: ChatRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;

    // For now, we'll create a simple echo response
    // In a real implementation, this would integrate with an AI service
    let last_message = chat_request.messages.last()
        .ok_or_else(|| AppError::BadRequest("No messages provided".to_string()))?;

    let response_content = format!(
        "I received your message: '{}'. This is a placeholder response from the Clay Studio backend.",
        last_message.content
    );

    let response = ChatResponse {
        id: Uuid::new_v4().to_string(),
        content: response_content,
        role: MessageRole::Assistant,
        created_at: Utc::now().to_rfc3339(),
        clay_tools_used: Some(vec!["data_analysis".to_string()]),
        processing_time_ms: Some(150),
    };

    res.render(Json(response));
    Ok(())
}