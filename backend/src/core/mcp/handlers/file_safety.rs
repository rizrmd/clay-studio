use crate::core::mcp::handlers::base::McpHandlers;
use crate::core::mcp::types::*;
use crate::models::file_upload::FileUpload;
use crate::utils::content_extractor::ContentExtractor;
use serde_json::json;
use std::fs;

impl McpHandlers {
    /// Safe file_read implementation with large file protection
    pub async fn handle_file_read_safe(
        &self,
        arguments: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<String, JsonRpcError> {
        let file_id = arguments
            .get("file_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: file_id".to_string(),
                data: None,
            })?;

        let file_uuid = uuid::Uuid::parse_str(file_id).map_err(|e| JsonRpcError {
            code: INVALID_PARAMS,
            message: format!("Invalid file ID: {}", e),
            data: None,
        })?;

        let client_uuid = uuid::Uuid::parse_str(&self.client_id).map_err(|e| JsonRpcError {
            code: INVALID_PARAMS,
            message: format!("Invalid client ID: {}", e),
            data: None,
        })?;

        let file = sqlx::query_as::<_, FileUpload>(
            "SELECT * FROM file_uploads 
             WHERE id = $1 AND client_id = $2 AND project_id = $3",
        )
        .bind(file_uuid)
        .bind(client_uuid)
        .bind(&self.project_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Database error: {}", e),
            data: None,
        })?
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "File not found".to_string(),
            data: None,
        })?;

        // Get processing limits
        let limits = ContentExtractor::get_limits();
        let file_size_mb = file.file_size as f64 / (1024.0 * 1024.0);
        let max_size_mb = limits.max_full_parse_size as f64 / (1024.0 * 1024.0);

        // Check if file is too large
        let is_large_file = file.file_size as u64 > limits.max_full_parse_size;

        let response = if is_large_file {
            // File is too large - provide helpful guidance instead of trying to read it
            json!({
                "status": "partial",
                "message": format!(
                    "File too large for full content retrieval ({}MB exceeds {}MB limit)",
                    file_size_mb.round(),
                    max_size_mb.round()
                ),
                "file": {
                    "id": file.id.to_string(),
                    "name": file.file_name,
                    "original_name": file.original_name,
                    "mime_type": file.mime_type,
                    "size": file.file_size,
                    "size_mb": file_size_mb,
                    "description": file.description,
                    "auto_description": file.auto_description,
                    "conversation_id": file.conversation_id,
                    "created_at": file.created_at,
                    "updated_at": file.updated_at,
                    "metadata": file.metadata
                },
                "large_file_info": {
                    "file_size_mb": file_size_mb,
                    "max_processable_size_mb": max_size_mb,
                    "exceeds_limit_by_mb": (file_size_mb - max_size_mb).max(0.0),
                    "recommended_tools": [
                        {
                            "tool": "file_peek",
                            "description": "Use to sample parts of the file without loading everything",
                            "example": json!({
                                "tool": "file_peek",
                                "arguments": {
                                    "file_id": file_id,
                                    "strategy": "smart",
                                    "sample_size": 5000
                                }
                            })
                        },
                        {
                            "tool": "file_range",
                            "description": "Use to extract specific ranges from the file",
                            "example": json!({
                                "tool": "file_range",
                                "arguments": {
                                    "file_id": file_id,
                                    "unit": "auto",
                                    "start": 0,
                                    "end": 100
                                }
                            })
                        },
                        {
                            "tool": "file_search",
                            "description": "Use to search for specific content within the file",
                            "example": json!({
                                "tool": "file_search",
                                "arguments": {
                                    "file_id": file_id,
                                    "pattern": "search_term",
                                    "max_results": 10
                                }
                            })
                        }
                    ]
                },
                "warning": "⚠️ DO NOT attempt to read this file directly. Use the recommended tools above for safe, memory-efficient access."
            })
        } else if let Some(content) = file.file_content {
            // Content is available in database (file was small enough to process)
            json!({
                "status": "success",
                "message": "File content retrieved successfully",
                "file": {
                    "id": file.id.to_string(),
                    "name": file.file_name,
                    "original_name": file.original_name,
                    "mime_type": file.mime_type,
                    "size": file.file_size,
                    "size_mb": file_size_mb,
                    "content": content,
                    "description": file.description,
                    "auto_description": file.auto_description,
                    "created_at": file.created_at
                }
            })
        } else {
            // Try reading from filesystem only for small files
            // Double-check file size on disk to prevent accidents
            let metadata = fs::metadata(&file.file_path).map_err(|_| JsonRpcError {
                code: INTERNAL_ERROR,
                message: "Unable to access file on filesystem".to_string(),
                data: None,
            })?;

            if metadata.len() > limits.max_full_parse_size {
                // File on disk is too large - don't attempt to read
                json!({
                    "status": "partial",
                    "message": format!(
                        "File too large to read directly ({}MB exceeds {}MB limit)",
                        (metadata.len() as f64 / (1024.0 * 1024.0)).round(),
                        max_size_mb.round()
                    ),
                    "file": {
                        "id": file.id.to_string(),
                        "name": file.file_name,
                        "original_name": file.original_name,
                        "mime_type": file.mime_type,
                        "size": file.file_size,
                        "size_mb": file_size_mb,
                        "description": file.description,
                        "auto_description": file.auto_description,
                        "metadata": file.metadata,
                        "created_at": file.created_at
                    },
                    "recommended_action": "Use file_peek or file_range tools for large files"
                })
            } else {
                // Safe to read small file from filesystem
                match fs::read_to_string(&file.file_path) {
                    Ok(content) => json!({
                        "status": "success",
                        "message": "File content retrieved from filesystem",
                        "file": {
                            "id": file.id.to_string(),
                            "name": file.file_name,
                            "original_name": file.original_name,
                            "mime_type": file.mime_type,
                            "size": file.file_size,
                            "content": content,
                            "description": file.description,
                            "auto_description": file.auto_description,
                            "created_at": file.created_at
                        }
                    }),
                    Err(_) => json!({
                        "status": "error",
                        "message": "File is binary or cannot be read as text",
                        "file": {
                            "id": file.id.to_string(),
                            "name": file.file_name,
                            "original_name": file.original_name,
                            "mime_type": file.mime_type,
                            "size": file.file_size,
                            "content": null,
                            "description": file.description,
                            "auto_description": file.auto_description,
                            "created_at": file.created_at,
                            "file_path": file.file_path
                        },
                        "note": "Use file_peek or file_range to access this file's content"
                    })
                }
            }
        };

        serde_json::to_string(&response).map_err(|e| JsonRpcError {
            code: -32603,
            message: format!("Failed to serialize response: {}", e),
            data: None,
        })
    }
}