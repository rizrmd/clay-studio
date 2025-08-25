use salvo::prelude::*;
use salvo::fs::NamedFile;
use serde::Deserialize;
use uuid::Uuid;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use crate::utils::AppError;
use crate::utils::AppState;
use crate::models::file_upload::{FileUpload, UpdateFileDescription, FileUploadResponse, is_text_file};
use crate::core::claude::ClaudeManager;
use chrono::Utc;
use sqlx::{PgPool, Row};


#[derive(Debug, Deserialize)]
pub struct UploadParams {
    pub client_id: String,
    pub project_id: String,
    pub conversation_id: Option<String>,
}

#[handler]
pub async fn handle_file_upload(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let params: UploadParams = req.parse_queries()
        .map_err(|_| AppError::BadRequest("Missing client_id or project_id".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&params.client_id)
        .map_err(|_| AppError::BadRequest("Invalid client_id".to_string()))?;
    
    let file = req.file("file").await
        .ok_or_else(|| AppError::BadRequest("No file provided".to_string()))?;
    
    let file_name = file.name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("file_{}", Uuid::new_v4()));
    
    let mime_type = file.content_type()
        .map(|ct| ct.to_string());
    
    let upload_dir = PathBuf::from(".clients")
        .join(client_uuid.to_string())
        .join(&params.project_id)
        .join("uploads");
    
    fs::create_dir_all(&upload_dir).await
        .map_err(|e| AppError::InternalServerError(format!("Failed to create upload directory: {}", e)))?;
    
    let file_id = Uuid::new_v4();
    let file_path_obj = Path::new(&file_name);
    let file_extension = file_path_obj
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("bin");
    let file_stem = file_path_obj
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(&file_name);
    let saved_file_name = format!("{}_{}.{}", file_id, file_stem, file_extension);
    let file_path = upload_dir.join(&saved_file_name);
    
    // Read bytes from the temporary file
    let temp_path = file.path();
    let bytes = fs::read(temp_path).await
        .map_err(|e| AppError::InternalServerError(format!("Failed to read file: {}", e)))?;
    
    let mut dest_file = tokio::fs::File::create(&file_path).await
        .map_err(|e| AppError::InternalServerError(format!("Failed to create file: {}", e)))?;
    
    dest_file.write_all(&bytes).await
        .map_err(|e| AppError::InternalServerError(format!("Failed to write file: {}", e)))?;
    
    let relative_path = format!(".clients/{}/{}/uploads/{}", 
        client_uuid, 
        params.project_id, 
        saved_file_name
    );
    
    // Check if it's a text file and read content if so
    let file_content = if is_text_file(mime_type.as_deref(), &file_name) {
        String::from_utf8(bytes.clone()).ok()
    } else {
        None
    };
    
    // Save to database
    let file_upload = sqlx::query_as::<_, FileUpload>(
        "INSERT INTO file_uploads 
        (id, client_id, project_id, conversation_id, file_name, original_name, 
         file_path, file_size, mime_type, file_content, created_at, updated_at) 
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11) 
        RETURNING *"
    )
    .bind(file_id)
    .bind(client_uuid)
    .bind(&params.project_id)
    .bind(&params.conversation_id)
    .bind(&saved_file_name)
    .bind(&file_name)
    .bind(&relative_path)
    .bind(bytes.len() as i64)
    .bind(&mime_type)
    .bind(&file_content)
    .bind(Utc::now())
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to save file metadata: {}", e)))?;
    
    // Generate auto description in the background
    let pool_clone = state.db_pool.clone();
    let file_id_clone = file_id;
    let file_name_clone = file_name.clone();
    let content_clone = file_content.clone();
    let client_id_clone = client_uuid;
    let project_id_clone = params.project_id.clone();
    
    tokio::spawn(async move {
        if let Err(e) = generate_file_description(
            pool_clone, 
            file_id_clone, 
            file_name_clone, 
            content_clone,
            client_id_clone,
            project_id_clone
        ).await {
            tracing::error!("Failed to generate file description: {}", e);
        }
    });
    
    let response = file_upload.to_response();
    res.render(Json(response));
    Ok(())
}

