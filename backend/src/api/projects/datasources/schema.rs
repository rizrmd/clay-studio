use salvo::prelude::*;
use serde_json::Value;
use sqlx::Row;

use crate::utils::middleware::{get_current_user_id, is_current_user_root};
use crate::utils::{get_app_state, AppError};

use super::crud::get_cached_datasource;
use super::types::TableStructure;

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
    let mut config = cached_datasource.connection_config.clone();
    
    // Add datasource ID to config for the connector
    config.as_object_mut()
        .ok_or_else(|| AppError::InternalServerError("Invalid config format".to_string()))?
        .insert("id".to_string(), Value::String(datasource_id.clone()));

    // Get tables based on source type using cached connection pools
    let result = match source_type.as_str() {
        "postgresql" | "mysql" | "sqlite" => {
            list_tables(&datasource_id, &config, &source_type).await
                .map_err(|e| AppError::InternalServerError(format!("Failed to list tables: {}", e)))?
        },
        "clickhouse" => {
            list_clickhouse_tables(&datasource_id, &config).await
                .map_err(|e| AppError::InternalServerError(format!("Failed to list tables: {}", e)))?
        },
        "oracle" => {
            list_oracle_tables(&datasource_id, &config).await
                .map_err(|e| AppError::InternalServerError(format!("Failed to list tables: {}", e)))?
        },
        "sqlserver" => {
            list_sqlserver_tables(&datasource_id, &config).await
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
    let mut config = cached_datasource.connection_config.clone();
    
    // Add datasource ID to config for the connector
    config.as_object_mut()
        .ok_or_else(|| AppError::InternalServerError("Invalid config format".to_string()))?
        .insert("id".to_string(), Value::String(datasource_id.clone()));

    // Get table structure based on source type using cached connection pools
    let result = match source_type.as_str() {
        "postgresql" => {
            get_postgres_table_structure(&datasource_id, &config, &table_name).await
                .map_err(|e| AppError::InternalServerError(format!("Failed to get table structure: {}", e)))?
        },
        "mysql" => {
            get_mysql_table_structure(&datasource_id, &config, &table_name).await
                .map_err(|e| AppError::InternalServerError(format!("Failed to get table structure: {}", e)))?
        },
        "sqlite" => {
            get_sqlite_table_structure(&datasource_id, &config, &table_name).await
                .map_err(|e| AppError::InternalServerError(format!("Failed to get table structure: {}", e)))?
        },
        "clickhouse" => {
            get_clickhouse_table_structure(&datasource_id, &config, &table_name).await
                .map_err(|e| AppError::InternalServerError(format!("Failed to get table structure: {}", e)))?
        },
        "oracle" => {
            get_oracle_table_structure(&datasource_id, &config, &table_name).await
                .map_err(|e| AppError::InternalServerError(format!("Failed to get table structure: {}", e)))?
        },
        "sqlserver" => {
            get_sqlserver_table_structure(&datasource_id, &config, &table_name).await
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

/// Update schema_info with table structure information  
pub async fn update_schema_info_with_table_structure(
    db_pool: &sqlx::PgPool,
    datasource_id: &str,
    table_name: &str,
    table_structure: &TableStructure,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get current schema_info
    let schema_info_row = sqlx::query("SELECT schema_info FROM data_sources WHERE id = $1")
        .bind(datasource_id)
        .fetch_optional(db_pool)
        .await?;

    let mut schema_info: Value = schema_info_row
        .and_then(|row| {
            let schema_str: Option<String> = row.get("schema_info");
            schema_str.and_then(|s| serde_json::from_str(&s).ok())
        })
        .unwrap_or_else(|| serde_json::json!({}));

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

// Placeholder functions - these will need implementations moved from the original file
async fn list_tables(
    _datasource_id: &str,
    _config: &Value,
    _source_type: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(vec![])
}

async fn list_clickhouse_tables(
    _datasource_id: &str,
    _config: &Value,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(vec![])
}

async fn list_oracle_tables(
    _datasource_id: &str,
    _config: &Value,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(vec![])
}

async fn list_sqlserver_tables(
    _datasource_id: &str,
    _config: &Value,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(vec![])
}

async fn get_postgres_table_structure(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
) -> Result<TableStructure, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(TableStructure {
        table_name: _table_name.to_string(),
        columns: vec![],
        primary_keys: vec![],
        foreign_keys: vec![],
        indexes: vec![],
    })
}

async fn get_mysql_table_structure(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
) -> Result<TableStructure, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(TableStructure {
        table_name: _table_name.to_string(),
        columns: vec![],
        primary_keys: vec![],
        foreign_keys: vec![],
        indexes: vec![],
    })
}

async fn get_sqlite_table_structure(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
) -> Result<TableStructure, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(TableStructure {
        table_name: _table_name.to_string(),
        columns: vec![],
        primary_keys: vec![],
        foreign_keys: vec![],
        indexes: vec![],
    })
}

async fn get_clickhouse_table_structure(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
) -> Result<TableStructure, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(TableStructure {
        table_name: _table_name.to_string(),
        columns: vec![],
        primary_keys: vec![],
        foreign_keys: vec![],
        indexes: vec![],
    })
}

async fn get_oracle_table_structure(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
) -> Result<TableStructure, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(TableStructure {
        table_name: _table_name.to_string(),
        columns: vec![],
        primary_keys: vec![],
        foreign_keys: vec![],
        indexes: vec![],
    })
}

async fn get_sqlserver_table_structure(
    _datasource_id: &str,
    _config: &Value,
    _table_name: &str,
) -> Result<TableStructure, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Move implementation from original datasources.rs
    Ok(TableStructure {
        table_name: _table_name.to_string(),
        columns: vec![],
        primary_keys: vec![],
        foreign_keys: vec![],
        indexes: vec![],
    })
}