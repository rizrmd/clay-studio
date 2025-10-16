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

    let schema_info: Option<Value> = datasource_row.get("schema_info");

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

    // Check if force_refresh is requested
    let force_refresh = req.query::<bool>("force_refresh").unwrap_or(false);

    // First check if we have cached table list (unless force_refresh is true)
    if !force_refresh {
        let table_list_row = sqlx::query("SELECT table_list FROM data_sources WHERE id = $1")
            .bind(&datasource_id)
            .fetch_optional(&state.db_pool)
            .await
            .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

        if let Some(row) = table_list_row {
            let cached_table_list: Option<Value> = row.get("table_list");
            if let Some(table_list_value) = cached_table_list {
                if let Ok(tables) = serde_json::from_value::<Vec<String>>(table_list_value) {
                    tracing::debug!("📋 Returning cached table list for datasource {}", datasource_id);
                    res.render(Json(tables));
                    return Ok(());
                }
            }
        }
    } else {
        tracing::info!("🔄 Force refresh requested for datasource {}", datasource_id);
    }

    // Get datasource and verify ownership using cache
    let cached_datasource = get_cached_datasource(&datasource_id, &user_id, is_current_user_root(depot), &state.db_pool).await?;

    let source_type = cached_datasource.datasource_type.clone();
    let mut config = cached_datasource.connection_config.clone();

    tracing::info!("📋 Getting tables for datasource {} (type: {})", datasource_id, source_type);
    tracing::debug!("Config (without sensitive data): {:?}", {
        let mut safe_config = config.clone();
        if let Some(obj) = safe_config.as_object_mut() {
            if obj.contains_key("password") {
                obj.insert("password".to_string(), serde_json::Value::String("***".to_string()));
            }
        }
        safe_config
    });

    // Add datasource ID to config for the connector
    config.as_object_mut()
        .ok_or_else(|| AppError::InternalServerError("Invalid config format".to_string()))?
        .insert("id".to_string(), Value::String(datasource_id.clone()));

    // Get tables based on source type using cached connection pools
    let result = match source_type.as_str() {
        "postgresql" | "mysql" | "sqlite" => {
            list_tables(&datasource_id, &config, &source_type).await
                .map_err(|e| {
                    tracing::error!("❌ Failed to list tables for datasource {}: {}", datasource_id, e);
                    AppError::InternalServerError(format!("Failed to list tables: {}", e))
                })?
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
        "csv" | "excel" | "json" => {
            // For file datasources, use the connector factory directly
            list_file_tables(&datasource_id, &config, &source_type).await
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
    
    // Check if force_refresh is requested
    let force_refresh = req.query::<bool>("force_refresh").unwrap_or(false);

    // First check if we have cached schema info for this table (unless force_refresh is true)
    if !force_refresh {
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
                            // Check if the cached structure has actual data (not empty columns)
                            if !structure.columns.is_empty() {
                                println!("DEBUG: Returning cached table structure with {} columns", structure.columns.len());
                                res.render(Json(structure));
                                return Ok(());
                            }
                            // If columns are empty, fall through to fetch fresh data
                            println!("DEBUG: Cached table structure has empty columns, fetching fresh data");
                        }
                    }
                }
            }
        }
    } else {
        println!("DEBUG: Force refresh requested, bypassing cache");
    }

    // Get datasource and verify ownership using cache
    let cached_datasource = get_cached_datasource(&datasource_id, &user_id, is_current_user_root(depot), &state.db_pool).await?;
    
    let source_type = cached_datasource.datasource_type.clone();
    let mut config = cached_datasource.connection_config.clone();
    
    println!("DEBUG: Getting table structure for datasource_id: {}, table: {}, source_type: {}", datasource_id, table_name, source_type);
    println!("DEBUG: Config before modification: {:?}", config);
    
    // Add datasource ID to config for the connector
    config.as_object_mut()
        .ok_or_else(|| AppError::InternalServerError("Invalid config format".to_string()))?
        .insert("id".to_string(), Value::String(datasource_id.clone()));

    println!("DEBUG: Config after adding ID: {:?}", config);
    
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
        "csv" | "excel" | "json" => {
            // For file datasources, use the connector factory directly
            get_file_table_structure(&datasource_id, &config, &source_type, &table_name).await
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
            let schema_json: Option<Value> = row.get("schema_info");
            schema_json
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

async fn list_tables(
    _datasource_id: &str,
    config: &Value,
    source_type: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    use crate::utils::datasource::create_connector;

    // Create connector using factory
    let connector = create_connector(source_type, config).await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(std::io::Error::other(e.to_string()))
        })?;

    // Get tables using connector's list_tables method
    connector.list_tables().await
}

