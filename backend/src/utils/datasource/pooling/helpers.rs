//! Connection pool helpers for different executables
//! 
//! This module provides utilities to easily integrate connection pooling
//! into different parts of the application (API, MCP server, etc.)
//! All databases use their respective connectors which handle pooling internally

use crate::utils::datasource::{create_connector, get_pool_manager, DatabasePool};
use serde_json::Value;
use std::error::Error;

/// Execute a query using the appropriate connector with pooling support
/// All databases use their respective connectors which handle:
/// - Connection pooling (where applicable)
/// - Proper type conversion
/// - Consistent result formatting
pub async fn execute_query_with_pooling(
    datasource_id: &str,
    source_type: &str,
    config: &Value,
    query: &str,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    // Always use the connector's execute_query method
    // This ensures consistent type conversion and result formatting
    // The connectors internally handle pooling where applicable
    tracing::info!("Executing query for {} datasource {} using connector", source_type, datasource_id);
    
    let mut config_with_id = config.clone();
    // Ensure datasource_id is in the config (required by some connectors for pooling)
    if let Some(config_obj) = config_with_id.as_object_mut() {
        config_obj.insert("id".to_string(), Value::String(datasource_id.to_string()));
    }
    
    let connector = create_connector(source_type, &config_with_id).await
        .map_err(|e| Box::new(std::io::Error::other(e.to_string())) as Box<dyn Error + Send + Sync>)?;
    
    let result = connector.execute_query(query, 1000000).await
        .map_err(|e| Box::new(std::io::Error::other(e.to_string())) as Box<dyn Error + Send + Sync>)?;
    
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