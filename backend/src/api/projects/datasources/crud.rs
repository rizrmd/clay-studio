use chrono::Utc;
use salvo::prelude::*;
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::core::datasources::cache::{get_datasource_cache, CachedDatasource};
use crate::utils::middleware::{get_current_user_id, is_current_user_root};
use crate::utils::{get_app_state, AppError};

use super::types::{CreateDatasourceRequest, DatasourceResponse, UpdateDatasourceRequest};

/// List all datasources for a project
#[handler]
pub async fn list_datasources(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;
    let project_id = req.param::<String>("project_id")
        .ok_or_else(|| AppError::BadRequest("Missing project_id".to_string()))?;

    // Validate project ownership (user owns project or is root)
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

    // Get datasources for the project
    let rows = sqlx::query(
        r#"
        SELECT id, name, source_type, connection_config as config, created_at, updated_at, project_id, schema_info, last_tested_at
        FROM data_sources 
        WHERE project_id = $1 AND deleted_at IS NULL
        ORDER BY created_at DESC
        "#
    )
    .bind(&project_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let datasources: Vec<DatasourceResponse> = rows
        .into_iter()
        .map(|row| {
            let config_json: Value = row.get("config");
            let schema_info_json: Option<Value> = row.get("schema_info");
            let last_tested_at: Option<chrono::DateTime<Utc>> = row.get("last_tested_at");
            
            // Determine connection status based on last_tested_at
            let connection_status = if last_tested_at.is_some() {
                Some("connected".to_string()) // If it was tested before, assume connected
            } else {
                Some("unknown".to_string()) // Never tested
            };
            
            DatasourceResponse {
                id: row.get("id"),
                name: row.get("name"),
                source_type: row.get("source_type"),
                config: config_json,
                created_at: row.get::<chrono::DateTime<Utc>, _>("created_at").to_rfc3339(),
                updated_at: row.get::<chrono::DateTime<Utc>, _>("updated_at").to_rfc3339(),
                project_id: row.get("project_id"),
                schema_info: schema_info_json,
                connection_status,
                connection_error: None, // TODO: Store connection errors in database
            }
        })
        .collect();

    res.render(Json(datasources));
    Ok(())
}

/// Create a new datasource
#[handler]
pub async fn create_datasource(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;
    let project_id = req.param::<String>("project_id")
        .ok_or_else(|| AppError::BadRequest("Missing project_id".to_string()))?;

    let request_data: CreateDatasourceRequest = req.parse_json().await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    // Validate project ownership (user owns project or is root)
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

    // Normalize and validate source_type
    let normalized_source_type = normalize_database_type(&request_data.source_type);
    
    let valid_types = ["postgresql", "mysql", "clickhouse", "sqlite", "oracle", "sqlserver", "csv", "excel", "json"];
    if !valid_types.contains(&normalized_source_type.as_str()) {
        return Err(AppError::BadRequest(format!("Invalid source_type '{}'. Must be one of: {}. Common variations are automatically normalized (e.g., 'postgres' → 'postgresql', 'MSSQL' → 'sqlserver', 'TSV' → 'csv')", request_data.source_type, valid_types.join(", "))));
    }

    let datasource_id = Uuid::new_v4().to_string();

    // Insert datasource
    let now = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO data_sources (id, name, source_type, connection_config, project_id, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#
    )
    .bind(&datasource_id)
    .bind(&request_data.name)
    .bind(&normalized_source_type)
    .bind(&request_data.config)
    .bind(&project_id)
    .bind(now)
    .bind(now)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to create datasource: {}", e)))?;

    // Return the created datasource
    let created_datasource = DatasourceResponse {
        id: datasource_id,
        name: request_data.name,
        source_type: normalized_source_type,
        config: request_data.config,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        project_id,
        schema_info: None,
        connection_status: Some("unknown".to_string()),
        connection_error: None,
    };

    res.status_code(StatusCode::CREATED);
    res.render(Json(created_datasource));
    Ok(())
}

/// Update a datasource
#[handler]
pub async fn update_datasource(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;

    let request_data: UpdateDatasourceRequest = req.parse_json().await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    // Check if datasource exists and belongs to user's project (or user is root)
    let existing = if is_current_user_root(depot) {
        sqlx::query(
            r#"
            SELECT ds.* 
            FROM data_sources ds
            JOIN projects p ON ds.project_id = p.id
            WHERE ds.id = $1 AND ds.deleted_at IS NULL AND p.deleted_at IS NULL
            "#
        )
        .bind(&datasource_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    } else {
        sqlx::query(
            r#"
            SELECT ds.* 
            FROM data_sources ds
            JOIN projects p ON ds.project_id = p.id
            WHERE ds.id = $1 AND p.user_id = $2 AND ds.deleted_at IS NULL AND p.deleted_at IS NULL
            "#
        )
        .bind(&datasource_id)
        .bind(user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    };

    let _existing_row = existing.ok_or_else(|| AppError::NotFound("Datasource not found".to_string()))?;

    if request_data.name.is_none() && request_data.config.is_none() {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    let now = Utc::now();
    
    // Handle different update scenarios
    let updated_row = match (&request_data.name, &request_data.config) {
        (Some(name), Some(config)) => {
            // When config changes, invalidate cache
            sqlx::query(
                "UPDATE data_sources SET name = $1, connection_config = $2, table_list = NULL, schema_info = NULL, updated_at = $3 WHERE id = $4 RETURNING *, connection_config as config, last_tested_at"
            )
            .bind(name)
            .bind(config)
            .bind(now)
            .bind(&datasource_id)
            .fetch_one(&state.db_pool)
            .await
            .map_err(|e| AppError::InternalServerError(format!("Failed to update datasource: {}", e)))?
        },
        (Some(name), None) => {
            // Name-only update doesn't affect cache
            sqlx::query(
                "UPDATE data_sources SET name = $1, updated_at = $2 WHERE id = $3 RETURNING *, connection_config as config, last_tested_at"
            )
            .bind(name)
            .bind(now)
            .bind(&datasource_id)
            .fetch_one(&state.db_pool)
            .await
            .map_err(|e| AppError::InternalServerError(format!("Failed to update datasource: {}", e)))?
        },
        (None, Some(config)) => {
            // When config changes, invalidate cache
            sqlx::query(
                "UPDATE data_sources SET connection_config = $1, table_list = NULL, schema_info = NULL, updated_at = $2 WHERE id = $3 RETURNING *, connection_config as config, last_tested_at"
            )
            .bind(config)
            .bind(now)
            .bind(&datasource_id)
            .fetch_one(&state.db_pool)
            .await
            .map_err(|e| AppError::InternalServerError(format!("Failed to update datasource: {}", e)))?
        },
        (None, None) => unreachable!(), // We already checked this case above
    };

    // Invalidate cache for this datasource
    let cache = get_datasource_cache().await;
    cache.invalidate(&datasource_id, None).await;

    // Return updated datasource
    let config_json: Value = updated_row.get("config");
    let schema_info_json: Option<Value> = updated_row.get("schema_info");
    let last_tested_at: Option<chrono::DateTime<Utc>> = updated_row.get("last_tested_at");
    
    // Determine connection status
    let connection_status = if last_tested_at.is_some() {
        Some("connected".to_string())
    } else {
        Some("unknown".to_string())
    };
    
    let updated_datasource = DatasourceResponse {
        id: updated_row.get("id"),
        name: updated_row.get("name"),
        source_type: updated_row.get("source_type"),
        config: config_json,
        created_at: updated_row.get::<chrono::DateTime<Utc>, _>("created_at").to_rfc3339(),
        updated_at: updated_row.get::<chrono::DateTime<Utc>, _>("updated_at").to_rfc3339(),
        project_id: updated_row.get("project_id"),
        schema_info: schema_info_json,
        connection_status,
        connection_error: None,
    };

    res.render(Json(updated_datasource));
    Ok(())
}

/// Delete a datasource (soft delete)
#[handler]
pub async fn delete_datasource(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;

    // Check if datasource exists and belongs to user's project (or user is root)
    let existing = if is_current_user_root(depot) {
        sqlx::query(
            r#"
            SELECT ds.id 
            FROM data_sources ds
            JOIN projects p ON ds.project_id = p.id
            WHERE ds.id = $1 AND ds.deleted_at IS NULL AND p.deleted_at IS NULL
            "#
        )
        .bind(&datasource_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    } else {
        sqlx::query(
            r#"
            SELECT ds.id 
            FROM data_sources ds
            JOIN projects p ON ds.project_id = p.id
            WHERE ds.id = $1 AND p.user_id = $2 AND ds.deleted_at IS NULL AND p.deleted_at IS NULL
            "#
        )
        .bind(&datasource_id)
        .bind(user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    };

    let _existing_row = existing.ok_or_else(|| AppError::NotFound("Datasource not found".to_string()))?;

    // Soft delete the datasource
    sqlx::query(
        "UPDATE data_sources SET deleted_at = $1 WHERE id = $2"
    )
    .bind(Utc::now())
    .bind(&datasource_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to delete datasource: {}", e)))?;

    // Invalidate cache for this datasource
    let cache = get_datasource_cache().await;
    cache.invalidate(&datasource_id, None).await;

    res.status_code(StatusCode::NO_CONTENT);
    Ok(())
}

/// Get a cached datasource with ownership validation
pub async fn get_cached_datasource(
    datasource_id: &str,
    user_id: &Uuid,
    is_root: bool,
    db_pool: &sqlx::PgPool,
) -> Result<CachedDatasource, AppError> {
    let cache = get_datasource_cache().await;
    
    // Try to get from cache first
    if let Some(cached) = cache.get(datasource_id, &user_id.to_string()).await {
        return Ok(cached);
    }
    
    // Cache miss - fetch from database
    let datasource_row = if is_root {
        sqlx::query(
            r#"
            SELECT ds.id, ds.name, ds.source_type, ds.connection_config, ds.project_id, p.user_id
            FROM data_sources ds
            JOIN projects p ON ds.project_id = p.id
            WHERE ds.id = $1 AND ds.deleted_at IS NULL AND p.deleted_at IS NULL
            "#
        )
        .bind(datasource_id)
        .fetch_optional(db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    } else {
        sqlx::query(
            r#"
            SELECT ds.id, ds.name, ds.source_type, ds.connection_config, ds.project_id, p.user_id
            FROM data_sources ds
            JOIN projects p ON ds.project_id = p.id
            WHERE ds.id = $1 AND p.user_id = $2 AND ds.deleted_at IS NULL AND p.deleted_at IS NULL
            "#
        )
        .bind(datasource_id)
        .bind(user_id)
        .fetch_optional(db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    };
    
    let row = datasource_row.ok_or_else(|| AppError::NotFound("Datasource not found".to_string()))?;
    
    // Parse connection config
    let connection_config: Value = row.get("connection_config");
    let owner_user_id: Uuid = row.get("user_id");
    
    // Create cached datasource
    let cached = CachedDatasource {
        id: row.get("id"),
        name: row.get("name"),
        datasource_type: row.get("source_type"),
        connection_config,
        user_id: owner_user_id,
        project_id: row.get("project_id"),
        cached_at: std::time::Instant::now(),
    };
    
    // Store in cache
    cache.set(cached.clone()).await;
    
    Ok(cached)
}

/// Normalize database type names to standard values
pub fn normalize_database_type(input: &str) -> String {
    // Convert to lowercase and remove spaces, hyphens, underscores
    let normalized = input
        .to_lowercase()
        .replace(" ", "")
        .replace("-", "")
        .replace("_", "");
    
    match normalized.as_str() {
        // PostgreSQL variations
        "postgres" | "postgresql" | "pgsql" | "pg" | "postgre" => "postgresql".to_string(),
        
        // MySQL variations
        "mysql" | "my" | "mariadb" | "maria" => "mysql".to_string(),
        
        // ClickHouse variations
        "clickhouse" | "click" | "ch" | "yandex" => "clickhouse".to_string(),
        
        // SQLite variations
        "sqlite" | "sqlite3" | "sql3" | "lite" => "sqlite".to_string(),
        
        // Oracle variations
        "oracle" | "oracledb" | "ora" | "orcl" => "oracle".to_string(),
        
        // SQL Server variations
        "sqlserver" | "mssql" | "microsoftsqlserver" | "microsoft" | "tsql" | "mssqlserver" => "sqlserver".to_string(),

        // CSV variations
        "csv" | "tsv" | "txt" | "delimited" => "csv".to_string(),

        // Excel variations
        "excel" | "xlsx" | "xls" | "xlsm" | "spreadsheet" => "excel".to_string(),

        // JSON variations
        "json" | "jsonl" | "ndjson" => "json".to_string(),

        // Return as-is if no match (will be caught by validation)
        _ => normalized,
    }
}