async fn list_clickhouse_tables(
    _datasource_id: &str,
    config: &Value,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    use crate::utils::datasource::create_connector;

    // Create connector using factory
    let connector = create_connector("clickhouse", config).await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(std::io::Error::other(e.to_string()))
        })?;

    // Get tables using connector's list_tables method
    connector.list_tables().await
}

async fn list_oracle_tables(
    _datasource_id: &str,
    config: &Value,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    use crate::utils::datasource::create_connector;

    // Create connector using factory
    let connector = create_connector("oracle", config).await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(std::io::Error::other(e.to_string()))
        })?;

    // Get tables using connector's list_tables method
    connector.list_tables().await
}

async fn list_sqlserver_tables(
    _datasource_id: &str,
    config: &Value,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    use crate::utils::datasource::create_connector;

    // Create connector using factory
    let connector = create_connector("sqlserver", config).await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(std::io::Error::other(e.to_string()))
        })?;

    // Get tables using connector's list_tables method
    connector.list_tables().await
}

async fn list_file_tables(
    _datasource_id: &str,
    config: &Value,
    source_type: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    use crate::utils::datasource::create_connector;

    // Create connector using factory
    let connector = create_connector(source_type, config).await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(std::io::Error::other(e.to_string()))
        })?;

    // Get tables using connector's list_tables method
    connector.list_tables().await
}

