use chrono::Utc;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::utils::middleware::{get_current_user_id, is_current_user_root};
use crate::utils::{get_app_state, AppError};
use crate::utils::datasource::get_pool_manager;
use crate::core::datasources::cache::{get_datasource_cache, CachedDatasource};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDatasourceRequest {
    pub name: String,
    pub source_type: String, // postgresql, mysql, clickhouse, sqlite, oracle, sqlserver
    pub config: Value, // Can be string (URL) or object (individual fields)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateDatasourceRequest {
    pub name: Option<String>,
    pub config: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasourceResponse {
    pub id: String,
    pub name: String,
    pub source_type: String,
    pub config: Value,
    pub created_at: String,
    pub updated_at: String,
    pub project_id: String,
    pub schema_info: Option<Value>,
    pub connection_status: Option<String>,
    pub connection_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryRequest {
    pub query: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableDataRequest {
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub sort_column: Option<String>,
    pub sort_direction: Option<String>, // "asc" or "desc"
    pub filters: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DistinctValuesRequest {
    pub column: String,
    pub limit: Option<i32>, // Limit number of distinct values returned
    pub search: Option<String>, // Optional search filter
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteRowsRequest {
    pub row_ids: Vec<String>, // IDs or conditions to identify rows to delete
    pub id_column: Option<String>, // Primary key column name (defaults to 'id')
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRowsRequest {
    pub updates: std::collections::HashMap<String, std::collections::HashMap<String, Value>>, // rowId -> columnKey -> newValue
    pub id_column: Option<String>, // Primary key column name (defaults to 'id')
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsertRowsRequest {
    pub rows: Vec<std::collections::HashMap<String, Value>>, // Array of row objects
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableColumn {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub column_default: Option<String>,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
    pub character_maximum_length: Option<i32>,
    pub numeric_precision: Option<i32>,
    pub numeric_scale: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ForeignKeyInfo {
    pub column_name: String,
    pub referenced_table: String,
    pub referenced_column: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableStructure {
    pub table_name: String,
    pub columns: Vec<TableColumn>,
    pub primary_keys: Vec<String>,
    pub foreign_keys: Vec<ForeignKeyInfo>,
    pub indexes: Vec<IndexInfo>,
}

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
        .bind(&user_id)
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
        .bind(&user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    };

    if project_exists.is_none() {
        return Err(AppError::NotFound("Project not found".to_string()));
    }

    // Normalize and validate source_type
    let normalized_source_type = normalize_database_type(&request_data.source_type);
    
    let valid_types = ["postgresql", "mysql", "clickhouse", "sqlite", "oracle", "sqlserver"];
    if !valid_types.contains(&normalized_source_type.as_str()) {
        return Err(AppError::BadRequest(format!("Invalid source_type '{}'. Must be one of: {}. Common variations are automatically normalized (e.g., 'postgres' → 'postgresql', 'MSSQL' → 'sqlserver')", request_data.source_type, valid_types.join(", "))));
    }

    let datasource_id = Uuid::new_v4().to_string();
    let config_json = serde_json::to_string(&request_data.config)
        .map_err(|e| AppError::BadRequest(format!("Invalid config format: {}", e)))?;

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
    .bind(&config_json)
    .bind(&project_id)
    .bind(&now)
    .bind(&now)
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
        .bind(&user_id)
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
            let config_json = serde_json::to_string(config)
                .map_err(|e| AppError::BadRequest(format!("Invalid config format: {}", e)))?;
            sqlx::query(
                "UPDATE data_sources SET name = $1, connection_config = $2, updated_at = $3 WHERE id = $4 RETURNING *, connection_config as config, last_tested_at"
            )
            .bind(name)
            .bind(&config_json)
            .bind(&now)
            .bind(&datasource_id)
            .fetch_one(&state.db_pool)
            .await
            .map_err(|e| AppError::InternalServerError(format!("Failed to update datasource: {}", e)))?
        },
        (Some(name), None) => {
            sqlx::query(
                "UPDATE data_sources SET name = $1, updated_at = $2 WHERE id = $3 RETURNING *, connection_config as config, last_tested_at"
            )
            .bind(name)
            .bind(&now)
            .bind(&datasource_id)
            .fetch_one(&state.db_pool)
            .await
            .map_err(|e| AppError::InternalServerError(format!("Failed to update datasource: {}", e)))?
        },
        (None, Some(config)) => {
            let config_json = serde_json::to_string(config)
                .map_err(|e| AppError::BadRequest(format!("Invalid config format: {}", e)))?;
            sqlx::query(
                "UPDATE data_sources SET connection_config = $1, updated_at = $2 WHERE id = $3 RETURNING *, connection_config as config, last_tested_at"
            )
            .bind(&config_json)
            .bind(&now)
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
        .bind(&user_id)
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

/// Test connection with arbitrary config (for form validation)
#[handler]
pub async fn test_connection_with_config(
    req: &mut Request,
    res: &mut Response,
    _depot: &mut Depot,
) -> Result<(), AppError> {
    #[derive(Debug, Serialize, Deserialize)]
    struct TestConfigRequest {
        source_type: String,
        config: Value,
    }

    let test_data: TestConfigRequest = req.parse_json().await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    // Normalize source type
    let normalized_source_type = normalize_database_type(&test_data.source_type);
    
    // Test connection based on source type
    let test_result = match normalized_source_type.as_str() {
        "postgresql" => test_postgres_connection(&test_data.config).await,
        "mysql" => test_mysql_connection(&test_data.config).await,
        "sqlite" => test_sqlite_connection(&test_data.config).await,
        _ => TestConnectionResponse {
            success: false,
            message: format!("Connection testing not implemented for {}", normalized_source_type),
            error: Some("Not implemented".to_string()),
        }
    };

    res.render(Json(test_result));
    Ok(())
}

/// Test connection to a datasource
#[handler]
pub async fn test_connection(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;

    // Get datasource and verify ownership using cache
    let cached_datasource = get_cached_datasource(&datasource_id, &user_id, is_current_user_root(depot), &state.db_pool).await?;
    
    let source_type = cached_datasource.datasource_type.clone();
    let config = cached_datasource.connection_config.clone();

    // Test connection based on source type
    let test_result = match source_type.as_str() {
        "postgresql" => test_postgres_connection(&config).await,
        "mysql" => test_mysql_connection(&config).await,
        "sqlite" => test_sqlite_connection(&config).await,
        _ => TestConnectionResponse {
            success: false,
            message: format!("Connection testing not implemented for {}", source_type),
            error: Some("Not implemented".to_string()),
        }
    };

    res.render(Json(test_result));
    Ok(())
}

/// Get schema information for a datasource
#[handler]
pub async fn get_schema(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;

    // Get datasource and verify ownership using cache
    let _cached_datasource = get_cached_datasource(&datasource_id, &user_id, is_current_user_root(depot), &state.db_pool).await?;

    // For schema info, we still need to query the database since schema_info is not cached
    let datasource_row = sqlx::query("SELECT schema_info FROM data_sources WHERE id = $1")
        .bind(&datasource_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound("Datasource not found".to_string()))?;

    let schema_info_str: Option<String> = datasource_row.get("schema_info");
    let schema_info: Option<Value> = schema_info_str
        .and_then(|s| serde_json::from_str(&s).ok());

    if let Some(schema) = schema_info {
        res.render(Json(schema));
    } else {
        res.render(Json(serde_json::json!({
            "message": "No schema information available"
        })));
    }
    
    Ok(())
}

/// Execute a custom query on a datasource
#[handler]
pub async fn execute_query(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;

    let request_data: QueryRequest = req.parse_json().await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    // Get datasource and verify ownership using cache
    let cached_datasource = get_cached_datasource(&datasource_id, &user_id, is_current_user_root(depot), &state.db_pool).await?;
    
    let source_type = cached_datasource.datasource_type.clone();
    let config = cached_datasource.connection_config.clone();

    // Execute query based on source type using cached connection pools
    let result = match source_type.as_str() {
        "postgresql" => {
            // Use a very high limit when no limit is specified (effectively unlimited)
            let limit = request_data.limit.unwrap_or(1000000);
            execute_custom_query(&datasource_id, &config, &request_data.query, limit).await
                .map_err(|e| AppError::InternalServerError(format!("Query execution failed: {}", e)))?
        },
        _ => {
            return Err(AppError::BadRequest(format!("Unsupported datasource type: {}", source_type)));
        }
    };

    res.render(Json(result));
    Ok(())
}

/// Get table data with pagination and sorting
#[handler]
pub async fn get_table_data(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let request_start = std::time::Instant::now();
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;
    let table_name = req.param::<String>("table_name")
        .ok_or_else(|| AppError::BadRequest("Missing table_name".to_string()))?;

    let request_data: TableDataRequest = req.parse_json().await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;
    
    let parse_time = request_start.elapsed().as_millis();
    tracing::info!("Request parsing took {}ms", parse_time);

    // Get datasource and verify ownership using cache
    let db_query_start = std::time::Instant::now();
    let cached_datasource = get_cached_datasource(&datasource_id, &user_id, is_current_user_root(depot), &state.db_pool).await?;
    let db_query_time = db_query_start.elapsed().as_millis();
    tracing::info!("Datasource validation query took {}ms", db_query_time);

    let source_type = cached_datasource.datasource_type.clone();
    let config = cached_datasource.connection_config.clone();

    // Get pagination parameters
    let page = request_data.page.unwrap_or(1);
    let limit = request_data.limit.unwrap_or(50);

    // Execute query based on source type using cached connection pools
    let result = match source_type.as_str() {
        "postgresql" => {
            execute_table_data_query(&datasource_id, &config, &table_name, page, limit, 
                                   request_data.sort_column.as_deref(), 
                                   request_data.sort_direction.as_deref(),
                                   request_data.filters.as_ref()).await
                .map_err(|e| AppError::InternalServerError(format!("Query execution failed: {}", e)))?
        },
        _ => {
            return Err(AppError::BadRequest(format!("Unsupported datasource type: {}", source_type)));
        }
    };

    let total_time = request_start.elapsed().as_millis();
    tracing::info!("Total request took {}ms", total_time);

    // Add timing breakdown to response
    if let Value::Object(mut result_obj) = result {
        result_obj.insert("total_api_time_ms".to_string(), Value::Number(serde_json::Number::from(total_time as u64)));
        result_obj.insert("validation_time_ms".to_string(), Value::Number(serde_json::Number::from(db_query_time as u64)));
        
        // Calculate API overhead
        let db_execution_time = result_obj.get("execution_time_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u128;
        let api_overhead = total_time.saturating_sub(db_execution_time);
        result_obj.insert("api_overhead_ms".to_string(), Value::Number(serde_json::Number::from(api_overhead as u64)));
        
        res.render(Json(Value::Object(result_obj)));
    } else {
        res.render(Json(result));
    }
    Ok(())
}

/// Get list of tables for a datasource
#[handler]
pub async fn get_tables(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;

    // First check if we have cached table list
    let table_list_row = sqlx::query("SELECT table_list FROM data_sources WHERE id = $1")
        .bind(&datasource_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    if let Some(row) = table_list_row {
        let cached_table_list: Option<Value> = row.get("table_list");
        if let Some(table_list_value) = cached_table_list {
            if let Ok(tables) = serde_json::from_value::<Vec<String>>(table_list_value) {
                res.render(Json(tables));
                return Ok(());
            }
        }
    }

    // Get datasource and verify ownership using cache
    let cached_datasource = get_cached_datasource(&datasource_id, &user_id, is_current_user_root(depot), &state.db_pool).await?;
    
    let source_type = cached_datasource.datasource_type.clone();
    let config = cached_datasource.connection_config.clone();

    // Get tables based on source type using cached connection pools
    let result = match source_type.as_str() {
        "postgresql" => {
            list_tables(&datasource_id, &config).await
                .map_err(|e| AppError::InternalServerError(format!("Failed to list tables: {}", e)))?
        },
        _ => {
            return Err(AppError::BadRequest(format!("Unsupported datasource type: {}", source_type)));
        }
    };

    // Update the table_list in database
    let table_list_json = serde_json::to_value(&result)
        .map_err(|e| AppError::InternalServerError(format!("Failed to serialize table list: {}", e)))?;
    
    sqlx::query("UPDATE data_sources SET table_list = $1, updated_at = NOW() WHERE id = $2")
        .bind(&table_list_json)
        .bind(&datasource_id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to update table list: {}", e)))?;

    res.render(Json(result));
    Ok(())
}

/// Get table structure information (columns, constraints, indexes)
#[handler]
pub async fn get_table_structure(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;
    let table_name = req.param::<String>("table_name")
        .ok_or_else(|| AppError::BadRequest("Missing table_name".to_string()))?;

    // First check if we have cached schema info for this table
    let schema_info_row = sqlx::query("SELECT schema_info FROM data_sources WHERE id = $1")
        .bind(&datasource_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    if let Some(row) = schema_info_row {
        let cached_schema_info: Option<Value> = row.get("schema_info");
        if let Some(schema_value) = cached_schema_info {
            if let Some(tables_obj) = schema_value.get("tables") {
                if let Some(table_structure) = tables_obj.get(&table_name) {
                    if let Ok(structure) = serde_json::from_value::<TableStructure>(table_structure.clone()) {
                        res.render(Json(structure));
                        return Ok(());
                    }
                }
            }
        }
    }

    // Get datasource and verify ownership using cache
    let cached_datasource = get_cached_datasource(&datasource_id, &user_id, is_current_user_root(depot), &state.db_pool).await?;
    
    let source_type = cached_datasource.datasource_type.clone();
    let config = cached_datasource.connection_config.clone();

    // Get table structure based on source type using cached connection pools
    let result = match source_type.as_str() {
        "postgresql" => {
            get_postgres_table_structure(&datasource_id, &config, &table_name).await
                .map_err(|e| AppError::InternalServerError(format!("Failed to get table structure: {}", e)))?
        },
        _ => {
            return Err(AppError::BadRequest(format!("Unsupported datasource type: {}", source_type)));
        }
    };

    // Update schema_info with the new table structure
    update_schema_info_with_table_structure(&state.db_pool, &datasource_id, &table_name, &result).await
        .map_err(|e| AppError::InternalServerError(format!("Failed to update schema info: {}", e)))?;

    res.render(Json(result));
    Ok(())
}

// Helper functions for connection testing
async fn test_postgres_connection(config: &Value) -> TestConnectionResponse {
    let connection_url = if let Some(url) = config.as_str() {
        url.to_string()
    } else if let Some(obj) = config.as_object() {
        // If object has a 'url' field, use that directly
        if let Some(url) = obj.get("url").and_then(|v| v.as_str()) {
            url.to_string()
        } else {
            // Build connection URL from individual fields
            let host = obj.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
            let port = obj.get("port").and_then(|v| v.as_u64()).unwrap_or(5432);
            let database = obj.get("database").and_then(|v| v.as_str()).unwrap_or("");
            let user = obj.get("user").and_then(|v| v.as_str())
                .or_else(|| obj.get("username").and_then(|v| v.as_str()))
                .unwrap_or("");
            let password = obj.get("password").and_then(|v| v.as_str()).unwrap_or("");
            
            format!("postgresql://{}:{}@{}:{}/{}", user, password, host, port, database)
        }
    } else {
        return TestConnectionResponse {
            success: false,
            message: "Invalid configuration format".to_string(),
            error: Some("Config must be a connection URL string or object with connection details".to_string()),
        };
    };

    match sqlx::postgres::PgPool::connect(&connection_url).await {
        Ok(pool) => {
            // Test with a simple query
            match sqlx::query("SELECT 1").fetch_one(&pool).await {
                Ok(_) => TestConnectionResponse {
                    success: true,
                    message: "Connection successful".to_string(),
                    error: None,
                },
                Err(e) => TestConnectionResponse {
                    success: false,
                    message: "Connection failed".to_string(),
                    error: Some(e.to_string()),
                }
            }
        }
        Err(e) => TestConnectionResponse {
            success: false,
            message: "Connection failed".to_string(),
            error: Some(e.to_string()),
        }
    }
}

async fn test_mysql_connection(_config: &Value) -> TestConnectionResponse {
    // For now, return not implemented
    // In a real implementation, you would use sqlx::mysql::MySqlPool
    TestConnectionResponse {
        success: false,
        message: "MySQL connection testing not implemented yet".to_string(),
        error: Some("Feature not implemented".to_string()),
    }
}

async fn test_sqlite_connection(_config: &Value) -> TestConnectionResponse {
    // For now, return not implemented
    // In a real implementation, you would use sqlx::sqlite::SqlitePool
    TestConnectionResponse {
        success: false,
        message: "SQLite connection testing not implemented yet".to_string(),
        error: Some("Feature not implemented".to_string()),
    }
}

/// Normalize database type strings to handle common variations
fn normalize_database_type(input: &str) -> String {
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
        
        // Return as-is if no match (will be caught by validation)
        _ => normalized,
    }
}

/// Optimized table data query using cached connection pools
async fn execute_table_data_query(
    datasource_id: &str, 
    config: &Value, 
    table_name: &str, 
    page: i32, 
    limit: i32,
    sort_column: Option<&str>,
    sort_direction: Option<&str>,
    filters: Option<&Value>
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    use sqlx::{Row as SqlxRow, Column};
    use serde_json::json;
    use std::time::Instant;
    
    let pool_start = Instant::now();
    let pool_manager = get_pool_manager().await;
    let pool = pool_manager.get_pool(datasource_id, config).await?;
    let pool_time = pool_start.elapsed().as_millis() as u64;
    
    let start = Instant::now();
    
    // Build WHERE clause from filters
    let mut where_conditions = Vec::new();
    let mut query_params: Vec<String> = Vec::new();
    
    if let Some(filters_value) = filters {
        if let Some(filters_obj) = filters_value.as_object() {
            for (column_name, filter_value) in filters_obj {
                if column_name == "global" {
                    // Handle global filter - search across all text columns
                    // For now, skip global filter in this simple implementation
                    continue;
                }
                
                // Handle column-specific filters
                if let Some(value_str) = filter_value.as_str() {
                    if !value_str.is_empty() {
                        // Use ILIKE for case-insensitive partial matching (PostgreSQL)
                        where_conditions.push(format!("{} ILIKE ${}", column_name, query_params.len() + 1));
                        query_params.push(format!("%{}%", value_str));
                    }
                } else if let Some(value_array) = filter_value.as_array() {
                    // Handle array filters (e.g., multi-select)
                    if !value_array.is_empty() {
                        let mut in_conditions = Vec::new();
                        for val in value_array {
                            if let Some(val_str) = val.as_str() {
                                in_conditions.push(format!("${}", query_params.len() + 1));
                                query_params.push(val_str.to_string());
                            }
                        }
                        if !in_conditions.is_empty() {
                            where_conditions.push(format!("{} IN ({})", column_name, in_conditions.join(", ")));
                        }
                    }
                }
            }
        }
    }
    
    let where_clause = if where_conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", where_conditions.join(" AND "))
    };
    
    // First, get the total count with filters
    let count_start = Instant::now();
    let count_query = format!("SELECT COUNT(*) as total FROM {}{}", table_name, where_clause);
    
    let mut count_query_builder = sqlx::query(&count_query);
    for param in &query_params {
        count_query_builder = count_query_builder.bind(param);
    }
    
    let count_row = count_query_builder
        .fetch_one(pool.as_ref())
        .await?;
    let total_rows: i64 = count_row.try_get("total")?;
    let count_time = count_start.elapsed().as_millis() as u64;
    
    // Build the data query
    let offset = (page - 1) * limit;
    let mut query = format!("SELECT * FROM {}{}", table_name, where_clause);
    
    // Add sorting if specified
    if let Some(sort_col) = sort_column {
        let direction = sort_direction.unwrap_or("ASC");
        query.push_str(&format!(" ORDER BY {} {}", sort_col, direction));
    }
    
    // Add pagination
    query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));
    
    // Execute the data query
    let data_start = Instant::now();
    let mut data_query_builder = sqlx::query(&query);
    for param in &query_params {
        data_query_builder = data_query_builder.bind(param);
    }
    
    let rows = data_query_builder
        .fetch_all(pool.as_ref())
        .await?;
    let data_time = data_start.elapsed().as_millis() as u64;

    let execution_time_ms = start.elapsed().as_millis() as u64;

    // Get column names - either from first row or from table structure query
    let columns: Vec<String> = if !rows.is_empty() {
        // Get column names from the first row if we have data
        let first_row = &rows[0];
        first_row.columns().iter().map(|c| c.name().to_string()).collect()
    } else {
        // For empty results, get column names from table structure
        let structure_query = format!("SELECT * FROM {} LIMIT 0", table_name);
        match sqlx::query(&structure_query).fetch_optional(pool.as_ref()).await {
            Ok(Some(row)) => {
                // We got a row with column structure (should have 0 rows but column info)
                row.columns().iter().map(|c| c.name().to_string()).collect()
            }
            Ok(None) | Err(_) => {
                // If the structure query fails, return empty columns (table might not exist)
                vec![]
            }
        }
    };

    if rows.is_empty() {
        return Ok(json!({
            "columns": columns,
            "rows": [],
            "row_count": rows.len(),
            "total_rows": total_rows,
            "execution_time_ms": execution_time_ms,
            "timing_breakdown": {
                "pool_access_ms": pool_time,
                "count_query_ms": count_time,
                "data_query_ms": data_time,
                "total_db_ms": execution_time_ms
            },
            "page": page,
            "page_size": limit
        }));
    }

    // Convert rows to JSON
    let mut result_rows = Vec::new();
    for row in rows.iter() {
        let mut row_data = Vec::new();
        for (i, _col) in columns.iter().enumerate() {
            // Try to get value as different types, including PostgreSQL NUMERIC and UUID
            if let Ok(val) = row.try_get::<String, _>(i) {
                row_data.push(val);
            } else if let Ok(val) = row.try_get::<sqlx::types::Uuid, _>(i) {
                row_data.push(val.to_string());
            } else if let Ok(val) = row.try_get::<rust_decimal::Decimal, _>(i) {
                row_data.push(val.to_string());
            } else if let Ok(val) = row.try_get::<sqlx::types::BigDecimal, _>(i) {
                row_data.push(val.to_string());
            } else if let Ok(val) = row.try_get::<i32, _>(i) {
                row_data.push(val.to_string());
            } else if let Ok(val) = row.try_get::<i64, _>(i) {
                row_data.push(val.to_string());
            } else if let Ok(val) = row.try_get::<f32, _>(i) {
                row_data.push(val.to_string());
            } else if let Ok(val) = row.try_get::<f64, _>(i) {
                row_data.push(val.to_string());
            } else if let Ok(val) = row.try_get::<bool, _>(i) {
                row_data.push(val.to_string());
            } else {
                row_data.push("NULL".to_string());
            }
        }
        result_rows.push(row_data);
    }

    Ok(json!({
        "columns": columns,
        "rows": result_rows,
        "row_count": result_rows.len(),
        "total_rows": total_rows,
        "execution_time_ms": execution_time_ms,
        "timing_breakdown": {
            "pool_access_ms": pool_time,
            "count_query_ms": count_time,
            "data_query_ms": data_time,
            "total_db_ms": execution_time_ms
        },
        "page": page,
        "page_size": limit
    }))
}

/// Optimized custom query execution using cached connection pools
async fn execute_custom_query(
    datasource_id: &str, 
    config: &Value, 
    query: &str, 
    limit: i32
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    use sqlx::{Row as SqlxRow, Column};
    use serde_json::json;
    
    let pool_manager = get_pool_manager().await;
    let pool = pool_manager.get_pool(datasource_id, config).await?;
    
    // Add LIMIT if not present
    let query_with_limit = if query.to_lowercase().contains("limit") {
        query.to_string()
    } else {
        format!("{} LIMIT {}", query, limit)
    };

    let start = std::time::Instant::now();
    let rows = sqlx::query(&query_with_limit).fetch_all(pool.as_ref()).await?;
    let execution_time_ms = start.elapsed().as_millis() as i64;

    // Get column names
    let columns: Vec<String> = if !rows.is_empty() {
        // Get column names from the first row if we have data
        let first_row = &rows[0];
        first_row.columns().iter().map(|c| c.name().to_string()).collect()
    } else {
        // For empty results, try to get column info using a LIMIT 0 query
        let describe_query = format!("SELECT * FROM ({}) AS temp_query LIMIT 0", query.trim_end_matches(';'));
        match sqlx::query(&describe_query).fetch_optional(pool.as_ref()).await {
            Ok(Some(row)) => {
                // We got a row with column structure
                row.columns().iter().map(|c| c.name().to_string()).collect()
            }
            Ok(None) | Err(_) => {
                // If the describe query fails or returns None, we can't determine columns
                // This happens with non-SELECT queries or invalid queries
                vec![]
            }
        }
    };

    if rows.is_empty() {
        return Ok(json!({
            "columns": columns,
            "rows": [],
            "row_count": 0,
            "execution_time_ms": execution_time_ms
        }));
    }

    // Convert rows to JSON
    let mut result_rows = Vec::new();
    for row in rows.iter() {
        let mut row_data = Vec::new();
        for (i, _col) in columns.iter().enumerate() {
            // Try to get value as different types, including PostgreSQL NUMERIC and UUID
            if let Ok(val) = row.try_get::<String, _>(i) {
                row_data.push(val);
            } else if let Ok(val) = row.try_get::<sqlx::types::Uuid, _>(i) {
                row_data.push(val.to_string());
            } else if let Ok(val) = row.try_get::<rust_decimal::Decimal, _>(i) {
                row_data.push(val.to_string());
            } else if let Ok(val) = row.try_get::<sqlx::types::BigDecimal, _>(i) {
                row_data.push(val.to_string());
            } else if let Ok(val) = row.try_get::<i32, _>(i) {
                row_data.push(val.to_string());
            } else if let Ok(val) = row.try_get::<i64, _>(i) {
                row_data.push(val.to_string());
            } else if let Ok(val) = row.try_get::<f32, _>(i) {
                row_data.push(val.to_string());
            } else if let Ok(val) = row.try_get::<f64, _>(i) {
                row_data.push(val.to_string());
            } else if let Ok(val) = row.try_get::<bool, _>(i) {
                row_data.push(val.to_string());
            } else {
                row_data.push("NULL".to_string());
            }
        }
        result_rows.push(row_data);
    }

    Ok(json!({
        "columns": columns,
        "rows": result_rows,
        "row_count": result_rows.len(),
        "execution_time_ms": execution_time_ms,
        "query": query
    }))
}

/// Optimized list tables using cached connection pools
async fn list_tables(
    datasource_id: &str, 
    config: &Value
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let pool_manager = get_pool_manager().await;
    let pool = pool_manager.get_pool(datasource_id, config).await?;

    // Get schema from config (default to 'public')
    let schema = config
        .get("schema")
        .and_then(|v| v.as_str())
        .unwrap_or("public");

    let tables = sqlx::query(
        "SELECT table_name 
         FROM information_schema.tables 
         WHERE table_schema = $1 
         AND table_type = 'BASE TABLE'
         ORDER BY table_name",
    )
    .bind(schema)
    .fetch_all(pool.as_ref())
    .await?;

    let mut table_names = Vec::new();
    for row in &tables {
        let table_name: String = row
            .try_get("table_name")
            .map_err(|e| format!("Failed to get table_name: {}", e))?;
        table_names.push(table_name);
    }
    Ok(table_names)
}

/// Get PostgreSQL table structure information
async fn get_postgres_table_structure(
    datasource_id: &str,
    config: &Value,
    table_name: &str,
) -> Result<TableStructure, Box<dyn std::error::Error + Send + Sync>> {
    let pool_manager = get_pool_manager().await;
    let pool = pool_manager.get_pool(datasource_id, config).await?;

    // Get schema from config (default to 'public')
    let schema = config
        .get("schema")
        .and_then(|v| v.as_str())
        .unwrap_or("public");

    // Get column information
    let column_rows = sqlx::query(
        r#"
        SELECT 
            c.column_name,
            c.data_type,
            c.is_nullable,
            c.column_default,
            c.character_maximum_length,
            c.numeric_precision,
            c.numeric_scale,
            CASE 
                WHEN pk.column_name IS NOT NULL THEN true 
                ELSE false 
            END as is_primary_key,
            CASE 
                WHEN fk.column_name IS NOT NULL THEN true 
                ELSE false 
            END as is_foreign_key
        FROM information_schema.columns c
        LEFT JOIN (
            SELECT ku.column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage ku ON tc.constraint_name = ku.constraint_name
            WHERE tc.constraint_type = 'PRIMARY KEY' 
            AND tc.table_name = $1 
            AND tc.table_schema = $2
        ) pk ON c.column_name = pk.column_name
        LEFT JOIN (
            SELECT ku.column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage ku ON tc.constraint_name = ku.constraint_name
            WHERE tc.constraint_type = 'FOREIGN KEY' 
            AND tc.table_name = $1 
            AND tc.table_schema = $2
        ) fk ON c.column_name = fk.column_name
        WHERE c.table_name = $1 AND c.table_schema = $2
        ORDER BY c.ordinal_position
        "#,
    )
    .bind(table_name)
    .bind(schema)
    .fetch_all(pool.as_ref())
    .await?;

    let mut columns = Vec::new();
    for row in column_rows {
        columns.push(TableColumn {
            name: row.try_get("column_name")?,
            data_type: row.try_get("data_type")?,
            is_nullable: row.try_get::<String, _>("is_nullable")? == "YES",
            column_default: row.try_get("column_default")?,
            is_primary_key: row.try_get("is_primary_key")?,
            is_foreign_key: row.try_get("is_foreign_key")?,
            character_maximum_length: row.try_get("character_maximum_length")?,
            numeric_precision: row.try_get("numeric_precision")?,
            numeric_scale: row.try_get("numeric_scale")?,
        });
    }

    // Get primary keys
    let pk_rows = sqlx::query(
        r#"
        SELECT ku.column_name
        FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage ku ON tc.constraint_name = ku.constraint_name
        WHERE tc.constraint_type = 'PRIMARY KEY' 
        AND tc.table_name = $1 
        AND tc.table_schema = $2
        ORDER BY ku.ordinal_position
        "#,
    )
    .bind(table_name)
    .bind(schema)
    .fetch_all(pool.as_ref())
    .await?;

    let mut primary_keys = Vec::new();
    for row in pk_rows {
        primary_keys.push(row.try_get("column_name")?);
    }

    // Get foreign keys
    let fk_rows = sqlx::query(
        r#"
        SELECT 
            ku.column_name,
            ccu.table_name as referenced_table,
            ccu.column_name as referenced_column
        FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage ku ON tc.constraint_name = ku.constraint_name
        JOIN information_schema.constraint_column_usage ccu ON tc.constraint_name = ccu.constraint_name
        WHERE tc.constraint_type = 'FOREIGN KEY' 
        AND tc.table_name = $1 
        AND tc.table_schema = $2
        "#,
    )
    .bind(table_name)
    .bind(schema)
    .fetch_all(pool.as_ref())
    .await?;

    let mut foreign_keys = Vec::new();
    for row in fk_rows {
        foreign_keys.push(ForeignKeyInfo {
            column_name: row.try_get("column_name")?,
            referenced_table: row.try_get("referenced_table")?,
            referenced_column: row.try_get("referenced_column")?,
        });
    }

    // Get indexes
    let index_rows = sqlx::query(
        r#"
        SELECT 
            i.indexname as index_name,
            i.indexdef,
            array_agg(a.attname ORDER BY a.attnum) as columns
        FROM pg_indexes i
        JOIN pg_class t ON t.relname = i.tablename
        JOIN pg_index ix ON ix.indrelid = t.oid
        JOIN pg_class idx ON idx.oid = ix.indexrelid
        JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey)
        WHERE i.tablename = $1 
        AND i.schemaname = $2
        GROUP BY i.indexname, i.indexdef, ix.indisunique
        "#,
    )
    .bind(table_name)
    .bind(schema)
    .fetch_all(pool.as_ref())
    .await?;

    let mut indexes = Vec::new();
    for row in index_rows {
        let index_def: String = row.try_get("indexdef")?;
        let is_unique = index_def.to_lowercase().contains("unique");
        let columns_array: Vec<String> = row.try_get("columns")?;
        
        indexes.push(IndexInfo {
            name: row.try_get("index_name")?,
            columns: columns_array,
            is_unique,
        });
    }

    Ok(TableStructure {
        table_name: table_name.to_string(),
        columns,
        primary_keys,
        foreign_keys,
        indexes,
    })
}

/// Cached datasource validation to avoid repeated database queries
async fn get_cached_datasource(
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
    
    let row = datasource_row
        .ok_or_else(|| AppError::NotFound("Datasource not found".to_string()))?;
    
    // Create cached datasource
    let cached_datasource = CachedDatasource {
        id: row.try_get("id").map_err(|e| AppError::InternalServerError(format!("Failed to get id: {}", e)))?,
        datasource_type: row.try_get("source_type").map_err(|e| AppError::InternalServerError(format!("Failed to get source_type: {}", e)))?,
        connection_config: row.try_get("connection_config").map_err(|e| AppError::InternalServerError(format!("Failed to get connection_config: {}", e)))?,
        user_id: row.try_get::<Uuid, _>("user_id").map_err(|e| AppError::InternalServerError(format!("Failed to get user_id: {}", e)))?,
        cached_at: std::time::Instant::now(),
    };
    
    // Store in cache
    cache.set(cached_datasource.clone()).await;
    
    Ok(cached_datasource)
}

/// Update schema_info with table structure information
async fn update_schema_info_with_table_structure(
    db_pool: &sqlx::PgPool,
    datasource_id: &str,
    table_name: &str,
    table_structure: &TableStructure,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get current schema_info
    let current_row = sqlx::query("SELECT schema_info FROM data_sources WHERE id = $1")
        .bind(datasource_id)
        .fetch_optional(db_pool)
        .await?;

    let mut schema_info = if let Some(row) = current_row {
        let current_schema: Option<Value> = row.get("schema_info");
        current_schema.unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Ensure tables object exists
    if schema_info.get("tables").is_none() {
        schema_info["tables"] = serde_json::json!({});
    }

    // Add/update the table structure
    let table_structure_json = serde_json::to_value(table_structure)?;
    schema_info["tables"][table_name] = table_structure_json;

    // Update the database
    sqlx::query("UPDATE data_sources SET schema_info = $1, updated_at = NOW() WHERE id = $2")
        .bind(&schema_info)
        .bind(datasource_id)
        .execute(db_pool)
        .await?;

    Ok(())
}

/// Get distinct values for a column
#[handler]
pub async fn get_distinct_values(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let request_start = std::time::Instant::now();
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;
    let table_name = req.param::<String>("table_name")
        .ok_or_else(|| AppError::BadRequest("Missing table_name".to_string()))?;

    let request_data: DistinctValuesRequest = req.parse_json().await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    // Get datasource and verify ownership using cache
    let cached_datasource = get_cached_datasource(&datasource_id, &user_id, is_current_user_root(depot), &state.db_pool).await?;
    let source_type = cached_datasource.datasource_type.clone();
    let config = cached_datasource.connection_config.clone();

    // Execute query based on source type using cached connection pools
    let result = match source_type.as_str() {
        "postgresql" => {
            execute_distinct_values_query(&datasource_id, &config, &table_name, 
                                        &request_data.column, 
                                        request_data.limit,
                                        request_data.search.as_deref()).await
                .map_err(|e| AppError::InternalServerError(format!("Query execution failed: {}", e)))?
        },
        _ => {
            return Err(AppError::BadRequest(format!("Unsupported datasource type: {}", source_type)));
        }
    };

    let total_time = request_start.elapsed().as_millis();
    tracing::info!("Distinct values request took {}ms", total_time);

    res.render(Json(result));
    Ok(())
}

async fn execute_distinct_values_query(
    datasource_id: &str,
    config: &Value,
    table_name: &str,
    column_name: &str,
    limit: Option<i32>,
    search: Option<&str>
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    use sqlx::{Row as SqlxRow};
    use serde_json::json;
    use std::time::Instant;

    let pool_start = Instant::now();
    let pool_manager = get_pool_manager().await;
    let pool = pool_manager.get_pool(datasource_id, config).await?;
    let pool_time = pool_start.elapsed().as_millis() as u64;

    let start = Instant::now();
    let limit_value = limit.unwrap_or(100); // Default to 100 distinct values

    // Build the query
    let mut query = format!(
        "SELECT DISTINCT {} FROM {} WHERE {} IS NOT NULL",
        column_name, table_name, column_name
    );
    let mut query_params: Vec<String> = Vec::new();

    // Add search filter if provided
    if let Some(search_term) = search {
        if !search_term.is_empty() {
            query.push_str(&format!(" AND {} ILIKE ${}", column_name, query_params.len() + 1));
            query_params.push(format!("%{}%", search_term));
        }
    }

    // Add ordering and limit
    query.push_str(&format!(" ORDER BY {} LIMIT {}", column_name, limit_value));

    // Execute the query
    let mut query_builder = sqlx::query(&query);
    for param in &query_params {
        query_builder = query_builder.bind(param);
    }

    let rows = query_builder
        .fetch_all(pool.as_ref())
        .await?;

    let execution_time_ms = start.elapsed().as_millis() as u64;

    // Extract values from rows
    let mut values = Vec::new();
    for row in rows {
        if let Ok(value) = row.try_get::<String, _>(0) {
            values.push(value);
        } else if let Ok(value) = row.try_get::<i32, _>(0) {
            values.push(value.to_string());
        } else if let Ok(value) = row.try_get::<i64, _>(0) {
            values.push(value.to_string());
        } else if let Ok(value) = row.try_get::<f64, _>(0) {
            values.push(value.to_string());
        } else if let Ok(value) = row.try_get::<bool, _>(0) {
            values.push(value.to_string());
        } else {
            // Skip null values or types we can't handle
        }
    }

    Ok(json!({
        "values": values,
        "count": values.len(),
        "execution_time_ms": execution_time_ms,
        "timing_breakdown": {
            "pool_access_ms": pool_time,
            "query_ms": execution_time_ms
        }
    }))
}

/// Delete rows from a table
#[handler]
pub async fn delete_rows(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;
    let table_name = req.param::<String>("table_name")
        .ok_or_else(|| AppError::BadRequest("Missing table_name".to_string()))?;

    let request_data: DeleteRowsRequest = req.parse_json().await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    if request_data.row_ids.is_empty() {
        return Err(AppError::BadRequest("No row IDs provided".to_string()));
    }

    // Get datasource and verify ownership using cache
    let cached_datasource = get_cached_datasource(&datasource_id, &user_id, is_current_user_root(depot), &state.db_pool).await?;
    let source_type = cached_datasource.datasource_type.clone();
    let config = cached_datasource.connection_config.clone();

    // Execute delete based on source type
    let result = match source_type.as_str() {
        "postgresql" => {
            execute_delete_rows_query(&datasource_id, &config, &table_name, 
                                    &request_data.row_ids,
                                    request_data.id_column.as_deref()).await
                .map_err(|e| AppError::InternalServerError(format!("Delete execution failed: {}", e)))?
        },
        _ => {
            return Err(AppError::BadRequest(format!("Unsupported datasource type: {}", source_type)));
        }
    };

    res.render(Json(result));
    Ok(())
}

/// Update rows in a table
#[handler]
pub async fn update_rows(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;
    let table_name = req.param::<String>("table_name")
        .ok_or_else(|| AppError::BadRequest("Missing table_name".to_string()))?;

    let request_data: UpdateRowsRequest = req.parse_json().await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    if request_data.updates.is_empty() {
        return Err(AppError::BadRequest("No updates provided".to_string()));
    }

    // Get datasource and verify ownership using cache
    let cached_datasource = get_cached_datasource(&datasource_id, &user_id, is_current_user_root(depot), &state.db_pool).await?;
    let source_type = cached_datasource.datasource_type.clone();
    let config = cached_datasource.connection_config.clone();

    // Execute update based on source type
    let result = match source_type.as_str() {
        "postgresql" => {
            execute_update_rows_query(&datasource_id, &config, &table_name, 
                                    &request_data.updates,
                                    request_data.id_column.as_deref()).await
                .map_err(|e| AppError::InternalServerError(format!("Update execution failed: {}", e)))?
        },
        _ => {
            return Err(AppError::BadRequest(format!("Unsupported datasource type: {}", source_type)));
        }
    };

    res.render(Json(result));
    Ok(())
}

/// Insert rows into a table
#[handler]
pub async fn insert_rows(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;
    let table_name = req.param::<String>("table_name")
        .ok_or_else(|| AppError::BadRequest("Missing table_name".to_string()))?;

    let request_data: InsertRowsRequest = req.parse_json().await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    if request_data.rows.is_empty() {
        return Err(AppError::BadRequest("No rows to insert provided".to_string()));
    }

    // Get datasource and verify ownership using cache
    let cached_datasource = get_cached_datasource(&datasource_id, &user_id, is_current_user_root(depot), &state.db_pool).await?;
    let source_type = cached_datasource.datasource_type.clone();
    let config = cached_datasource.connection_config.clone();

    // Execute insert based on source type
    let result = match source_type.as_str() {
        "postgresql" => {
            execute_insert_rows_query(&datasource_id, &config, &table_name, 
                                    &request_data.rows).await
                .map_err(|e| AppError::InternalServerError(format!("Insert execution failed: {}", e)))?
        },
        _ => {
            return Err(AppError::BadRequest(format!("Unsupported datasource type: {}", source_type)));
        }
    };

    res.render(Json(result));
    Ok(())
}

async fn execute_update_rows_query(
    datasource_id: &str,
    config: &Value,
    table_name: &str,
    updates: &std::collections::HashMap<String, std::collections::HashMap<String, Value>>,
    id_column: Option<&str>
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    use serde_json::json;
    use std::time::Instant;

    let pool_manager = get_pool_manager().await;
    let pool = pool_manager.get_pool(datasource_id, config).await?;

    let start = Instant::now();
    let id_col = id_column.unwrap_or("id");
    let mut total_affected = 0u64;
    let mut updated_ids = Vec::new();

    // Process each row update
    for (row_id, column_updates) in updates {
        if column_updates.is_empty() {
            continue;
        }

        let mut set_clauses = Vec::new();
        let mut query_params = Vec::new();
        let mut param_counter = 1;

        // Build SET clauses
        for (column_name, new_value) in column_updates {
            set_clauses.push(format!("{} = ${}", column_name, param_counter));
            query_params.push(new_value.clone());
            param_counter += 1;
        }

        // Add WHERE clause parameter
        let query = format!(
            "UPDATE {} SET {} WHERE {} = ${}",
            table_name,
            set_clauses.join(", "),
            id_col,
            param_counter
        );
        query_params.push(Value::String(row_id.clone()));

        // Execute the update query
        let mut query_builder = sqlx::query(&query);
        for param in &query_params {
            match param {
                Value::String(s) => query_builder = query_builder.bind(s),
                Value::Number(n) if n.is_i64() => query_builder = query_builder.bind(n.as_i64().unwrap()),
                Value::Number(n) if n.is_f64() => query_builder = query_builder.bind(n.as_f64().unwrap()),
                Value::Bool(b) => query_builder = query_builder.bind(b),
                Value::Null => query_builder = query_builder.bind(Option::<String>::None),
                _ => query_builder = query_builder.bind(param.to_string()),
            }
        }

        let result = query_builder
            .execute(pool.as_ref())
            .await?;

        let rows_affected = result.rows_affected();
        if rows_affected > 0 {
            total_affected += rows_affected;
            updated_ids.push(row_id.clone());
        }
    }

    let execution_time_ms = start.elapsed().as_millis() as u64;

    Ok(json!({
        "success": true,
        "rows_affected": total_affected,
        "execution_time_ms": execution_time_ms,
        "updated_ids": updated_ids
    }))
}

async fn execute_insert_rows_query(
    datasource_id: &str,
    config: &Value,
    table_name: &str,
    rows: &[std::collections::HashMap<String, Value>]
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    use serde_json::json;
    use std::time::Instant;

    let pool_manager = get_pool_manager().await;
    let pool = pool_manager.get_pool(datasource_id, config).await?;

    let start = Instant::now();
    
    if rows.is_empty() {
        return Ok(json!({
            "success": true,
            "rows_affected": 0,
            "execution_time_ms": 0,
            "inserted_ids": []
        }));
    }

    // Get column names from first row
    let columns: Vec<&String> = rows[0].keys().collect();
    let column_names = columns.iter().map(|c| c.as_str()).collect::<Vec<_>>();
    
    // Build INSERT query
    
    let mut all_placeholders = Vec::new();
    let mut param_counter = 1;
    
    for _ in 0..rows.len() {
        let row_placeholders: Vec<String> = (0..columns.len())
            .map(|_| {
                let placeholder = format!("${}", param_counter);
                param_counter += 1;
                placeholder
            })
            .collect();
        all_placeholders.push(format!("({})", row_placeholders.join(", ")));
    }

    let query = format!(
        "INSERT INTO {} ({}) VALUES {} RETURNING *",
        table_name,
        column_names.join(", "),
        all_placeholders.join(", ")
    );

    // Bind parameters
    let mut query_builder = sqlx::query(&query);
    for row in rows {
        for column_name in &column_names {
            let value = row.get(*column_name).unwrap_or(&Value::Null);
            match value {
                Value::String(s) => query_builder = query_builder.bind(s),
                Value::Number(n) if n.is_i64() => query_builder = query_builder.bind(n.as_i64().unwrap()),
                Value::Number(n) if n.is_f64() => query_builder = query_builder.bind(n.as_f64().unwrap()),
                Value::Bool(b) => query_builder = query_builder.bind(b),
                Value::Null => query_builder = query_builder.bind(Option::<String>::None),
                _ => query_builder = query_builder.bind(value.to_string()),
            }
        }
    }

    let result_rows = query_builder
        .fetch_all(pool.as_ref())
        .await?;

    let execution_time_ms = start.elapsed().as_millis() as u64;
    let rows_affected = result_rows.len() as u64;
    
    // Extract inserted IDs (assume first column is the ID)
    let mut inserted_ids = Vec::new();
    for row in &result_rows {
        if let Ok(id) = row.try_get::<String, _>(0) {
            inserted_ids.push(id);
        } else if let Ok(id) = row.try_get::<i32, _>(0) {
            inserted_ids.push(id.to_string());
        } else if let Ok(id) = row.try_get::<i64, _>(0) {
            inserted_ids.push(id.to_string());
        }
    }

    Ok(json!({
        "success": true,
        "rows_affected": rows_affected,
        "execution_time_ms": execution_time_ms,
        "inserted_ids": inserted_ids
    }))
}

async fn execute_delete_rows_query(
    datasource_id: &str,
    config: &Value,
    table_name: &str,
    row_ids: &[String],
    id_column: Option<&str>
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    use serde_json::json;
    use std::time::Instant;

    let pool_manager = get_pool_manager().await;
    let pool = pool_manager.get_pool(datasource_id, config).await?;

    let start = Instant::now();
    let id_col = id_column.unwrap_or("id");

    // Build the DELETE query with parameterized values for security
    let placeholders: Vec<String> = (1..=row_ids.len())
        .map(|i| format!("${}", i))
        .collect();
    
    let query = format!(
        "DELETE FROM {} WHERE {} IN ({})",
        table_name,
        id_col,
        placeholders.join(", ")
    );

    // Execute the delete query
    let mut query_builder = sqlx::query(&query);
    for row_id in row_ids {
        query_builder = query_builder.bind(row_id);
    }

    let result = query_builder
        .execute(pool.as_ref())
        .await?;

    let execution_time_ms = start.elapsed().as_millis() as u64;
    let rows_affected = result.rows_affected();

    Ok(json!({
        "success": true,
        "rows_affected": rows_affected,
        "execution_time_ms": execution_time_ms,
        "deleted_ids": row_ids
    }))
}

pub fn datasource_routes() -> Router {
    Router::new()
        // Project-scoped routes
        .push(Router::with_path("/projects/{project_id}/datasources").get(list_datasources).post(create_datasource))
        // Datasource-specific routes
        .push(Router::with_path("/datasources/{datasource_id}").put(update_datasource).delete(delete_datasource))
        .push(Router::with_path("/datasources/{datasource_id}/test").post(test_connection))
        .push(Router::with_path("/datasources/{datasource_id}/schema").get(get_schema))
        // Data browser routes
        .push(Router::with_path("/datasources/{datasource_id}/query").post(execute_query))
        .push(Router::with_path("/datasources/{datasource_id}/tables").get(get_tables))
        .push(Router::with_path("/datasources/{datasource_id}/tables/{table_name}/data").post(get_table_data))
        .push(Router::with_path("/datasources/{datasource_id}/tables/{table_name}/structure").get(get_table_structure))
        .push(Router::with_path("/datasources/{datasource_id}/tables/{table_name}/distinct").post(get_distinct_values))
        .push(Router::with_path("/datasources/{datasource_id}/tables/{table_name}/rows").delete(delete_rows).put(update_rows).post(insert_rows))
        // Test arbitrary config
        .push(Router::with_path("/test-connection").post(test_connection_with_config))
}