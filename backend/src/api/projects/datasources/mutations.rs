use salvo::prelude::*;
use serde_json::Value;

use crate::utils::middleware::{get_current_user_id, is_current_user_root};
use crate::utils::{get_app_state, AppError};

use super::crud::get_cached_datasource;
use super::types::{DeleteRowsRequest, UpdateRowsRequest, InsertRowsRequest};


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
    let mut config = cached_datasource.connection_config.clone();
    
    // Add datasource ID to config for the connector
    config.as_object_mut()
        .ok_or_else(|| AppError::InternalServerError("Invalid config format".to_string()))?
        .insert("id".to_string(), Value::String(datasource_id.clone()));

    // Execute delete based on source type
    let result = match source_type.as_str() {
        "postgresql" | "mysql" | "sqlite" => {
            execute_delete_rows_query(&datasource_id, &config, &table_name, 
                                    &request_data.row_ids,
                                    request_data.id_column.as_deref(), &source_type).await
                .map_err(|e| AppError::InternalServerError(format!("Delete execution failed: {}", e)))?
        },
        "clickhouse" => {
            execute_clickhouse_delete_rows_query(&datasource_id, &config, &table_name, 
                                                &request_data.row_ids,
                                                request_data.id_column.as_deref()).await
                .map_err(|e| AppError::InternalServerError(format!("Delete execution failed: {}", e)))?
        },
        "oracle" => {
            execute_oracle_delete_rows_query(&datasource_id, &config, &table_name, 
                                            &request_data.row_ids,
                                            request_data.id_column.as_deref()).await
                .map_err(|e| AppError::InternalServerError(format!("Delete execution failed: {}", e)))?
        },
        "sqlserver" => {
            execute_sqlserver_delete_rows_query(&datasource_id, &config, &table_name, 
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
    let mut config = cached_datasource.connection_config.clone();
    
    // Add datasource ID to config for the connector
    config.as_object_mut()
        .ok_or_else(|| AppError::InternalServerError("Invalid config format".to_string()))?
        .insert("id".to_string(), Value::String(datasource_id.clone()));

    // Execute update based on source type
    let result = match source_type.as_str() {
        "postgresql" | "mysql" | "sqlite" => {
            execute_update_rows_query(&datasource_id, &config, &table_name, 
                                    &request_data.updates,
                                    request_data.id_column.as_deref(), &source_type).await
                .map_err(|e| AppError::InternalServerError(format!("Update execution failed: {}", e)))?
        },
        "clickhouse" => {
            execute_clickhouse_update_rows_query(&datasource_id, &config, &table_name, 
                                                &request_data.updates,
                                                request_data.id_column.as_deref()).await
                .map_err(|e| AppError::InternalServerError(format!("Update execution failed: {}", e)))?
        },
        "oracle" => {
            execute_oracle_update_rows_query(&datasource_id, &config, &table_name, 
                                            &request_data.updates,
                                            request_data.id_column.as_deref()).await
                .map_err(|e| AppError::InternalServerError(format!("Update execution failed: {}", e)))?
        },
        "sqlserver" => {
            execute_sqlserver_update_rows_query(&datasource_id, &config, &table_name, 
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
    let mut config = cached_datasource.connection_config.clone();
    
    // Add datasource ID to config for the connector
    config.as_object_mut()
        .ok_or_else(|| AppError::InternalServerError("Invalid config format".to_string()))?
        .insert("id".to_string(), Value::String(datasource_id.clone()));

    // Execute insert based on source type
    let result = match source_type.as_str() {
        "postgresql" | "mysql" | "sqlite" => {
            execute_insert_rows_query(&datasource_id, &config, &table_name, 
                                    &request_data.rows, &source_type).await
                .map_err(|e| AppError::InternalServerError(format!("Insert execution failed: {}", e)))?
        },
        "clickhouse" => {
            execute_clickhouse_insert_rows_query(&datasource_id, &config, &table_name, 
                                                &request_data.rows).await
                .map_err(|e| AppError::InternalServerError(format!("Insert execution failed: {}", e)))?
        },
        "oracle" => {
            execute_oracle_insert_rows_query(&datasource_id, &config, &table_name, 
                                            &request_data.rows).await
                .map_err(|e| AppError::InternalServerError(format!("Insert execution failed: {}", e)))?
        },
        "sqlserver" => {
            execute_sqlserver_insert_rows_query(&datasource_id, &config, &table_name, 
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

// Helper functions - these are implementation stubs, the actual implementations
// need to be moved from the original datasources.rs file


async fn execute_delete_rows_query(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
    _row_ids: &[String],
    _id_column: Option<&str>,
    _source_type: &str
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(serde_json::json!({"success": true, "rows_affected": 0}))
}

async fn execute_clickhouse_delete_rows_query(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
    _row_ids: &[String],
    _id_column: Option<&str>
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(serde_json::json!({"success": true, "rows_affected": 0}))
}

async fn execute_oracle_delete_rows_query(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
    _row_ids: &[String],
    _id_column: Option<&str>
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(serde_json::json!({"success": true, "rows_affected": 0}))
}

async fn execute_sqlserver_delete_rows_query(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
    _row_ids: &[String],
    _id_column: Option<&str>
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(serde_json::json!({"success": true, "rows_affected": 0}))
}

async fn execute_update_rows_query(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
    _updates: &std::collections::HashMap<String, std::collections::HashMap<String, Value>>,
    _id_column: Option<&str>,
    _source_type: &str
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(serde_json::json!({"success": true, "rows_affected": 0}))
}

async fn execute_clickhouse_update_rows_query(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
    _updates: &std::collections::HashMap<String, std::collections::HashMap<String, Value>>,
    _id_column: Option<&str>
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(serde_json::json!({"success": true, "rows_affected": 0}))
}

async fn execute_oracle_update_rows_query(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
    _updates: &std::collections::HashMap<String, std::collections::HashMap<String, Value>>,
    _id_column: Option<&str>
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(serde_json::json!({"success": true, "rows_affected": 0}))
}

async fn execute_sqlserver_update_rows_query(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
    _updates: &std::collections::HashMap<String, std::collections::HashMap<String, Value>>,
    _id_column: Option<&str>
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(serde_json::json!({"success": true, "rows_affected": 0}))
}

async fn execute_insert_rows_query(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
    _rows: &[std::collections::HashMap<String, Value>],
    _source_type: &str
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(serde_json::json!({"success": true, "rows_affected": 0}))
}

async fn execute_clickhouse_insert_rows_query(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
    _rows: &[std::collections::HashMap<String, Value>]
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(serde_json::json!({"success": true, "rows_affected": 0}))
}

async fn execute_oracle_insert_rows_query(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
    _rows: &[std::collections::HashMap<String, Value>]
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(serde_json::json!({"success": true, "rows_affected": 0}))
}

async fn execute_sqlserver_insert_rows_query(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
    _rows: &[std::collections::HashMap<String, Value>]
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(serde_json::json!({"success": true, "rows_affected": 0}))
}