async fn get_postgres_table_structure(
    _datasource_id: &str,
    config: &Value,
    table_name: &str,
) -> Result<TableStructure, Box<dyn std::error::Error + Send + Sync>> {
    use super::types::{TableColumn, ForeignKeyInfo, IndexInfo};
    
    // Get the schema name from config, default to 'public'
    let schema_name = config.as_object()
        .and_then(|obj| obj.get("schema"))
        .and_then(|v| v.as_str())
        .unwrap_or("public");
    
    println!("DEBUG: Using schema '{}' for table '{}'", schema_name, table_name);
    
    // Build connection URL
    let connection_url = if let Some(url) = config.as_str() {
        url.to_string()
    } else if let Some(obj) = config.as_object() {
        if let Some(url) = obj.get("url").and_then(|v| v.as_str()) {
            url.to_string()
        } else {
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
        return Err("Invalid configuration format".into());
    };

    // Connect to database
    let pool = sqlx::postgres::PgPool::connect(&connection_url).await?;
    
    // Get column information
    let columns_query = r#"
        SELECT 
            c.column_name,
            c.data_type,
            c.is_nullable,
            c.column_default,
            c.character_maximum_length,
            c.numeric_precision,
            c.numeric_scale,
            CASE WHEN pk.column_name IS NOT NULL THEN true ELSE false END as is_primary_key,
            CASE WHEN fk.column_name IS NOT NULL THEN true ELSE false END as is_foreign_key
        FROM information_schema.columns c
        LEFT JOIN (
            SELECT ku.column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage ku
                ON tc.constraint_name = ku.constraint_name
                AND tc.table_schema = ku.table_schema
            WHERE tc.constraint_type = 'PRIMARY KEY'
                AND tc.table_name = $1
                AND tc.table_schema = $2
        ) pk ON c.column_name = pk.column_name
        LEFT JOIN (
            SELECT DISTINCT ku.column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage ku
                ON tc.constraint_name = ku.constraint_name
                AND tc.table_schema = ku.table_schema
            WHERE tc.constraint_type = 'FOREIGN KEY'
                AND tc.table_name = $1
                AND tc.table_schema = $2
        ) fk ON c.column_name = fk.column_name
        WHERE c.table_name = $1
            AND c.table_schema = $2
        ORDER BY c.ordinal_position
    "#;
    
    let column_rows = sqlx::query(columns_query)
        .bind(table_name)
        .bind(schema_name)
        .fetch_all(&pool)
        .await?;
    
    let mut columns = Vec::new();
    let mut primary_keys = Vec::new();
    
    for row in column_rows {
        let column_name: String = row.get("column_name");
        let is_primary_key: bool = row.get("is_primary_key");
        
        if is_primary_key {
            primary_keys.push(column_name.clone());
        }
        
        columns.push(TableColumn {
            name: column_name,
            data_type: row.get("data_type"),
            is_nullable: row.get::<&str, _>("is_nullable") == "YES",
            column_default: row.get("column_default"),
            is_primary_key,
            is_foreign_key: row.get("is_foreign_key"),
            character_maximum_length: row.get("character_maximum_length"),
            numeric_precision: row.get("numeric_precision"),
            numeric_scale: row.get("numeric_scale"),
        });
    }
    
    // Get foreign key information
    let fk_query = r#"
        SELECT
            kcu.column_name,
            ccu.table_name AS referenced_table,
            ccu.column_name AS referenced_column
        FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage kcu
            ON tc.constraint_name = kcu.constraint_name
            AND tc.table_schema = kcu.table_schema
        JOIN information_schema.constraint_column_usage ccu
            ON ccu.constraint_name = tc.constraint_name
            AND ccu.table_schema = tc.table_schema
        WHERE tc.constraint_type = 'FOREIGN KEY'
            AND tc.table_name = $1
            AND tc.table_schema = $2
    "#;
    
    let fk_rows = sqlx::query(fk_query)
        .bind(table_name)
        .bind(schema_name)
        .fetch_all(&pool)
        .await?;
    
    let foreign_keys: Vec<ForeignKeyInfo> = fk_rows.iter().map(|row| {
        ForeignKeyInfo {
            column_name: row.get("column_name"),
            referenced_table: row.get("referenced_table"),
            referenced_column: row.get("referenced_column"),
        }
    }).collect();
    
    // Get index information
    let index_query = r#"
        SELECT
            i.indexname as name,
            array_agg(a.attname ORDER BY array_position(ix.indkey, a.attnum)) as columns,
            ix.indisunique as is_unique
        FROM pg_indexes i
        JOIN pg_class c ON c.relname = i.tablename
        JOIN pg_index ix ON ix.indexrelid = (i.schemaname || '.' || i.indexname)::regclass
        JOIN pg_attribute a ON a.attrelid = c.oid AND a.attnum = ANY(ix.indkey)
        WHERE i.tablename = $1
            AND i.schemaname = $2
            AND NOT ix.indisprimary
        GROUP BY i.indexname, ix.indisunique
    "#;
    
    let index_rows = sqlx::query(index_query)
        .bind(table_name)
        .bind(schema_name)
        .fetch_all(&pool)
        .await?;
    
    let indexes: Vec<IndexInfo> = index_rows.iter().map(|row| {
        IndexInfo {
            name: row.get("name"),
            columns: row.get("columns"),
            is_unique: row.get("is_unique"),
        }
    }).collect();
    
    Ok(TableStructure {
        table_name: table_name.to_string(),
        columns,
        primary_keys,
        foreign_keys,
        indexes,
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

async fn get_file_table_structure(
    _datasource_id: &str,
    config: &Value,
    source_type: &str,
    table_name: &str,
) -> Result<TableStructure, Box<dyn std::error::Error + Send + Sync>> {
    use super::types::TableColumn;
    use crate::utils::datasource::create_connector;

    // Create connector using factory
    let connector = create_connector(source_type, config).await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(std::io::Error::other(e.to_string()))
        })?;

    // Get schema from connector
    let schema = connector.fetch_schema().await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(std::io::Error::other(e.to_string()))
        })?;

    // Extract table structure from schema
    if let Some(tables) = schema.get("tables") {
        if let Some(table_data) = tables.get(table_name) {
            // Extract columns from the table data
            let columns_array = table_data.get("columns")
                .and_then(|c| c.as_array())
                .unwrap_or(&serde_json::json!([]));

            let mut columns = Vec::new();
            for col_data in columns_array {
                if let Some(col_obj) = col_data.as_object() {
                    columns.push(TableColumn {
                        name: col_obj.get("column_name")
                            .and_then(|s| s.as_str())
                            .unwrap_or("unknown").to_string(),
                        data_type: col_obj.get("data_type")
                            .and_then(|s| s.as_str())
                            .unwrap_or("text").to_string(),
                        is_nullable: col_obj.get("is_nullable")
                            .and_then(|s| s.as_str())
                            .unwrap_or("YES") == "YES",
                        column_default: None,
                        is_primary_key: false, // File datasources don't have primary keys
                        is_foreign_key: false, // File datasources don't have foreign keys
                        character_maximum_length: None,
                        numeric_precision: None,
                        numeric_scale: None,
                    });
                }
            }

            Ok(TableStructure {
                table_name: table_name.to_string(),
                columns,
                primary_keys: vec![],
                foreign_keys: vec![],
                indexes: vec![],
            })
        } else {
            Err(format!("Table '{}' not found in schema", table_name).into())
        }
    } else {
        Err("No tables found in schema".into())
    }
}