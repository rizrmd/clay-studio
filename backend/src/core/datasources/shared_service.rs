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
    
    // Execute query using pooling
    execute_query_with_pooling(
        datasource_id,
        &datasource.source_type,
        &datasource.connection_config,
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