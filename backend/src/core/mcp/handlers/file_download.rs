use crate::core::mcp::handlers::base::McpHandlers;
use crate::core::mcp::types::*;
use crate::models::file_upload::FileUpload;
use crate::utils::content_extractor::ContentExtractor;
use chrono::Utc;
use serde_json::json;
use std::path::Path;
use uuid::Uuid;

/// Download and process a file from URL
impl McpHandlers {
    pub async fn handle_file_download_url(
        &self,
        arguments: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<String, JsonRpcError> {
        // Extract parameters
        let url = arguments
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: url".to_string(),
                data: None,
            })?;

        let file_name = arguments
            .get("file_name")
            .and_then(|v| v.as_str());

        let auto_extract = arguments
            .get("auto_extract")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let conversation_id = arguments
            .get("conversation_id")
            .and_then(|v| v.as_str());

        // Validate URL
        let parsed_url = url::Url::parse(url).map_err(|e| JsonRpcError {
            code: INVALID_PARAMS,
            message: format!("Invalid URL: {}", e),
            data: None,
        })?;

        // Security checks
        if !Self::is_url_safe(&parsed_url) {
            return Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: "URL is not allowed. Only HTTP(S) URLs to public resources are permitted.".to_string(),
                data: None,
            });
        }

        // Determine file name from URL or parameter
        let original_name = file_name.unwrap_or_else(|| {
            parsed_url
                .path_segments()
                .and_then(|mut segments| segments.next_back())
                .filter(|s| !s.is_empty() && s.contains('.'))
                .unwrap_or("downloaded_file")
        }).to_string();

        // Create temporary directory for download
        let temp_dir = format!(".clients/{}/{}/temp", self.client_id, self.project_id);
        std::fs::create_dir_all(&temp_dir).map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Failed to create temp directory: {}", e),
            data: None,
        })?;

        // Generate unique file name for storage
        let file_id = Uuid::new_v4();
        let file_extension = Path::new(&original_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        let stored_filename = if file_extension.is_empty() {
            format!("{}", file_id)
        } else {
            format!("{}.{}", file_id, file_extension)
        };

        let temp_path = format!("{}/{}", temp_dir, stored_filename);

        // Download the file with size limits
        let download_result = self.download_file_with_limits(url, &temp_path).await?;

        // Move to permanent storage
        let upload_dir = format!(".clients/{}/{}/uploads", self.client_id, self.project_id);
        std::fs::create_dir_all(&upload_dir).map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Failed to create upload directory: {}", e),
            data: None,
        })?;

        let final_path = format!("{}/{}", upload_dir, stored_filename);
        std::fs::rename(&temp_path, &final_path).map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Failed to move file: {}", e),
            data: None,
        })?;

        // Extract content if requested and file is small enough
        let (extracted_content, file_content, metadata) = if auto_extract && download_result.size_bytes < 10 * 1024 * 1024 {
            match ContentExtractor::extract_content(
                Path::new(&final_path),
                &original_name,
                &download_result.content_type,
            ).await {
                Ok(extracted) => {
                    let content = extracted.text_content.clone();
                    let meta = extracted.structured_data.clone();
                    (Some(extracted), content, meta)
                },
                Err(e) => {
                    tracing::warn!("Content extraction failed: {}", e);
                    (None, None, None)
                }
            }
        } else {
            (None, None, None)
        };

        // Parse client UUID
        let client_uuid = Uuid::parse_str(&self.client_id).map_err(|e| JsonRpcError {
            code: INVALID_PARAMS,
            message: format!("Invalid client ID: {}", e),
            data: None,
        })?;

        // Keep conversation ID as string (not UUID)

        // Save to database
        let file_record = FileUpload {
            id: file_id,
            client_id: client_uuid,
            project_id: self.project_id.clone(),
            conversation_id: conversation_id.map(|s| s.to_string()),
            file_name: stored_filename.clone(),
            original_name: original_name.clone(),
            file_path: final_path.clone(),
            file_size: download_result.size_bytes as i64,
            mime_type: Some(download_result.content_type.clone()),
            description: Some(format!("Downloaded from: {}", url)),
            auto_description: extracted_content.as_ref().and_then(|e| e.description.clone()),
            file_content,
            metadata,
            uploaded_by: None, // uploaded_by is Uuid, we'll leave it as None for AI downloads
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Insert into database
        sqlx::query(
            "INSERT INTO file_uploads 
            (id, client_id, project_id, conversation_id, file_name, original_name, 
             file_path, file_size, mime_type, description, auto_description, 
             file_content, metadata, uploaded_by, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)"
        )
        .bind(file_record.id)
        .bind(file_record.client_id)
        .bind(&file_record.project_id)
        .bind(file_record.conversation_id)
        .bind(&file_record.file_name)
        .bind(&file_record.original_name)
        .bind(&file_record.file_path)
        .bind(file_record.file_size)
        .bind(&file_record.mime_type)
        .bind(&file_record.description)
        .bind(&file_record.auto_description)
        .bind(&file_record.file_content)
        .bind(&file_record.metadata)
        .bind(file_record.uploaded_by)
        .bind(file_record.created_at)
        .bind(file_record.updated_at)
        .execute(&self.db_pool)
        .await
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Database error: {}", e),
            data: None,
        })?;

        // Build response
        let response = json!({
            "status": "success",
            "message": format!("File downloaded successfully from {}", url),
            "file": {
                "id": file_record.id.to_string(),
                "name": file_record.file_name,
                "original_name": file_record.original_name,
                "size": file_record.file_size,
                "size_mb": (file_record.file_size as f64 / (1024.0 * 1024.0)),
                "mime_type": file_record.mime_type,
                "source_url": url,
                "content_extracted": file_record.file_content.is_some(),
                "auto_description": file_record.auto_description,
                "created_at": file_record.created_at,
            },
            "download_info": {
                "url": url,
                "size_bytes": download_result.size_bytes,
                "content_type": download_result.content_type,
                "download_time_ms": download_result.download_time_ms,
            },
            "access_tools": if file_record.file_size > 10 * 1024 * 1024 {
                Some(json!([
                    {
                        "tool": "file_peek",
                        "description": "Use to sample parts of the large file",
                        "example": json!({
                            "tool": "file_peek",
                            "arguments": {
                                "file_id": file_record.id.to_string(),
                                "strategy": "smart"
                            }
                        })
                    },
                    {
                        "tool": "file_range",
                        "description": "Use to extract specific ranges",
                        "example": json!({
                            "tool": "file_range",
                            "arguments": {
                                "file_id": file_record.id.to_string(),
                                "unit": "auto",
                                "start": 0,
                                "end": 100
                            }
                        })
                    }
                ]))
            } else {
                None
            }
        });

        serde_json::to_string(&response).map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Failed to serialize response: {}", e),
            data: None,
        })
    }

    /// Check if URL is safe to download from
    fn is_url_safe(url: &url::Url) -> bool {
        // Only allow HTTP(S) protocols
        if !matches!(url.scheme(), "http" | "https") {
            return false;
        }

        // Block local/internal addresses
        if let Some(host) = url.host_str() {
            // Block localhost and local IPs
            if host == "localhost" || host == "127.0.0.1" || host == "0.0.0.0" {
                return false;
            }

            // Block private IP ranges (RFC 1918)
            if host.starts_with("10.") || 
               host.starts_with("192.168.") || 
               host.starts_with("172.16.") ||
               host.starts_with("172.17.") ||
               host.starts_with("172.18.") ||
               host.starts_with("172.19.") ||
               host.starts_with("172.20.") ||
               host.starts_with("172.21.") ||
               host.starts_with("172.22.") ||
               host.starts_with("172.23.") ||
               host.starts_with("172.24.") ||
               host.starts_with("172.25.") ||
               host.starts_with("172.26.") ||
               host.starts_with("172.27.") ||
               host.starts_with("172.28.") ||
               host.starts_with("172.29.") ||
               host.starts_with("172.30.") ||
               host.starts_with("172.31.") {
                return false;
            }

            // Block link-local addresses
            if host.starts_with("169.254.") {
                return false;
            }

            // Block file:// URLs disguised as HTTP
            if host.starts_with("file") {
                return false;
            }
        }

        true
    }

    /// Download file with size and timeout limits
    async fn download_file_with_limits(
        &self,
        url: &str,
        target_path: &str,
    ) -> Result<DownloadResult, JsonRpcError> {
        use tokio::time::{timeout, Duration};
        
        const MAX_DOWNLOAD_SIZE: u64 = 100 * 1024 * 1024; // 100MB max
        const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60); // 60 second timeout

        let start_time = std::time::Instant::now();

        // Create HTTP client with timeout
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Clay Studio AI Assistant/1.0")
            .build()
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to create HTTP client: {}", e),
                data: None,
            })?;

        // Send GET request with timeout
        let response = timeout(DOWNLOAD_TIMEOUT, client.get(url).send())
            .await
            .map_err(|_| JsonRpcError {
                code: INTERNAL_ERROR,
                message: "Download timeout exceeded (60 seconds)".to_string(),
                data: None,
            })?
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to download file: {}", e),
                data: None,
            })?;

        // Check response status
        if !response.status().is_success() {
            return Err(JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Download failed with status: {}", response.status()),
                data: None,
            });
        }

        // Get content type
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        // Check content length
        if let Some(content_length) = response.content_length() {
            if content_length > MAX_DOWNLOAD_SIZE {
                return Err(JsonRpcError {
                    code: INVALID_PARAMS,
                    message: format!(
                        "File too large: {}MB exceeds maximum of {}MB",
                        content_length / (1024 * 1024),
                        MAX_DOWNLOAD_SIZE / (1024 * 1024)
                    ),
                    data: None,
                });
            }
        }

        // Download with streaming to prevent memory exhaustion
        let mut file = tokio::fs::File::create(target_path)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to create file: {}", e),
                data: None,
            })?;

        let mut stream = response.bytes_stream();
        let mut total_bytes = 0u64;

        use tokio::io::AsyncWriteExt;
        use futures_util::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Download stream error: {}", e),
                data: None,
            })?;

            total_bytes += chunk.len() as u64;

            // Check size limit during download
            if total_bytes > MAX_DOWNLOAD_SIZE {
                // Clean up partial file
                drop(file);
                let _ = tokio::fs::remove_file(target_path).await;
                
                return Err(JsonRpcError {
                    code: INVALID_PARAMS,
                    message: format!(
                        "Download exceeded maximum size of {}MB",
                        MAX_DOWNLOAD_SIZE / (1024 * 1024)
                    ),
                    data: None,
                });
            }

            file.write_all(&chunk).await.map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to write file: {}", e),
                data: None,
            })?;
        }

        file.flush().await.map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Failed to flush file: {}", e),
            data: None,
        })?;

        let download_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(DownloadResult {
            size_bytes: total_bytes,
            content_type,
            download_time_ms,
        })
    }
}

#[derive(Debug)]
struct DownloadResult {
    size_bytes: u64,
    content_type: String,
    download_time_ms: u64,
}