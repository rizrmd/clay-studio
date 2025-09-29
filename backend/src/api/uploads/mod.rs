// File upload functionality with content extraction

use salvo::prelude::*;
use salvo::fs::NamedFile;
use crate::models::file_upload::FileUpload;
use crate::utils::content_extractor::ContentExtractor;
use crate::utils::AppState;
use uuid::Uuid;
use std::fs;
use std::path::Path;
use chrono::Utc;

#[handler]
pub async fn handle_excel_download(req: &mut Request, res: &mut Response) -> Result<(), salvo::Error> {
    let client_id = req.param::<String>("client_id").ok_or_else(|| {
        salvo::Error::other("Missing client_id parameter")
    })?;
    
    let project_id = req.param::<String>("project_id").ok_or_else(|| {
        salvo::Error::other("Missing project_id parameter")
    })?;
    
    let export_id = req.param::<String>("export_id").ok_or_else(|| {
        salvo::Error::other("Missing export_id parameter")
    })?;

    // Find the Excel file in the excel_exports directory
    let excel_dir = format!(".clients/{}/{}/excel_exports", client_id, project_id);
    
    if !Path::new(&excel_dir).exists() {
        return Err(salvo::Error::other("Excel export directory not found"));
    }

    // Look for files that start with the export_id
    let dir_entries = fs::read_dir(&excel_dir).map_err(|e| {
        salvo::Error::other(format!("Failed to read excel directory: {}", e))
    })?;

    let mut excel_file_path = None;
    for entry in dir_entries {
        if let Ok(entry) = entry {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            if file_name_str.starts_with(&export_id) && file_name_str.ends_with(".xlsx") {
                excel_file_path = Some(entry.path());
                break;
            }
        }
    }

    let file_path = excel_file_path.ok_or_else(|| {
        salvo::Error::other("Excel file not found")
    })?;

    // Extract pretty filename from the file path
    // Format is: {export_id}_{pretty_name}.xlsx
    let file_name = file_path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("export.xlsx");
    
    let pretty_filename = if let Some(underscore_pos) = file_name.find('_') {
        // Extract everything after the first underscore
        &file_name[underscore_pos + 1..]
    } else {
        // Fallback to the full filename if no underscore found
        file_name
    };

    // Set Content-Disposition header for pretty download filename
    res.headers_mut().insert(
        "Content-Disposition",
        format!("attachment; filename=\"{}\"", pretty_filename).parse().unwrap()
    );

    let named_file = NamedFile::builder(file_path).build().await.map_err(|e| {
        salvo::Error::other(format!("Failed to serve excel file: {}", e))
    })?;

    named_file.send(req.headers(), res).await;
    Ok(())
}

pub fn upload_routes() -> Router {
    Router::new()
        .push(Router::with_path("/upload").post(handle_file_upload))
        .push(Router::with_path("/uploads").get(handle_list_uploads))
        .push(Router::with_path("/uploads/{client_id}/{project_id}/{file_name}").get(handle_file_download))
        .push(Router::with_path("/uploads/{file_id}").delete(handle_delete_upload))
        .push(Router::with_path("/uploads/{file_id}/description").put(handle_update_file_description))
        .push(Router::with_path("/files/excel/{client_id}/{project_id}/{export_id}").get(handle_excel_download))
}

