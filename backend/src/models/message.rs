use crate::models::file_upload::FileUploadResponse;
use crate::models::message_role::MessageRole;
use crate::models::tool_usage::ToolUsage;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub content: String,
    pub role: MessageRole,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    pub processing_time_ms: Option<i64>,
    pub file_attachments: Option<Vec<FileUploadResponse>>,
    pub tool_usages: Option<Vec<ToolUsage>>,
}

impl Message {
    #[allow(dead_code)]
    pub fn new_user(content: String) -> Self {
        Message {
            id: Uuid::new_v4().to_string(),
            content,
            role: MessageRole::User,
            created_at: Some(Utc::now().to_rfc3339()),
            processing_time_ms: None,
            file_attachments: None,
            tool_usages: None,
        }
    }

    #[allow(dead_code)]
    pub fn new_assistant(content: String) -> Self {
        Message {
            id: Uuid::new_v4().to_string(),
            content,
            role: MessageRole::Assistant,
            created_at: Some(Utc::now().to_rfc3339()),
            processing_time_ms: None,
            file_attachments: None,
            tool_usages: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_files(mut self, files: Vec<FileUploadResponse>) -> Self {
        self.file_attachments = if files.is_empty() { None } else { Some(files) };
        self
    }
}
