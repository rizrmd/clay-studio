use chrono::Utc;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::utils::middleware::get_current_client_id;
use crate::utils::{get_app_state, AppError};

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

/// List all datasources for a project
#[handler]
pub async fn list_datasources(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let client_id = get_current_client_id(depot)?;
    let project_id = req.param::<String>("project_id")
        .ok_or_else(|| AppError::BadRequest("Missing project_id".to_string()))?;

    // Validate project ownership
    let project_exists = sqlx::query(
        "SELECT 1 FROM projects WHERE id = $1 AND client_id = $2 AND deleted_at IS NULL"
    )
    .bind(&project_id)
    .bind(&client_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

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
    let client_id = get_current_client_id(depot)?;
    let project_id = req.param::<String>("project_id")
        .ok_or_else(|| AppError::BadRequest("Missing project_id".to_string()))?;

    let request_data: CreateDatasourceRequest = req.parse_json().await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    // Validate project ownership
    let project_exists = sqlx::query(
        "SELECT 1 FROM projects WHERE id = $1 AND client_id = $2 AND deleted_at IS NULL"
    )
    .bind(&project_id)
    .bind(&client_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

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
    let client_id = get_current_client_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;

    let request_data: UpdateDatasourceRequest = req.parse_json().await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    // Check if datasource exists and belongs to client's project
    let existing = sqlx::query(
        r#"
        SELECT ds.*, p.client_id 
        FROM data_sources ds
        JOIN projects p ON ds.project_id = p.id
        WHERE ds.id = $1 AND ds.deleted_at IS NULL AND p.deleted_at IS NULL
        "#
    )
    .bind(&datasource_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let existing_row = existing.ok_or_else(|| AppError::NotFound("Datasource not found".to_string()))?;
    let row_client_id: Uuid = existing_row.get("client_id");
    
    if row_client_id != client_id {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }

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
    let client_id = get_current_client_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;

    // Check if datasource exists and belongs to client's project
    let existing = sqlx::query(
        r#"
        SELECT ds.id, p.client_id 
        FROM data_sources ds
        JOIN projects p ON ds.project_id = p.id
        WHERE ds.id = $1 AND ds.deleted_at IS NULL AND p.deleted_at IS NULL
        "#
    )
    .bind(&datasource_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let existing_row = existing.ok_or_else(|| AppError::NotFound("Datasource not found".to_string()))?;
    let row_client_id: Uuid = existing_row.get("client_id");
    
    if row_client_id != client_id {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }

    // Soft delete the datasource
    sqlx::query(
        "UPDATE data_sources SET deleted_at = $1 WHERE id = $2"
    )
    .bind(Utc::now())
    .bind(&datasource_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to delete datasource: {}", e)))?;

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
    let client_id = get_current_client_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;

    // Get datasource and verify ownership
    let datasource = sqlx::query(
        r#"
        SELECT ds.*, ds.connection_config as config, p.client_id 
        FROM data_sources ds
        JOIN projects p ON ds.project_id = p.id
        WHERE ds.id = $1 AND ds.deleted_at IS NULL AND p.deleted_at IS NULL
        "#
    )
    .bind(&datasource_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let datasource_row = datasource.ok_or_else(|| AppError::NotFound("Datasource not found".to_string()))?;
    let row_client_id: Uuid = datasource_row.get("client_id");
    
    if row_client_id != client_id {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }

    let source_type: String = datasource_row.get("source_type");
    let config: Value = datasource_row.get("config");

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
    let client_id = get_current_client_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;

    // Get datasource and verify ownership
    let datasource = sqlx::query(
        r#"
        SELECT ds.*, ds.connection_config as config, p.client_id 
        FROM data_sources ds
        JOIN projects p ON ds.project_id = p.id
        WHERE ds.id = $1 AND ds.deleted_at IS NULL AND p.deleted_at IS NULL
        "#
    )
    .bind(&datasource_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let datasource_row = datasource.ok_or_else(|| AppError::NotFound("Datasource not found".to_string()))?;
    let row_client_id: Uuid = datasource_row.get("client_id");
    
    if row_client_id != client_id {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }

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

pub fn datasource_routes() -> Router {
    Router::new()
        // Project-scoped routes
        .push(Router::with_path("/projects/{project_id}/datasources").get(list_datasources).post(create_datasource))
        // Datasource-specific routes
        .push(Router::with_path("/datasources/{datasource_id}").put(update_datasource).delete(delete_datasource))
        .push(Router::with_path("/datasources/{datasource_id}/test").post(test_connection))
        .push(Router::with_path("/datasources/{datasource_id}/schema").get(get_schema))
        // Test arbitrary config
        .push(Router::with_path("/test-connection").post(test_connection_with_config))
}