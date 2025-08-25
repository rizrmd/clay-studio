use serde::{Deserialize, Serialize};
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub content: String,
    pub role: MessageRole,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    pub clay_tools_used: Option<Vec<String>>,
    pub processing_time_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl Message {
    pub fn new_user(content: String) -> Self {
        Message {
            id: Uuid::new_v4().to_string(),
            content,
            role: MessageRole::User,
            created_at: Some(Utc::now().to_rfc3339()),
            clay_tools_used: None,
            processing_time_ms: None,
        }
    }

    pub fn new_assistant(content: String) -> Self {
        Message {
            id: Uuid::new_v4().to_string(),
            content,
            role: MessageRole::Assistant,
            created_at: Some(Utc::now().to_rfc3339()),
            clay_tools_used: None,
            processing_time_ms: None,
        }
    }
}