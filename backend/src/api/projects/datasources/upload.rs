use chrono::Utc;
use salvo::prelude::*;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use uuid::Uuid;

use crate::utils::middleware::{get_current_user_id, is_current_user_root};
use crate::utils::{get_app_state, AppError};

use super::types::{CreateDatasourceRequest, DatasourceResponse};

/// Upload a file and create a datasource
#[handler]
pub async fn upload_file_datasource(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;
    let project_id = req.param::<String>("project_id")
        .ok_or_else(|| AppError::BadRequest("Missing project_id".to_string()))?;

    // Validate project ownership
    let project_exists = if is_current_user_root(depot) {
        sqlx::query(
            "SELECT 1 FROM projects WHERE id = $1 AND deleted_at IS NULL"
        )
        .bind(&project_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    } else {
        sqlx::query(
            "SELECT 1 FROM projects WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL"
        )
        .bind(&project_id)
        .bind(user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    };

    if project_exists.is_none() {
        return Err(AppError::NotFound("Project not found".to_string()));
    }

    // Get form data
    let form_data = req.form_data().await
        .map_err(|e| AppError::BadRequest(format!("Invalid form data: {}", e)))?;

    // Extract fields from form
    let name = form_data.fields.get("name")
        .and_then(|v| v.first())
        .and_then(|s| s.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing name field"))?;

    let source_type = form_data.fields.get("source_type")
        .and_then(|v| v.first())
        .and_then(|s| s.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing source_type field"))?;

    // Validate source type
    let valid_types = ["csv", "excel", "json"];
    if !valid_types.contains(&source_type) {
        return Err(AppError::BadRequest(format!("Invalid source_type '{}'. Must be one of: {}", source_type, valid_types.join(", "))));
    }

    // Get file from form
    let file = form_data.files.get("file")
        .and_then(|files| files.first())
        .ok_or_else(|| AppError::BadRequest("Missing file field"))?;

    // Validate file
    let file_extension = Path::new(&file.name)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // Validate file extension matches source type
    let allowed_extensions = match source_type {
        "csv" => vec!["csv", "tsv", "txt"],
        "excel" => vec!["xlsx", "xls", "xlsm"],
        "json" => vec!["json", "jsonl"],
        _ => return Err(AppError::BadRequest("Invalid source type".to_string())),
    };

    if !allowed_extensions.contains(&file_extension.as_str()) {
        return Err(AppError::BadRequest(format!("File extension '.{}' is not allowed for source type '{}'. Allowed extensions: {}",
            file_extension, source_type, allowed_extensions.join(", "))));
    }

    // Create upload directory
    let upload_dir = format!("uploads/{}/datasources", project_id);
    fs::create_dir_all(&upload_dir)
        .map_err(|e| AppError::InternalServerError(format!("Failed to create upload directory: {}", e)))?;

    // Generate unique filename
    let datasource_id = Uuid::new_v4().to_string();
    let file_extension_with_dot = if file_extension.is_empty() {
        "".to_string()
    } else {
        format!(".{}", file_extension)
    };
    let file_name = format!("{}{}", datasource_id, file_extension_with_dot);
    let file_path = format!("{}/{}", upload_dir, file_name);

    // Save file
    fs::write(&file_path, &file.data)
        .map_err(|e| AppError::InternalServerError(format!("Failed to save file: {}", e)))?;

    // Get file metadata
    let file_metadata = fs::metadata(&file_path)
        .map_err(|e| AppError::InternalServerError(format!("Failed to get file metadata: {}", e)))?;
    let file_size = file_metadata.len();

    // Create connection config based on source type
    let connection_config = match source_type {
        "csv" => {
            // Extract parsing options from form
            let delimiter = form_data.fields.get("delimiter")
                .and_then(|v| v.first())
                .and_then(|s| s.as_str())
                .unwrap_or(",");

            let has_header = form_data.fields.get("has_header")
                .and_then(|v| v.first())
                .and_then(|s| s.as_str())
                .unwrap_or("true") == "true";

            json!({
                "file_path": file_path,
                "delimiter": delimiter,
                "has_header": has_header,
                "encoding": "utf-8",
                "skip_rows": 0,
                "quote_char": "\"",
                "flexible": false
            })
        },
        "excel" => {
            let sheet_name = form_data.fields.get("sheet_name")
                .and_then(|v| v.first())
                .and_then(|s| s.as_str());

            let header_row = form_data.fields.get("header_row")
                .and_then(|v| v.first())
                .and_then(|s| s.as_str())
                .and_then(|s| s.parse::<u64>().ok());

            let mut config = json!({
                "file_path": file_path,
                "data_start_row": 1
            });

            if let Some(sheet) = sheet_name {
                config["sheet_name"] = json!(sheet);
            }

            if let Some(header) = header_row {
                config["header_row"] = json!(header);
            }

            config
        },
        "json" => {
            let root_path = form_data.fields.get("root_path")
                .and_then(|v| v.first())
                .and_then(|s| s.as_str());

            let array_path = form_data.fields.get("array_path")
                .and_then(|v| v.first())
                .and_then(|s| s.as_str());

            let mut config = json!({
                "file_path": file_path
            });

            if let Some(root) = root_path {
                config["root_path"] = json!(root);
            }

            if let Some(array) = array_path {
                config["array_path"] = json!(array);
            }

            config
        },
        _ => return Err(AppError::BadRequest("Invalid source type".to_string())),
    };

    // Insert datasource with file metadata
    let now = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO data_sources (id, name, source_type, connection_config, project_id, file_path, file_size, file_type, file_metadata, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#
    )
    .bind(&datasource_id)
    .bind(name)
    .bind(source_type)
    .bind(&connection_config)
    .bind(&project_id)
    .bind(&file_path)
    .bind(file_size as i64)
    .bind(&file_extension)
    .bind(json!({
        "original_name": file.name,
        "uploaded_at": now.to_rfc3339(),
        "mime_type": file.mime_type
    }))
    .bind(now)
    .bind(now)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to create datasource: {}", e)))?;

    // Return the created datasource
    let created_datasource = DatasourceResponse {
        id: datasource_id,
        name: name.to_string(),
        source_type: source_type.to_string(),
        config: connection_config,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        project_id,
        schema_info: None,
        connection_status: Some("uploaded".to_string()),
        connection_error: None,
    };

    res.status_code(StatusCode::CREATED);
    res.render(Json(created_datasource));
    Ok(())
}

/// Preview a file before creating datasource
#[handler]
pub async fn preview_file(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;

    // Get form data
    let form_data = req.form_data().await
        .map_err(|e| AppError::BadRequest(format!("Invalid form data: {}", e)))?;

    // Get source type from form
    let source_type = form_data.fields.get("source_type")
        .and_then(|v| v.first())
        .and_then(|s| s.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing source_type field"))?;

    // Validate source type
    let valid_types = ["csv", "excel", "json"];
    if !valid_types.contains(&source_type) {
        return Err(AppError::BadRequest(format!("Invalid source_type '{}'. Must be one of: {}", source_type, valid_types.join(", "))));
    }

    // Get file from form
    let file = form_data.files.get("file")
        .and_then(|files| files.first())
        .ok_or_else(|| AppError::BadRequest("Missing file field"))?;

    // Create temporary file for preview
    let temp_dir = "temp/previews";
    fs::create_dir_all(temp_dir)
        .map_err(|e| AppError::InternalServerError(format!("Failed to create temp directory: {}", e)))?;

    let temp_filename = format!("preview_{}_{}", Uuid::new_v4(), file.name);
    let temp_path = format!("{}/{}", temp_dir, temp_filename);

    // Save temporary file
    fs::write(&temp_path, &file.data)
        .map_err(|e| AppError::InternalServerError(format!("Failed to save temp file: {}", e)))?;

    // Create a temporary connector to preview the data
    let preview_result = match source_type {
        "csv" => {
            let delimiter = form_data.fields.get("delimiter")
                .and_then(|v| v.first())
                .and_then(|s| s.as_str())
                .unwrap_or(",");

            let has_header = form_data.fields.get("has_header")
                .and_then(|v| v.first())
                .and_then(|s| s.as_str())
                .unwrap_or("true") == "true";

            let config = json!({
                "file_path": temp_path,
                "delimiter": delimiter,
                "has_header": has_header,
                "encoding": "utf-8",
                "skip_rows": 0,
                "quote_char": "\"",
                "flexible": false
            });

            preview_csv_data(&config).await
        },
        "excel" => {
            let sheet_name = form_data.fields.get("sheet_name")
                .and_then(|v| v.first())
                .and_then(|s| s.as_str());

            let header_row = form_data.fields.get("header_row")
                .and_then(|v| v.first())
                .and_then(|s| s.as_str())
                .and_then(|s| s.parse::<usize>().ok());

            let mut config = json!({
                "file_path": temp_path,
                "data_start_row": 0
            });

            if let Some(sheet) = sheet_name {
                config["sheet_name"] = json!(sheet);
            }

            if let Some(header) = header_row {
                config["header_row"] = json!(header);
                config["data_start_row"] = json!(header + 1);
            }

            preview_excel_data(&config).await
        },
        "json" => {
            let root_path = form_data.fields.get("root_path")
                .and_then(|v| v.first())
                .and_then(|s| s.as_str());

            let array_path = form_data.fields.get("array_path")
                .and_then(|v| v.first())
                .and_then(|s| s.as_str());

            let mut config = json!({
                "file_path": temp_path
            });

            if let Some(root) = root_path {
                config["root_path"] = json!(root);
            }

            if let Some(array) = array_path {
                config["array_path"] = json!(array);
            }

            preview_json_data(&config).await
        },
        _ => Err(AppError::BadRequest("Invalid source type".to_string())),
    };

    // Clean up temporary file
    let _ = fs::remove_file(&temp_path);

    match preview_result {
        Ok(preview) => {
            res.render(json!(preview));
            Ok(())
        },
        Err(e) => Err(e),
    }
}

// Preview functions (simplified versions that don't require the full connectors)
async fn preview_csv_data(config: &Value) -> Result<Value, AppError> {
    // This is a simplified preview - in a real implementation, you'd use the CSV connector
    Ok(json!({
        "preview_type": "csv",
        "estimated_rows": "unknown",
        "columns": ["Will be detected from file"],
        "sample_data": [],
        "config": config
    }))
}

async fn preview_excel_data(config: &Value) -> Result<Value, AppError> {
    // This is a simplified preview - in a real implementation, you'd use the Excel connector
    Ok(json!({
        "preview_type": "excel",
        "available_sheets": ["Sheet1", "Sheet2"], // Would be detected from file
        "estimated_rows": "unknown",
        "columns": ["Will be detected from file"],
        "sample_data": [],
        "config": config
    }))
}

async fn preview_json_data(config: &Value) -> Result<Value, AppError> {
    // This is a simplified preview - in a real implementation, you'd use the JSON connector
    Ok(json!({
        "preview_type": "json",
        "estimated_objects": "unknown",
        "structure": "Will be detected from file",
        "sample_data": [],
        "config": config
    }))
}