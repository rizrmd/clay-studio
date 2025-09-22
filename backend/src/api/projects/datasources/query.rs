use salvo::prelude::*;
use serde_json::Value;

use crate::utils::middleware::{get_current_user_id, is_current_user_root};
use crate::utils::{get_app_state, AppError};
use crate::utils::datasource::{create_connector, get_pool_manager};

use super::crud::get_cached_datasource;
use super::types::{QueryRequest, TableDataRequest, DistinctValuesRequest, RowIdsRequest};

/// Execute a custom query on a datasource
#[handler]
#[allow(dead_code)]
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
    let mut config = cached_datasource.connection_config.clone();
    
    // Add datasource ID to config for the connector
    config.as_object_mut()
        .ok_or_else(|| AppError::InternalServerError("Invalid config format".to_string()))?
        .insert("id".to_string(), Value::String(datasource_id.clone()));

    // Create connector using factory
    let connector = create_connector(&source_type, &config)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to create connector: {}", e)))?;

    // Execute query using connector
    let result = connector.execute_query(&request_data.query, 1000000).await
        .map_err(|e| AppError::InternalServerError(format!("Query execution failed: {}", e)))?;

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
    let mut config = cached_datasource.connection_config.clone();
    
    // Add datasource ID to config for the connector
    config.as_object_mut()
        .ok_or_else(|| AppError::InternalServerError("Invalid config format".to_string()))?
        .insert("id".to_string(), Value::String(datasource_id.clone()));

    // Create connector using factory
    let connector = create_connector(&source_type, &config)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to create connector: {}", e)))?;

    // Get pagination parameters
    let page = request_data.page.unwrap_or(1);
    let limit = request_data.limit.unwrap_or(50);

    // Execute table data query using connector
    let result = connector.get_table_data_with_pagination(
        &table_name, 
        page, 
        limit, 
        request_data.sort_column.as_deref(), 
        request_data.sort_direction.as_deref()
    ).await
        .map_err(|e| AppError::InternalServerError(format!("Query execution failed: {}", e)))?;

    // Convert result format to match expected response structure
    let formatted_result = if let Some(columns) = result.get("columns") {
        if let Some(rows) = result.get("rows") {
            if let Some(total_rows) = result.get("total_rows") {
                serde_json::json!({
                    "columns": columns,
                    "data": rows,
                    "total": total_rows,
                    "execution_time_ms": result.get("execution_time_ms"),
                    "timing_breakdown": result.get("timing_breakdown")
                })
            } else {
                serde_json::json!({
                    "columns": columns,
                    "data": rows,
                    "total": 0,
                    "execution_time_ms": result.get("execution_time_ms")
                })
            }
        } else {
            serde_json::json!({
                "columns": [],
                "data": [],
                "total": 0
            })
        }
    } else {
        serde_json::json!({
            "columns": [],
            "data": [],
            "total": 0
        })
    };

    let total_time = request_start.elapsed().as_millis();
    tracing::info!("Total request took {}ms", total_time);

    // Add timing breakdown to response
    if let Value::Object(mut result_obj) = formatted_result {
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
        res.render(Json(formatted_result));
    }
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

    // Execute distinct values query using pool manager
    let result = execute_distinct_values_query(&datasource_id, &config, &table_name, 
                                        &request_data.column, 
                                        request_data.limit,
                                        request_data.search.as_deref(), &source_type).await
        .map_err(|e| AppError::InternalServerError(format!("Query execution failed: {}", e)))?;

    let total_time = request_start.elapsed().as_millis();
    tracing::info!("Distinct values request took {}ms", total_time);

    res.render(Json(result));
    Ok(())
}

// Execute distinct values query using connection pool
#[allow(dead_code)]
async fn execute_distinct_values_query(
    _datasource_id: &str,
    config: &Value,
    table_name: &str,
    column_name: &str,
    limit: Option<i32>,
    search: Option<&str>,
    source_type: &str
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    use serde_json::json;
    use std::time::Instant;
    
    let pool_start = Instant::now();
    let _pool_manager = get_pool_manager().await;
    
    // For now, fall back to using the connector approach
    // This maintains backward compatibility while we transition
    let mut config_with_id = config.clone();
    if let Some(obj) = config_with_id.as_object_mut() {
        obj.insert("id".to_string(), Value::String(_datasource_id.to_string()));
    }
    let connector = create_connector(source_type, &config_with_id).await
        .map_err(|e| format!("Failed to create connector: {}", e))?;
    
    let pool_time = pool_start.elapsed().as_millis() as u64;
    let start = Instant::now();
    
    // Execute query using connector
    let result = connector.execute_query(
        &build_distinct_values_query(source_type, table_name, column_name, limit, search),
        1000000
    ).await
        .map_err(|e| format!("Query execution failed: {}", e))?;
    
    let execution_time_ms = start.elapsed().as_millis() as u64;
    
    // Extract values from result
    let mut values = Vec::new();
    if let Some(data) = result.get("data").and_then(|d| d.as_array()) {
        for row in data {
            if let Some(obj) = row.as_object() {
                if let Some(val) = obj.values().next().and_then(|v| v.as_str()) {
                    values.push(val.to_string());
                }
            }
        }
    }
    
    Ok(json!({
        "values": values,
        "count": values.len(),
        "execution_time_ms": execution_time_ms,
        "timing_breakdown": {
            "pool_access_ms": pool_time,
            "query_execution_ms": execution_time_ms
        }
    }))
}