#[handler]
pub async fn handle_file_upload(req: &mut Request, res: &mut Response, depot: &mut Depot) -> Result<(), salvo::Error> {
    let state = depot.obtain::<AppState>().map_err(|_| {
        salvo::Error::other("App state not found")
    })?;

    // Get client_id and project_id from query params
    let client_id = req.query::<String>("client_id").ok_or_else(|| {
        salvo::Error::other("Missing client_id parameter")
    })?;
    
    let project_id = req.query::<String>("project_id").ok_or_else(|| {
        salvo::Error::other("Missing project_id parameter")
    })?;

    // Parse client_id as UUID
    let client_uuid = Uuid::parse_str(&client_id).map_err(|_| {
        salvo::Error::other("Invalid client_id format")
    })?;

    // Handle multipart file upload
    let file = req.file("file").await.ok_or_else(|| {
        salvo::Error::other("No file provided")
    })?;

    let original_name = file.name().unwrap_or("unknown").to_string();
    let mime_type = file.content_type().map(|ct| ct.to_string());
    let file_size = file.size();

    // Create upload directory
    let upload_dir = format!(".clients/{}/{}/uploads", client_id, project_id);
    fs::create_dir_all(&upload_dir).map_err(|e| {
        salvo::Error::other(format!("Failed to create upload directory: {}", e))
    })?;

    // Generate unique filename
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
    let file_path = format!("{}/{}", upload_dir, stored_filename);

    // Save file to filesystem
    let temp_path = file.path();
    fs::copy(temp_path, &file_path).map_err(|e| {
        salvo::Error::other(format!("Failed to save file: {}", e))
    })?;

    // Extract content using our content extractor with large file handling
    let extracted = ContentExtractor::extract_content(
        Path::new(&file_path),
        &original_name,
        &mime_type.clone().unwrap_or_else(|| "application/octet-stream".to_string()),
    ).await.map_err(|e| {
        salvo::Error::other(format!("Content extraction failed: {}", e))
    })?;

    // Get file size for response metadata
    let file_metadata = fs::metadata(&file_path).map_err(|e| {
        salvo::Error::other(format!("Failed to get file metadata: {}", e))
    })?;
    let file_size_mb = file_metadata.len() as f64 / (1024.0 * 1024.0);
    let limits = ContentExtractor::get_limits();
    let is_large_file = file_metadata.len() > limits.max_full_parse_size;

    // Create file upload record
    let file_upload = FileUpload {
        id: file_id,
        client_id: client_uuid,
        project_id: project_id.clone(),
        conversation_id: None, // Will be set when associated with a message
        file_name: stored_filename.clone(),
        original_name: original_name.clone(),
        file_path: file_path.clone(),
        file_size: file_size as i64,
        mime_type: mime_type.clone(),
        description: None,
        auto_description: extracted.description.clone(),
        file_content: extracted.text_content.clone(),
        metadata: extracted.structured_data.clone(),
        uploaded_by: None, // TODO: Get from auth context
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Save to database
    sqlx::query(
        "INSERT INTO file_uploads 
        (id, client_id, project_id, file_name, original_name, file_path, file_size, 
         mime_type, auto_description, file_content, metadata, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"
    )
    .bind(file_upload.id)
    .bind(file_upload.client_id)
    .bind(&file_upload.project_id)
    .bind(&file_upload.file_name)
    .bind(&file_upload.original_name)
    .bind(&file_upload.file_path)
    .bind(file_upload.file_size)
    .bind(&file_upload.mime_type)
    .bind(&file_upload.auto_description)
    .bind(&file_upload.file_content)
    .bind(&file_upload.metadata)
    .bind(file_upload.created_at)
    .bind(file_upload.updated_at)
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        salvo::Error::other(format!("Database error: {}", e))
    })?;

    // Return success response with file info and large file handling details
    let mut response = serde_json::json!({
        "id": file_upload.id,
        "file_name": file_upload.file_name,
        "original_name": file_upload.original_name,
        "file_size": file_upload.file_size,
        "file_size_mb": file_size_mb,
        "mime_type": file_upload.mime_type,
        "description": file_upload.auto_description,
        "auto_description": file_upload.auto_description,
        "has_text_content": file_upload.file_content.is_some(),
        "preview": extracted.preview,
        "created_at": file_upload.created_at,
        "is_large_file": is_large_file
    });

    // Add large file handling info if applicable
    if is_large_file {
        response["processing_limits"] = serde_json::json!({
            "max_parse_size_mb": limits.max_full_parse_size as f64 / (1024.0 * 1024.0),
            "content_extraction_skipped": true,
            "reason": "File exceeds maximum processing size"
        });
    }

    // Add content truncation info if present in metadata
    if let Some(ref structured_data) = extracted.structured_data {
        if structured_data.get("content_truncated").and_then(|v| v.as_bool()).unwrap_or(false) {
            response["content_truncated"] = serde_json::json!(true);
            response["original_content_length"] = structured_data.get("original_length").cloned().unwrap_or_default();
        }
    }

    res.render(Json(response));
    Ok(())
}

