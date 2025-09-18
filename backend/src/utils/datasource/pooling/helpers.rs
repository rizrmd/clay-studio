//! Connection pool helpers for different executables
//! 
//! This module provides utilities to easily integrate connection pooling
//! into different parts of the application (API, MCP server, etc.)
//! For SQLx databases (PostgreSQL, MySQL, SQLite), it uses global connection pools
//! For other databases, it falls back to individual connector instances

use crate::utils::datasource::{create_connector, get_pool_manager, DatabasePool};
use super::clickhouse_client_pool::get_clickhouse_client_pool;
use serde_json::Value;
use std::error::Error;
use sqlx::Column;

/// Execute a query using the appropriate pooling mechanism
/// - SQLx databases (PostgreSQL, MySQL, SQLite): Global connection pools
/// - ClickHouse: Client pooling (HTTP connection reuse)
/// - SQL Server: Individual connectors (Tiberius client ownership constraints)
/// - Other databases: Individual connector instances
pub async fn execute_query_with_pooling(
    datasource_id: &str,
    source_type: &str,
    config: &Value,
    query: &str,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    match source_type.to_lowercase().as_str() {
        "clickhouse" | "ch" => {
            // Use ClickHouse client pooling
            tracing::info!("Using pooled ClickHouse client for datasource {}", datasource_id);
            let client_pool = get_clickhouse_client_pool().await;
            let client = client_pool.get_client(datasource_id, config).await
                .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn Error + Send + Sync>)?;
            
            // Execute query with the pooled client
            execute_clickhouse_query_with_client(&client, query).await
        },
        "sqlserver" | "mssql" => {
            // SQL Server uses individual connectors due to Tiberius client ownership constraints
            // The existing SqlServerConnector already has basic connection caching
            tracing::info!("Using individual connector for SQL Server (Tiberius limitations)");
            execute_with_individual_connector(source_type, config, query).await
        },
        "postgresql" | "postgres" | "mysql" | "sqlite" => {
            // Try to use global pool for SQLx databases
            let pool_manager = get_pool_manager().await;
            match pool_manager.get_pool(datasource_id, source_type, config).await {
                Ok(db_pool) => {
                    // Use pooled connection for SQLx databases
                    match db_pool {
                        DatabasePool::PostgreSQL(pool) => {
                            let rows = sqlx::query(query).fetch_all(pool.as_ref()).await?;
                            convert_postgres_rows_to_json(rows).await
                        },
                        DatabasePool::MySQL(pool) => {
                            let rows = sqlx::query(query).fetch_all(pool.as_ref()).await?;
                            convert_mysql_rows_to_json(rows).await
                        },
                        DatabasePool::SQLite(pool) => {
                            let rows = sqlx::query(query).fetch_all(pool.as_ref()).await?;
                            convert_sqlite_rows_to_json(rows).await
                        }
                    }
                },
                Err(_) => {
                    // Fall back to individual connector if pooling fails
                    tracing::warn!("Pool creation failed for {} datasource {}, falling back to individual connector", source_type, datasource_id);
                    execute_with_individual_connector(source_type, config, query).await
                }
            }
        },
        _ => {
            // Fall back to individual connector for other databases
            tracing::info!("Using individual connector for {} (no pooling support)", source_type);
            execute_with_individual_connector(source_type, config, query).await
        }
    }
}

/// Execute query using a pooled ClickHouse client
async fn execute_clickhouse_query_with_client(
    client: &clickhouse::Client, 
    query: &str
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    use std::time::Instant;
    
    // Add LIMIT if not present (safety measure)
    let query_with_limit = if query.to_lowercase().contains("limit") {
        query.to_string()
    } else {
        format!("{} LIMIT 1000000", query)
    };

    let start = Instant::now();
    let query_with_format = format!("{} FORMAT JSON", query_with_limit);

    // Execute query with the pooled client
    let raw_result = client
        .query(&query_with_format)
        .fetch_one::<String>()
        .await;

    let execution_time_ms = start.elapsed().as_millis() as i64;

    match raw_result {
        Ok(json_str) => {
            // Parse the JSON response from ClickHouse
            if let Ok(json_response) = serde_json::from_str::<Value>(&json_str) {
                let meta = json_response.get("meta").and_then(|m| m.as_array());
                let data = json_response.get("data").and_then(|d| d.as_array());

                if let (Some(meta_array), Some(data_array)) = (meta, data) {
                    // Extract column names from metadata
                    let columns: Vec<String> = meta_array
                        .iter()
                        .filter_map(|col| col.get("name").and_then(|n| n.as_str()))
                        .map(str::to_string)
                        .collect();

                    // Convert to standardized format - match original connector format
                    Ok(serde_json::json!({
                        "columns": columns,
                        "rows": data_array,
                        "row_count": data_array.len(),
                        "execution_time_ms": execution_time_ms
                    }))
                } else {
                    Ok(serde_json::json!({
                        "columns": [],
                        "rows": [],
                        "row_count": 0,
                        "execution_time_ms": execution_time_ms
                    }))
                }
            } else {
                // Fallback for non-JSON responses
                Ok(serde_json::json!({
                    "columns": ["result"],
                    "rows": [[json_str]],
                    "row_count": 1,
                    "execution_time_ms": execution_time_ms
                }))
            }
        }
        Err(e) => Err(Box::new(e) as Box<dyn Error + Send + Sync>)
    }
}


