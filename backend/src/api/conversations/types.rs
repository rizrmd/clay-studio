use crate::models::tool_usage::ToolUsage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateConversationRequest {
    pub project_id: String,
    pub title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateFromMessageRequest {
    pub project_id: String,
    pub source_conversation_id: Option<String>,
    pub message_id: String,
    pub messages: Vec<MessageForClone>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageForClone {
    pub id: String,
    pub content: String,
    pub role: String,
    pub file_attachments: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateConversationRequest {
    pub title: Option<String>,
    pub is_title_manually_set: Option<bool>,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct MessageResponse {
    pub id: String,
    pub content: String,
    pub role: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub processing_time_ms: Option<i64>,
    pub tool_usages: Option<Vec<ToolUsage>>,
}