#[handler]
pub async fn handle_list_uploads(req: &mut Request, res: &mut Response, depot: &mut Depot) -> Result<(), salvo::Error> {
    let state = depot.obtain::<AppState>().map_err(|_| {
        salvo::Error::other("App state not found")
    })?;

    let client_id = req.query::<String>("client_id").ok_or_else(|| {
        salvo::Error::other("Missing client_id parameter")
    })?;
    
    let project_id = req.query::<String>("project_id").ok_or_else(|| {
        salvo::Error::other("Missing project_id parameter")
    })?;

    let client_uuid = Uuid::parse_str(&client_id).map_err(|_| {
        salvo::Error::other("Invalid client_id format")
    })?;

    let files = sqlx::query_as::<_, FileUpload>(
        "SELECT * FROM file_uploads 
         WHERE client_id = $1 AND project_id = $2 
         ORDER BY created_at DESC"
    )
    .bind(client_uuid)
    .bind(project_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        salvo::Error::other(format!("Database error: {}", e))
    })?;

    let file_responses: Vec<_> = files.iter().map(|f| f.to_response()).collect();
    res.render(Json(file_responses));
    Ok(())
}

#[handler]
pub async fn handle_file_download(req: &mut Request, res: &mut Response) -> Result<(), salvo::Error> {
    let client_id = req.param::<String>("client_id").ok_or_else(|| {
        salvo::Error::other("Missing client_id parameter")
    })?;
    
    let project_id = req.param::<String>("project_id").ok_or_else(|| {
        salvo::Error::other("Missing project_id parameter")
    })?;
    
    let file_name = req.param::<String>("file_name").ok_or_else(|| {
        salvo::Error::other("Missing file_name parameter")
    })?;

    let file_path = format!(".clients/{}/{}/uploads/{}", client_id, project_id, file_name);
    
    if !Path::new(&file_path).exists() {
        return Err(salvo::Error::other("File not found"));
    }

    let named_file = NamedFile::builder(file_path).build().await.map_err(|e| {
        salvo::Error::other(format!("Failed to serve file: {}", e))
    })?;

    named_file.send(req.headers(), res).await;
    Ok(())
}

#[handler]
pub async fn handle_delete_upload(req: &mut Request, res: &mut Response, depot: &mut Depot) -> Result<(), salvo::Error> {
    let state = depot.obtain::<AppState>().map_err(|_| {
        salvo::Error::other("App state not found")
    })?;

    let file_id = req.param::<String>("file_id").ok_or_else(|| {
        salvo::Error::other("Missing file_id parameter")
    })?;

    let file_uuid = Uuid::parse_str(&file_id).map_err(|_| {
        salvo::Error::other("Invalid file_id format")
    })?;

    // Get file info before deletion
    let file_info = sqlx::query_as::<_, FileUpload>(
        "SELECT * FROM file_uploads WHERE id = $1"
    )
    .bind(file_uuid)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        salvo::Error::other(format!("Database error: {}", e))
    })?;

    if let Some(file) = file_info {
        // Delete file from filesystem
        if Path::new(&file.file_path).exists() {
            fs::remove_file(&file.file_path).map_err(|e| {
                salvo::Error::other(format!("Failed to delete file: {}", e))
            })?;
        }

        // Delete from database
        sqlx::query("DELETE FROM file_uploads WHERE id = $1")
            .bind(file_uuid)
            .execute(&state.db_pool)
            .await
            .map_err(|e| {
                salvo::Error::other(format!("Database error: {}", e))
            })?;

        res.render(Json(serde_json::json!({
            "success": true,
            "message": "File deleted successfully"
        })));
    } else {
        return Err(salvo::Error::other("File not found"));
    }

    Ok(())
}

#[handler]
pub async fn handle_update_file_description(req: &mut Request, res: &mut Response, depot: &mut Depot) -> Result<(), salvo::Error> {
    let state = depot.obtain::<AppState>().map_err(|_| {
        salvo::Error::other("App state not found")
    })?;

    let file_id = req.param::<String>("file_id").ok_or_else(|| {
        salvo::Error::other("Missing file_id parameter")
    })?;

    let file_uuid = Uuid::parse_str(&file_id).map_err(|_| {
        salvo::Error::other("Invalid file_id format")
    })?;

    let body: serde_json::Value = req.parse_json().await.map_err(|_| {
        salvo::Error::other("Invalid JSON body")
    })?;

    let description = body.get("description")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            salvo::Error::other("Missing description field")
        })?;

    sqlx::query(
        "UPDATE file_uploads SET description = $1, updated_at = $2 WHERE id = $3"
    )
    .bind(description)
    .bind(Utc::now())
    .bind(file_uuid)
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        salvo::Error::other(format!("Database error: {}", e))
    })?;

    res.render(Json(serde_json::json!({
        "success": true,
        "message": "Description updated successfully"
    })));
    
    Ok(())
}