// Helper function to generate file description using Claude
async fn generate_file_description(
    pool: PgPool,
    file_id: Uuid,
    file_name: String,
    file_content: Option<String>,
    client_id: Uuid,
    project_id: String,
) -> Result<(), AppError> {
    // Get Claude token from database
    let client_row = sqlx::query(
        "SELECT claude_token FROM clients WHERE id = $1"
    )
    .bind(client_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let claude_token = if let Some(row) = client_row {
        row.get::<Option<String>, _>("claude_token")
    } else {
        None
    };
    
    if claude_token.is_none() {
        return Ok(()); // Skip if no Claude token
    }
    
    let prompt = if let Some(content) = file_content {
        let preview = if content.len() > 2000 {
            format!("{}...", &content[..2000])
        } else {
            content
        };
        format!(
            "Please provide a brief, informative description (1-2 sentences) for this file named '{}'. \
             Here's a preview of its contents:\n\n{}\n\n\
             Focus on what the file does or contains, its purpose, and any key information. \
             Be concise and factual.",
            file_name, preview
        )
    } else {
        format!(
            "Please provide a brief, informative description (1-2 sentences) for a file named '{}'. \
             Based on the filename, describe what this file likely contains or its purpose. \
             Be concise and factual.",
            file_name
        )
    };
    
    // Query Claude for description
    match ClaudeManager::query_claude_with_project_and_token(
        client_id,
        &project_id,
        prompt,
        None,
        claude_token,
    ).await {
        Ok(mut receiver) => {
            let mut description = String::new();
            while let Some(message) = receiver.recv().await {
                if let crate::core::claude::ClaudeMessage::Result { result } = message {
                    description = result;
                    break;
                }
            }
            
            if !description.is_empty() {
                // Update the file with the auto-generated description
                sqlx::query(
                    "UPDATE file_uploads SET auto_description = $1, updated_at = $2 WHERE id = $3"
                )
                .bind(&description)
                .bind(Utc::now())
                .bind(file_id)
                .execute(&pool)
                .await
                .map_err(|e| AppError::InternalServerError(format!("Failed to update description: {}", e)))?;
            }
        }
        Err(e) => {
            tracing::warn!("Failed to generate description for file {}: {}", file_id, e);
        }
    }
    
    Ok(())
}

#[handler]
pub async fn handle_update_file_description(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    
    let file_id = req.param::<String>("file_id")
        .ok_or_else(|| AppError::BadRequest("Missing file_id".to_string()))?;
    
    let file_uuid = Uuid::parse_str(&file_id)
        .map_err(|_| AppError::BadRequest("Invalid file_id".to_string()))?;
    
    let update_data: UpdateFileDescription = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;
    
    // Update the file description
    let file = sqlx::query_as::<_, FileUpload>(
        "UPDATE file_uploads 
         SET description = COALESCE($1, description), 
             auto_description = COALESCE($2, auto_description),
             updated_at = $3
         WHERE id = $4
         RETURNING *"
    )
    .bind(&update_data.description)
    .bind(&update_data.auto_description)
    .bind(Utc::now())
    .bind(file_uuid)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to update file: {}", e)))?;
    
    res.render(Json(file.to_response()));
    Ok(())
}

#[handler]
pub async fn handle_file_download(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let client_id = req.param::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("Missing client_id".to_string()))?;
    let project_id = req.param::<String>("project_id")
        .ok_or_else(|| AppError::BadRequest("Missing project_id".to_string()))?;
    let file_name = req.param::<String>("file_name")
        .ok_or_else(|| AppError::BadRequest("Missing file_name".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client_id".to_string()))?;
    
    let file_path = PathBuf::from(".clients")
        .join(client_uuid.to_string())
        .join(&project_id)
        .join("uploads")
        .join(&file_name);
    
    if !file_path.exists() {
        return Err(AppError::NotFound("File not found".to_string()));
    }
    
    NamedFile::builder(file_path)
        .send(req.headers(), res)
        .await;
    
    Ok(())
}

#[handler]
pub async fn handle_list_uploads(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    
    let client_id = req.query::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("Missing client_id".to_string()))?;
    let project_id = req.query::<String>("project_id")
        .ok_or_else(|| AppError::BadRequest("Missing project_id".to_string()))?;
    let conversation_id = req.query::<String>("conversation_id");
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client_id".to_string()))?;
    
    // Query from database instead of filesystem
    let files = if let Some(conv_id) = conversation_id {
        sqlx::query_as::<_, FileUpload>(
            "SELECT * FROM file_uploads 
             WHERE client_id = $1 AND project_id = $2 AND conversation_id = $3 
             ORDER BY created_at DESC"
        )
        .bind(client_uuid)
        .bind(&project_id)
        .bind(&conv_id)
        .fetch_all(&state.db_pool)
        .await
    } else {
        sqlx::query_as::<_, FileUpload>(
            "SELECT * FROM file_uploads 
             WHERE client_id = $1 AND project_id = $2 
             ORDER BY created_at DESC"
        )
        .bind(client_uuid)
        .bind(&project_id)
        .fetch_all(&state.db_pool)
        .await
    }
    .map_err(|e| AppError::InternalServerError(format!("Failed to fetch files: {}", e)))?;
    
    let responses: Vec<FileUploadResponse> = files.iter()
        .map(|f| f.to_response())
        .collect();
    
    res.render(Json(responses));
    Ok(())
}

#[handler]
pub async fn handle_delete_upload(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    
    let file_id_str = req.param::<String>("id")
        .ok_or_else(|| AppError::BadRequest("Missing file id".to_string()))?;
    
    let file_id = Uuid::parse_str(&file_id_str)
        .map_err(|_| AppError::BadRequest("Invalid file id".to_string()))?;
    
    // Get client_id from query params for authorization
    let client_id_str = req.query::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("Missing client_id".to_string()))?;
    
    let client_id = Uuid::parse_str(&client_id_str)
        .map_err(|_| AppError::BadRequest("Invalid client_id".to_string()))?;
    
    // First get the file info to delete the physical file
    let file = sqlx::query_as::<_, FileUpload>(
        "SELECT * FROM file_uploads WHERE id = $1 AND client_id = $2"
    )
    .bind(file_id)
    .bind(client_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    .ok_or_else(|| AppError::NotFound("File not found".to_string()))?;
    
    // Delete the physical file
    if let Ok(path) = PathBuf::from_str(&file.file_path) {
        if path.exists() {
            if let Err(e) = fs::remove_file(&path).await {
                tracing::warn!("Failed to delete physical file {}: {}", file.file_path, e);
            }
        }
    }
    
    // Delete from database
    sqlx::query(
        "DELETE FROM file_uploads WHERE id = $1 AND client_id = $2"
    )
    .bind(file_id)
    .bind(client_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to delete file: {}", e)))?;
    
    res.render(Json(serde_json::json!({
        "message": "File deleted successfully",
        "id": file_id_str
    })));
    Ok(())
}