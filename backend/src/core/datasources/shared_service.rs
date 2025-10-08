//! Shared datasource service for use across different parts of the application
//! This module provides common datasource operations that can be reused by both API and MCP server

use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use crate::core::datasources::cache::{get_datasource_cache, CachedDatasource};
use crate::utils::datasource::{create_connector, pooling::execute_query_with_pooling};

/// Shared datasource information structure
#[derive(Debug, Clone)]
pub struct SharedDatasourceInfo {
    pub id: String,
    pub name: String,
    pub source_type: String,
    pub connection_config: Value,
    pub project_id: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Get datasource with caching and ownership validation
/// This is a shared version that doesn't depend on web framework-specific types
pub async fn get_datasource_with_validation(
    datasource_id: &str,
    project_id: &str,
    db_pool: &PgPool,
) -> Result<SharedDatasourceInfo, Box<dyn std::error::Error + Send + Sync>> {
    // Check cache first
    let cache = get_datasource_cache().await;
    
    // For MCP, we don't have user_id in the same way, so we'll use project_id as part of cache key
    let _cache_key = format!("{}:{}", datasource_id, project_id);
    
    // Try to get from cache
    let cached = cache.get(datasource_id, project_id).await;
    if let Some(cached) = cached {
        return Ok(SharedDatasourceInfo {
            id: cached.id,
            name: cached.name,
            source_type: cached.datasource_type,
            connection_config: cached.connection_config,
            project_id: cached.project_id,
            created_at: None, // Cache doesn't store created_at
        });
    }
    
    // Cache miss - fetch from database
    let row = sqlx::query(
        r#"
        SELECT id, name, source_type, connection_config, project_id, created_at
        FROM data_sources 
        WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL
        "#
    )
    .bind(datasource_id)
    .bind(project_id)
    .fetch_optional(db_pool)
    .await?
    .ok_or("Datasource not found")?;

    let datasource = SharedDatasourceInfo {
        id: row.get("id"),
        name: row.get("name"),
        source_type: row.get("source_type"),
        connection_config: row.get("connection_config"),
        project_id: row.get("project_id"),
        created_at: row.get("created_at"),
    };

    // Cache the result
    let cached_datasource = CachedDatasource {
        id: datasource.id.clone(),
        name: datasource.name.clone(),
        datasource_type: datasource.source_type.clone(),
        connection_config: datasource.connection_config.clone(),
        user_id: Uuid::nil(), // Not used in MCP context
        project_id: datasource.project_id.clone(),
        cached_at: std::time::Instant::now(),
    };
    
    cache.set(cached_datasource).await;

    Ok(datasource)
}

/// Execute a query using the shared datasource service
/// This provides a consistent way to execute queries across different parts of the application
pub async fn execute_query_on_datasource(
    datasource_id: &str,
    project_id: &str,
    query: &str,
    db_pool: &PgPool,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    
    // Get datasource info
    let datasource = get_datasource_with_validation(datasource_id, project_id, db_pool).await?;
    
    // Add datasource ID to config if not present (needed by connectors)
    let mut config_with_id = datasource.connection_config.clone();
    if config_with_id.is_object() {
        let config_obj = config_with_id.as_object_mut().unwrap();
        config_obj.insert("id".to_string(), Value::String(datasource_id.to_string()));
    }
    
    // Execute query using pooling
    execute_query_with_pooling(
        datasource_id,
        &datasource.source_type,
        &config_with_id,
        query
    ).await
}

/// List datasources for a project
pub async fn list_datasources_for_project(
    project_id: &str,
    db_pool: &PgPool,
) -> Result<Vec<SharedDatasourceInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, source_type, connection_config, project_id, created_at
        FROM data_sources 
        WHERE project_id = $1 AND deleted_at IS NULL
        ORDER BY created_at DESC
        "#
    )
    .bind(project_id)
    .fetch_all(db_pool)
    .await?;

    let datasources: Vec<SharedDatasourceInfo> = rows
        .into_iter()
        .map(|row| SharedDatasourceInfo {
            id: row.get("id"),
            name: row.get("name"),
            source_type: row.get("source_type"),
            connection_config: row.get("connection_config"),
            project_id: row.get("project_id"),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(datasources)
}

/// Test a datasource connection
pub async fn test_datasource_connection(
    source_type: &str,
    connection_config: &Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut connector = create_connector(source_type, connection_config).await
        .map_err(|e| Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error + Send + Sync>)?;
    connector.test_connection().await
        .map_err(|e| Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error + Send + Sync>)?;
    Ok(())
}

/// Test a datasource connection directly without requiring an ID
/// This is used for initial connection validation before creating a datasource
pub async fn test_datasource_connection_direct(
    source_type: &str,
    connection_config: &Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match source_type.to_lowercase().as_str() {
        "postgresql" | "postgres" => {
            test_postgres_connection_direct(connection_config).await
        },
        "mysql" => {
            test_mysql_connection_direct(connection_config).await
        },
        "sqlite" => {
            test_sqlite_connection_direct(connection_config).await
        },
        _ => {
            // For other database types, fall back to the connector-based approach
            // but with a temporary ID for testing
            let mut config_with_temp_id = connection_config.clone();
            if let Some(obj) = config_with_temp_id.as_object_mut() {
                obj.insert("id".to_string(), Value::String("temp-test-id".to_string()));
            }
            test_datasource_connection(source_type, &config_with_temp_id).await
        }
    }
}

/// Test PostgreSQL connection directly using SQLx
pub async fn test_postgres_connection_direct(config: &Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let connection_url = build_postgres_url_from_config(config)?;
    
    let pool = sqlx::postgres::PgPool::connect(&connection_url).await
        .map_err(|e| format!("Failed to connect to PostgreSQL: {}", e))?;
    
    // Test with a simple query
    sqlx::query("SELECT 1").fetch_one(&pool).await
        .map_err(|e| format!("Connection established but query failed: {}", e))?;
    
    Ok(())
}

/// Test MySQL connection directly using SQLx
async fn test_mysql_connection_direct(config: &Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let connection_url = build_mysql_url_from_config(config)?;
    
    let pool = sqlx::mysql::MySqlPool::connect(&connection_url).await
        .map_err(|e| format!("Failed to connect to MySQL: {}", e))?;
    
    // Test with a simple query
    sqlx::query("SELECT 1").fetch_one(&pool).await
        .map_err(|e| format!("Connection established but query failed: {}", e))?;
    
    Ok(())
}

/// Test SQLite connection directly using SQLx
async fn test_sqlite_connection_direct(config: &Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let database_path = get_sqlite_path_from_config(config)?;
    
    let pool = sqlx::sqlite::SqlitePool::connect(&format!("sqlite:{}", database_path)).await
        .map_err(|e| format!("Failed to connect to SQLite: {}", e))?;
    
    // Test with a simple query
    sqlx::query("SELECT 1").fetch_one(&pool).await
        .map_err(|e| format!("Connection established but query failed: {}", e))?;
    
    Ok(())
}

/// Build PostgreSQL connection URL from config
pub fn build_postgres_url_from_config(config: &Value) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // If config is a string, treat it as a URL directly
    if let Some(url) = config.as_str() {
        return Ok(url.to_string());
    }
    
    // If config is an object, build URL from components
    let obj = config.as_object()
        .ok_or("Config must be a string URL or object")?;
    
    let host = obj.get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("localhost");
    
    let port = obj.get("port")
        .and_then(|v| v.as_u64())
        .unwrap_or(5432);
    
    let database = obj.get("database")
        .and_then(|v| v.as_str())
        .ok_or("Missing database name")?;
    
    let user = obj.get("user")
        .and_then(|v| v.as_str())
        .or_else(|| obj.get("username").and_then(|v| v.as_str()))
        .unwrap_or("postgres");
    
    let password = obj.get("password")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    
    // Build connection URL
    let connection_url = if password.is_empty() {
        format!("postgresql://{}@{}:{}/{}", user, host, port, database)
    } else {
        format!("postgresql://{}:{}@{}:{}/{}", user, password, host, port, database)
    };
    
    Ok(connection_url)
}

