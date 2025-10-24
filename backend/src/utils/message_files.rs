use crate::models::file_upload::FileUpload;
use crate::utils::AppError;
use chrono::Utc;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Associate files with a message
pub async fn associate_files_with_message(
    pool: &PgPool,
    message_id: &str,
    file_ids: Vec<Uuid>,
) -> Result<(), AppError> {
    for file_id in file_ids {
        // Check if the association already exists to avoid duplicates
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM message_files WHERE message_id = $1 AND file_id = $2"
        )
        .bind(message_id)
        .bind(file_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to check existing association: {}", e)))?;

        if existing == 0 {
            // Only insert if it doesn't already exist
            sqlx::query(
                "INSERT INTO message_files (id, message_id, file_id, created_at)
                 VALUES ($1, $2, $3, $4)"
            )
            .bind(Uuid::new_v4())
            .bind(message_id)
            .bind(file_id)
            .bind(Utc::now())
            .execute(pool)
            .await
            .map_err(|e| AppError::InternalServerError(format!("Failed to associate file with message: {}", e)))?;
        }
    }
    Ok(())
}

/// Get files associated with a message
pub async fn get_message_files(
    pool: &PgPool,
    message_id: &str,
) -> Result<Vec<FileUpload>, AppError> {
    let files = sqlx::query_as::<_, FileUpload>(
        "SELECT f.* FROM file_uploads f
         JOIN message_files mf ON f.id = mf.file_id
         WHERE mf.message_id = $1
         ORDER BY mf.created_at ASC"
    )
    .bind(message_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to get message files: {}", e)))?;

    Ok(files)
}

/// Get files associated with multiple messages
#[allow(dead_code)]
pub async fn get_messages_files(
    pool: &PgPool,
    message_ids: Vec<String>,
) -> Result<std::collections::HashMap<String, Vec<FileUpload>>, AppError> {
    if message_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let files = sqlx::query(
        "SELECT f.*, mf.message_id FROM file_uploads f
         JOIN message_files mf ON f.id = mf.file_id
         WHERE mf.message_id = ANY($1)
         ORDER BY mf.created_at ASC"
    )
    .bind(&message_ids)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to get messages files: {}", e)))?;

    let mut result = std::collections::HashMap::new();
    for row in files {
        let message_id: String = row.get("message_id");
        let file = FileUpload {
            id: row.get("id"),
            client_id: row.get("client_id"),
            project_id: row.get("project_id"),
            conversation_id: row.get("conversation_id"),
            file_name: row.get("file_name"),
            original_name: row.get("original_name"),
            file_path: row.get("file_path"),
            file_size: row.get("file_size"),
            mime_type: row.get("mime_type"),
            description: row.get("description"),
            auto_description: row.get("auto_description"),
            file_content: row.get("file_content"),
            metadata: row.get("metadata"),
            uploaded_by: row.get("uploaded_by"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };
        
        result.entry(message_id).or_insert_with(Vec::new).push(file);
    }

    Ok(result)
}

/// Remove file associations from a message
#[allow(dead_code)]
pub async fn remove_files_from_message(
    pool: &PgPool,
    message_id: &str,
    file_ids: Option<Vec<Uuid>>,
) -> Result<(), AppError> {
    if let Some(file_ids) = file_ids {
        // Remove specific files
        sqlx::query(
            "DELETE FROM message_files WHERE message_id = $1 AND file_id = ANY($2)"
        )
        .bind(message_id)
        .bind(&file_ids)
        .execute(pool)
        .await
    } else {
        // Remove all files from the message
        sqlx::query("DELETE FROM message_files WHERE message_id = $1")
            .bind(message_id)
            .execute(pool)
            .await
    }
    .map_err(|e| AppError::InternalServerError(format!("Failed to remove file associations: {}", e)))?;

    Ok(())
}

/// Format files for Claude prompt context
pub fn format_files_for_prompt(files: Vec<FileUpload>) -> String {
    if files.is_empty() {
        return String::new();
    }

    let mut formatted = String::from("\n# Attached Files:\n\n");

    // Get the current working directory to convert relative paths to absolute
    let current_dir = std::env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "/Users/riz/Developer/clay-studio".to_string());

    for (i, file) in files.iter().enumerate() {
        formatted.push_str(&format!("## File {}: {}\n", i + 1, file.original_name));
        formatted.push_str(&format!("- **File ID**: {}\n", file.id));
        formatted.push_str(&format!("- **Type**: {}\n", file.mime_type.as_deref().unwrap_or("unknown")));
        formatted.push_str(&format!("- **Size**: {} bytes\n", file.file_size));

        if let Some(description) = &file.description {
            formatted.push_str(&format!("- **Description**: {}\n", description));
        }

        if let Some(auto_desc) = &file.auto_description {
            formatted.push_str(&format!("- **Auto Description**: {}\n", auto_desc));
        }

        if let Some(content) = &file.file_content {
            let preview = if content.len() > 1000 {
                format!("{}... (truncated)", &content[..1000])
            } else {
                content.clone()
            };
            formatted.push_str(&format!("- **Content Preview**:\n```\n{}\n```\n", preview));
        }

        // Always include file path for images
        if file.mime_type.as_ref().map(|mt| mt.starts_with("image/")).unwrap_or(false) {
            // Convert relative path to absolute path
            let absolute_path = if file.file_path.starts_with('/') {
                file.file_path.clone()
            } else {
                format!("{}/{}", current_dir, file.file_path)
            };

            formatted.push_str(&format!("- **Image File Path**: {}\n", absolute_path));
            formatted.push_str("- **Note**: This is an image file. Use the Read tool with the file path to access and analyze the image content.\n");
        } else if file.file_content.is_none() {
            // Convert relative path to absolute path for non-image files too
            let absolute_path = if file.file_path.starts_with('/') {
                file.file_path.clone()
            } else {
                format!("{}/{}", current_dir, file.file_path)
            };
            formatted.push_str(&format!("- **File Path**: {}\n", absolute_path));
            formatted.push_str("- **Note**: This is a binary file. Use Read tool to access if needed.\n");
        }

        formatted.push('\n');
    }

    formatted
}

/// Check if a message has file attachments
#[allow(dead_code)]
pub async fn message_has_files(
    pool: &PgPool,
    message_id: &str,
) -> Result<bool, AppError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM message_files WHERE message_id = $1"
    )
    .bind(message_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to check message files: {}", e)))?;

    Ok(count > 0)
}

/// Get file association statistics for a conversation
#[allow(dead_code)]
pub async fn get_conversation_file_stats(
    pool: &PgPool,
    conversation_id: &str,
) -> Result<serde_json::Value, AppError> {
    let stats = sqlx::query(
        "SELECT 
            COUNT(DISTINCT mf.file_id) as total_files,
            COUNT(DISTINCT mf.message_id) as messages_with_files,
            COUNT(*) as total_associations,
            array_agg(DISTINCT f.mime_type) as file_types
         FROM message_files mf
         JOIN messages m ON m.id = mf.message_id
         JOIN file_uploads f ON f.id = mf.file_id
         WHERE m.conversation_id = $1"
    )
    .bind(conversation_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to get conversation file stats: {}", e)))?;

    Ok(json!({
        "total_files": stats.get::<Option<i64>, _>("total_files").unwrap_or(0),
        "messages_with_files": stats.get::<Option<i64>, _>("messages_with_files").unwrap_or(0),
        "total_associations": stats.get::<Option<i64>, _>("total_associations").unwrap_or(0),
        "file_types": stats.get::<Option<Vec<String>>, _>("file_types").unwrap_or_default()
    }))
}