/// Execute query using individual connector (fallback)
async fn execute_with_individual_connector(
    source_type: &str,
    config: &Value,
    query: &str
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let connector = create_connector(source_type, config).await
        .map_err(|e| Box::new(std::io::Error::other(e.to_string())) as Box<dyn Error + Send + Sync>)?;
    let result = connector.execute_query(query, 1000000).await
        .map_err(|e| Box::new(std::io::Error::other(e.to_string())) as Box<dyn Error + Send + Sync>)?;
    
    // The individual connector already returns the correct format
    // (columns, rows, row_count, execution_time_ms)
    // So we can return it directly
    Ok(result)
}

/// Get a connection pool for direct use
/// This is useful when you need to perform multiple operations on the same connection
#[allow(dead_code)]
pub async fn get_connection_pool(
    datasource_id: &str,
    source_type: &str,
    config: &Value,
) -> Result<DatabasePool, Box<dyn Error + Send + Sync>> {
    let pool_manager = get_pool_manager().await;
    pool_manager.get_pool(datasource_id, source_type, config).await
        .map_err(Box::<dyn Error + Send + Sync>::from)
}

// Helper functions to convert different row types to JSON
async fn convert_postgres_rows_to_json(
    rows: Vec<sqlx::postgres::PgRow>,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    use serde_json::json;
    use sqlx::Row;
    
    if rows.is_empty() {
        return Ok(json!({"data": [], "columns": []}));
    }
    
    let columns: Vec<String> = rows[0].columns().iter().map(|c| c.name().to_string()).collect();
    let data: Vec<Value> = rows.into_iter().map(|row| {
        let mut obj = serde_json::Map::new();
        for (i, col) in columns.iter().enumerate() {
            // Try different types
            if let Ok(val) = row.try_get::<Option<String>, _>(i) {
                obj.insert(col.clone(), json!(val));
            } else if let Ok(val) = row.try_get::<Option<i64>, _>(i) {
                obj.insert(col.clone(), json!(val));
            } else if let Ok(val) = row.try_get::<Option<f64>, _>(i) {
                obj.insert(col.clone(), json!(val));
            } else if let Ok(val) = row.try_get::<Option<bool>, _>(i) {
                obj.insert(col.clone(), json!(val));
            } else {
                obj.insert(col.clone(), json!(null));
            }
        }
        Value::Object(obj)
    }).collect();
    
    // Convert object data back to array format for consistency with original connectors
    let rows: Vec<Vec<Value>> = data.iter().map(|row_obj| {
        columns.iter().map(|col| {
            row_obj.get(col).cloned().unwrap_or(json!(null))
        }).collect()
    }).collect();
    
    Ok(json!({
        "columns": columns,
        "rows": rows,
        "row_count": rows.len()
    }))
}

async fn convert_mysql_rows_to_json(
    rows: Vec<sqlx::mysql::MySqlRow>,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    use serde_json::json;
    use sqlx::Row;
    
    if rows.is_empty() {
        return Ok(json!({"columns": [], "rows": [], "row_count": 0}));
    }
    
    let columns: Vec<String> = rows[0].columns().iter().map(|c| c.name().to_string()).collect();
    let data: Vec<Value> = rows.into_iter().map(|row| {
        let mut obj = serde_json::Map::new();
        for (i, col) in columns.iter().enumerate() {
            // Try different types
            if let Ok(val) = row.try_get::<Option<String>, _>(i) {
                obj.insert(col.clone(), json!(val));
            } else if let Ok(val) = row.try_get::<Option<i64>, _>(i) {
                obj.insert(col.clone(), json!(val));
            } else if let Ok(val) = row.try_get::<Option<f64>, _>(i) {
                obj.insert(col.clone(), json!(val));
            } else if let Ok(val) = row.try_get::<Option<bool>, _>(i) {
                obj.insert(col.clone(), json!(val));
            } else {
                obj.insert(col.clone(), json!(null));
            }
        }
        Value::Object(obj)
    }).collect();
    
    // Convert object data back to array format for consistency with original connectors
    let rows: Vec<Vec<Value>> = data.iter().map(|row_obj| {
        columns.iter().map(|col| {
            row_obj.get(col).cloned().unwrap_or(json!(null))
        }).collect()
    }).collect();
    
    Ok(json!({
        "columns": columns,
        "rows": rows,
        "row_count": rows.len()
    }))
}

async fn convert_sqlite_rows_to_json(
    rows: Vec<sqlx::sqlite::SqliteRow>,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    use serde_json::json;
    use sqlx::Row;
    
    if rows.is_empty() {
        return Ok(json!({"columns": [], "rows": [], "row_count": 0}));
    }
    
    let columns: Vec<String> = rows[0].columns().iter().map(|c| c.name().to_string()).collect();
    let data: Vec<Value> = rows.into_iter().map(|row| {
        let mut obj = serde_json::Map::new();
        for (i, col) in columns.iter().enumerate() {
            // Try different types
            if let Ok(val) = row.try_get::<Option<String>, _>(i) {
                obj.insert(col.clone(), json!(val));
            } else if let Ok(val) = row.try_get::<Option<i64>, _>(i) {
                obj.insert(col.clone(), json!(val));
            } else if let Ok(val) = row.try_get::<Option<f64>, _>(i) {
                obj.insert(col.clone(), json!(val));
            } else if let Ok(val) = row.try_get::<Option<bool>, _>(i) {
                obj.insert(col.clone(), json!(val));
            } else {
                obj.insert(col.clone(), json!(null));
            }
        }
        Value::Object(obj)
    }).collect();
    
    // Convert object data back to array format for consistency with original connectors
    let rows: Vec<Vec<Value>> = data.iter().map(|row_obj| {
        columns.iter().map(|col| {
            row_obj.get(col).cloned().unwrap_or(json!(null))
        }).collect()
    }).collect();
    
    Ok(json!({
        "columns": columns,
        "rows": rows,
        "row_count": rows.len()
    }))
}