/// Build MySQL connection URL from config
fn build_mysql_url_from_config(config: &Value) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // If config is a string, treat it as a URL directly
    if let Some(url) = config.as_str() {
        return Ok(url.to_string());
    }
    
    // If config is an object, build URL from components
    let obj = config.as_object()
        .ok_or("Config must be a string URL or object")?;
    
    let host = obj.get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("localhost");
    
    let port = obj.get("port")
        .and_then(|v| v.as_u64())
        .unwrap_or(3306);
    
    let database = obj.get("database")
        .and_then(|v| v.as_str())
        .ok_or("Missing database name")?;
    
    let user = obj.get("user")
        .and_then(|v| v.as_str())
        .or_else(|| obj.get("username").and_then(|v| v.as_str()))
        .unwrap_or("root");
    
    let password = obj.get("password")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    
    // Build connection URL
    let connection_url = if password.is_empty() {
        format!("mysql://{}@{}:{}/{}", user, host, port, database)
    } else {
        format!("mysql://{}:{}@{}:{}/{}", user, password, host, port, database)
    };
    
    Ok(connection_url)
}

/// Get SQLite database path from config
fn get_sqlite_path_from_config(config: &Value) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // If config is a string, treat it as a database path
    if let Some(path) = config.as_str() {
        return Ok(path.to_string());
    }
    
    // If config is an object, get the database path
    let obj = config.as_object()
        .ok_or("Config must be a string path or object")?;
    
    obj.get("database")
        .and_then(|v| v.as_str())
        .or_else(|| obj.get("path").and_then(|v| v.as_str()))
        .ok_or_else(|| "Missing database path".into())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_build_postgres_url_from_config_object() {
        let config = json!({
            "database": "pi-smart-stg",
            "host": "159.65.9.242",
            "password": "PasswordRO123!",
            "port": 5432,
            "user": "readonly_user"
        });
        
        let url = build_postgres_url_from_config(&config).unwrap();
        assert_eq!(url, "postgresql://readonly_user:PasswordRO123!@159.65.9.242:5432/pi-smart-stg");
    }

    #[test]
    fn test_build_postgres_url_from_config_string() {
        let config = json!("postgresql://user:pass@localhost:5432/testdb");
        
        let url = build_postgres_url_from_config(&config).unwrap();
        assert_eq!(url, "postgresql://user:pass@localhost:5432/testdb");
    }

    #[test]
    fn test_build_postgres_url_missing_database() {
        let config = json!({
            "host": "localhost",
            "user": "user"
        });
        
        let result = build_postgres_url_from_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing database name"));
    }

    #[tokio::test]
    async fn test_datasource_connection_direct_postgres() {
        let config = json!({
            "database": "pi-smart-stg",
            "host": "159.65.9.242",
            "password": "PasswordRO123!",
            "port": 5432,
            "user": "readonly_user"
        });
        
        // This should fail with connection error, not "Missing datasource ID"
        let result = test_datasource_connection_direct("postgresql", &config).await;
        
        // We expect this to fail (since the database might not be reachable), 
        // but it should NOT fail with "Missing datasource ID"
        match result {
            Ok(_) => println!("✅ Connection successful"),
            Err(e) => {
                let error_msg = e.to_string();
                assert!(!error_msg.contains("Missing datasource ID"), 
                    "Error should not contain 'Missing datasource ID', but got: {}", error_msg);
                println!("✅ Expected connection error: {}", error_msg);
            }
        }
    }
}