// Helper function to build distinct values query
fn build_distinct_values_query(
    source_type: &str,
    table_name: &str,
    column_name: &str,
    limit: Option<i32>,
    search: Option<&str>,
) -> String {
    let limit_val = limit.unwrap_or(100);
    let mut query = format!("SELECT DISTINCT {} FROM {}", column_name, table_name);
    
    if let Some(search_term) = search {
        if !search_term.is_empty() {
            let like_operator = match source_type.to_lowercase().as_str() {
                "postgresql" | "postgres" => "ILIKE",
                "mysql" | "sqlite" => "LIKE",
                _ => "LIKE"
            };
            query.push_str(&format!(" WHERE {} {} '%{}%'", column_name, like_operator, search_term));
        }
    }
    
    query.push_str(&format!(" LIMIT {}", limit_val));
    query
}

/// Get all row IDs for a table (for bulk selection)
#[handler]
pub async fn get_table_row_ids(
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

    let request_data: RowIdsRequest = req.parse_json().await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    // Get datasource and verify ownership using cache
    let cached_datasource = get_cached_datasource(&datasource_id, &user_id, is_current_user_root(depot), &state.db_pool).await?;
    let source_type = cached_datasource.datasource_type.clone();
    let mut config = cached_datasource.connection_config.clone();
    
    // Add datasource ID to config for the connector
    config.as_object_mut()
        .ok_or_else(|| AppError::InternalServerError("Invalid config format".to_string()))?
        .insert("id".to_string(), Value::String(datasource_id.clone()));

    // Create connector using factory
    let connector = create_connector(&source_type, &config)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to create connector: {}", e)))?;

    // Build query to get the primary key or first column
    let limit = request_data.limit.unwrap_or(10000); // Default limit for performance
    
    // First, try to get table structure to find the primary key
    let mut id_column = request_data.id_column.unwrap_or_else(|| "id".to_string());
    
    // For now, let's try a more robust approach - get the first column
    // This matches what the table data query does
    let query = format!("SELECT * FROM {} LIMIT 1", table_name);
    let structure_result = connector.execute_query(&query, 1).await
        .map_err(|e| AppError::InternalServerError(format!("Failed to get table structure: {}", e)))?;
    
    // Extract the first column name from the structure
    if let Some(columns) = structure_result.get("columns").and_then(|c| c.as_array()) {
        if let Some(first_col) = columns.get(0).and_then(|c| c.as_str()) {
            id_column = first_col.to_string();
            tracing::info!("Using first column '{}' as ID column for table {}", id_column, table_name);
        }
    }
    
    let actual_query = format!("SELECT {} FROM {} LIMIT {}", id_column, table_name, limit);

    // Execute the actual query using connector
    tracing::info!("Executing query: {}", actual_query);
    let result = connector.execute_query(&actual_query, limit).await
        .map_err(|e| AppError::InternalServerError(format!("Query execution failed: {}", e)))?;

    // Extract row IDs from result
    let mut row_ids = Vec::new();
    
    // Log the raw result for debugging
    tracing::info!("Raw query result: {}", serde_json::to_string_pretty(&result).unwrap_or_else(|_| "Failed to serialize".to_string()));
    
    if let Some(data) = result.get("data").and_then(|d| d.as_array()) {
        for row in data {
            if let Some(obj) = row.as_object() {
                // Try to get the ID value by column name first
                if let Some(id_value) = obj.get(&id_column) {
                    match id_value {
                        Value::String(s) => row_ids.push(s.clone()),
                        Value::Number(n) => row_ids.push(n.to_string()),
                        _ => row_ids.push(id_value.to_string()),
                    }
                } else {
                    // If column name doesn't work, try to get the first value
                    if let Some((_, first_value)) = obj.iter().next() {
                        match first_value {
                            Value::String(s) => row_ids.push(s.clone()),
                            Value::Number(n) => row_ids.push(n.to_string()),
                            _ => row_ids.push(first_value.to_string()),
                        }
                    }
                }
            }
        }
    }
    
    tracing::info!("Extracted {} row IDs", row_ids.len());

    let total_time = request_start.elapsed().as_millis();
    tracing::info!("Row IDs request took {}ms", total_time);

    let response = serde_json::json!({
        "row_ids": row_ids,
        "count": row_ids.len(),
        "id_column": id_column,
        "execution_time_ms": total_time
    });

    res.render(Json(response));
    Ok(())
}











