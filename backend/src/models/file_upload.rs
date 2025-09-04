use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FileUpload {
    pub id: Uuid,
    pub client_id: Uuid,
    pub project_id: String,
    pub conversation_id: Option<String>,
    pub file_name: String,
    pub original_name: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: Option<String>,
    pub description: Option<String>,
    pub auto_description: Option<String>,
    pub file_content: Option<String>, // For text files
    pub metadata: Option<serde_json::Value>,
    pub uploaded_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateFileUpload {
    pub client_id: Uuid,
    pub project_id: String,
    pub conversation_id: Option<String>,
    pub file_name: String,
    pub original_name: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: Option<String>,
    pub description: Option<String>,
    pub file_content: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub uploaded_by: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateFileDescription {
    pub description: Option<String>,
    pub auto_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUploadResponse {
    pub id: String,
    pub file_name: String,
    pub original_name: String,
    pub file_path: String,
    pub file_size: i64, // Changed from 'size' to 'file_size' to match frontend expectations
    pub mime_type: Option<String>,
    pub description: Option<String>,
    pub auto_description: Option<String>,
    pub created_at: String,
    pub is_text_file: bool,
    pub preview: Option<String>, // First 500 chars of text files
}

impl FileUpload {
    pub fn to_response(&self) -> FileUploadResponse {
        let is_text = self
            .mime_type
            .as_ref()
            .map(|m| {
                m.starts_with("text/") || m == "application/json" || m == "application/javascript"
            })
            .unwrap_or(false);

        let preview = if is_text {
            self.file_content.as_ref().map(|content| {
                let preview_len = content.len().min(500);
                content[..preview_len].to_string()
            })
        } else {
            None
        };

        FileUploadResponse {
            id: self.id.to_string(),
            file_name: self.file_name.clone(),
            original_name: self.original_name.clone(),
            file_path: self.file_path.clone(),
            file_size: self.file_size, // Changed from 'size' to 'file_size'
            mime_type: self.mime_type.clone(),
            description: self.description.clone(),
            auto_description: self.auto_description.clone(),
            created_at: self.created_at.to_rfc3339(),
            is_text_file: is_text,
            preview,
        }
    }
}

// Helper function to determine if a file is text-based
pub fn is_text_file(mime_type: Option<&str>, file_name: &str) -> bool {
    if let Some(mime) = mime_type {
        if mime.starts_with("text/")
            || mime == "application/json"
            || mime == "application/javascript"
            || mime == "application/xml"
            || mime == "application/x-sh"
            || mime == "application/x-python"
            || mime == "application/x-ruby"
            || mime == "application/yaml"
        {
            return true;
        }
    }

    // Check by extension if MIME type is not conclusive
    let extensions = vec![
        ".txt", ".md", ".json", ".js", ".ts", ".jsx", ".tsx", ".py", ".rb", ".rs", ".go", ".java",
        ".c", ".cpp", ".h", ".css", ".html", ".xml", ".yaml", ".yml", ".toml", ".ini", ".sh",
        ".bash", ".zsh", ".fish", ".ps1", ".bat", ".cmd",
    ];

    extensions.iter().any(|ext| file_name.ends_with(ext))
}
