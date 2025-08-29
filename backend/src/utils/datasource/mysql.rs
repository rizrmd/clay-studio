use super::base::DataSourceConnector;
use serde_json::{json, Value};
use sqlx::{mysql::{MySqlPool, MySqlPoolOptions}, Row as SqlxRow, Column};
use std::error::Error;
use async_trait::async_trait;
use std::time::Duration;

pub struct MySQLConnector {
    connection_string: String,
}

impl MySQLConnector {
    async fn create_pool(&self) -> Result<MySqlPool, sqlx::Error> {
        let pool_options = MySqlPoolOptions::new()
            .max_connections(5)
            .min_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Some(Duration::from_secs(30)));
        
        pool_options.connect(&self.connection_string).await
    }
    
    pub fn new(config: &Value) -> Result<Self, Box<dyn Error>> {
        // Prefer URL if provided, otherwise construct from individual components
        let connection_string = if let Some(url) = config.get("url").and_then(|v| v.as_str()) {
            eprintln!("[DEBUG] Using provided MySQL URL directly");
            url.to_string()
        } else {
            // Fallback to individual components for backward compatibility
            let host = config.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
            let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(3306);
            let database = config.get("database").and_then(|v| v.as_str()).ok_or("Missing database name")?;
            let username = config.get("username").and_then(|v| v.as_str()).unwrap_or("root");
            let password = config.get("password").and_then(|v| v.as_str()).unwrap_or("");
            
            eprintln!("[DEBUG] Building MySQL URL from components: host={}, port={}, database={}, username={}", 
                host, port, database, username);
            
            // URL encode username and password to handle special characters
            let encoded_username = urlencoding::encode(username);
            let encoded_password = if password.is_empty() {
                String::new()
            } else {
                urlencoding::encode(password).to_string()
            };
            
            if encoded_password.is_empty() {
                format!("mysql://{}@{}:{}/{}", encoded_username, host, port, database)
            } else {
                format!("mysql://{}:{}@{}:{}/{}", encoded_username, encoded_password, host, port, database)
            }
        };
        
        // Log connection string (masking password for security)
        let masked_conn_str = if connection_string.contains('@') {
            let parts: Vec<&str> = connection_string.splitn(2, '@').collect();
            if parts.len() == 2 {
                // Extract the protocol and auth part
                if let Some(protocol_end) = parts[0].find("://") {
                    let protocol = &parts[0][..protocol_end+3];
                    let auth = &parts[0][protocol_end+3..];
                    
                    // Check if there's a password
                    if auth.contains(':') {
                        let user_parts: Vec<&str> = auth.splitn(2, ':').collect();
                        format!("{}{}:****@{}", protocol, user_parts[0], parts[1])
                    } else {
                        connection_string.clone()
                    }
                } else {
                    "mysql://****".to_string()
                }
            } else {
                connection_string.clone()
            }
        } else {
            connection_string.clone()
        };
        eprintln!("[DEBUG] MySQL connection string (masked): {}", masked_conn_str);
        
        Ok(Self { connection_string })
    }
}

#[async_trait]
impl DataSourceConnector for MySQLConnector {
    async fn test_connection(&self) -> Result<bool, Box<dyn Error>> {
        eprintln!("[DEBUG] Attempting MySQL connection test...");
        
        // Try to connect with a timeout
        match self.create_pool().await {
            Ok(pool) => {
                eprintln!("[DEBUG] MySQL pool created successfully");
                
                // Try a simple query
                match sqlx::query("SELECT 1 as test").fetch_one(&pool).await {
                    Ok(_) => {
                        eprintln!("[DEBUG] MySQL test query successful");
                        pool.close().await;
                        Ok(true)
                    }
                    Err(e) => {
                        eprintln!("[ERROR] MySQL test query failed: {}", e);
                        pool.close().await;
                        Err(Box::new(e) as Box<dyn Error>)
                    }
                }
            }
            Err(e) => {
                eprintln!("[ERROR] Failed to create MySQL connection pool: {}", e);
                eprintln!("[ERROR] Error type: {:?}", e);
                
                // Check if it's a specific type of error
                let error_string = e.to_string();
                if error_string.contains("Access denied") {
                    eprintln!("[ERROR] Authentication failed - check username and password");
                } else if error_string.contains("Unknown database") {
                    eprintln!("[ERROR] Database does not exist");
                } else if error_string.contains("Can't connect") || error_string.contains("Connection refused") {
                    eprintln!("[ERROR] Cannot reach MySQL server - check host and port");
                } else if error_string.contains("timeout") {
                    eprintln!("[ERROR] Connection timeout - server may be unreachable or slow");
                }
                
                Err(Box::new(e) as Box<dyn Error>)
            }
        }
    }
    
    async fn execute_query(&self, query: &str, limit: i32) -> Result<Value, Box<dyn Error>> {
        let pool = self.create_pool().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
        
        let query_with_limit = if query.to_lowercase().contains("limit") {
            query.to_string()
        } else {
            format!("{} LIMIT {}", query, limit)
        };
        
        let start = std::time::Instant::now();
        let rows = sqlx::query(&query_with_limit).fetch_all(&pool).await?;
        let execution_time_ms = start.elapsed().as_millis() as i64;
        
        if rows.is_empty() {
            pool.close().await;
            return Ok(json!({
                "columns": [],
                "rows": [],
                "row_count": 0,
                "execution_time_ms": execution_time_ms
            }));
        }
        
        // Get column names from the first row
        let first_row = &rows[0];
        let columns: Vec<String> = first_row.columns().iter().map(|c| c.name().to_string()).collect();
        
        // Convert rows to JSON
        let mut result_rows = Vec::new();
        for row in rows.iter() {
            let mut row_data = Vec::new();
            for (i, _col) in columns.iter().enumerate() {
                // Try to get value as different types
                if let Ok(val) = row.try_get::<String, _>(i) {
                    row_data.push(val);
                } else if let Ok(val) = row.try_get::<i32, _>(i) {
                    row_data.push(val.to_string());
                } else if let Ok(val) = row.try_get::<i64, _>(i) {
                    row_data.push(val.to_string());
                } else if let Ok(val) = row.try_get::<f64, _>(i) {
                    row_data.push(val.to_string());
                } else if let Ok(val) = row.try_get::<f32, _>(i) {
                    row_data.push(val.to_string());
                } else if let Ok(val) = row.try_get::<bool, _>(i) {
                    row_data.push(if val { "1" } else { "0" }.to_string());
                } else if let Ok(val) = row.try_get::<chrono::NaiveDateTime, _>(i) {
                    row_data.push(val.to_string());
                } else if let Ok(val) = row.try_get::<chrono::NaiveDate, _>(i) {
                    row_data.push(val.to_string());
                } else {
                    row_data.push("NULL".to_string());
                }
            }
            result_rows.push(row_data);
        }
        
        let result = json!({
            "columns": columns,
            "rows": result_rows,
            "row_count": result_rows.len(),
            "execution_time_ms": execution_time_ms
        });
        
        pool.close().await;
        Ok(result)
    }
    
    async fn fetch_schema(&self) -> Result<Value, Box<dyn Error>> {
        let pool = self.create_pool().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
        
        let tables = sqlx::query(
            "SELECT 
                TABLE_NAME as table_name,
                COLUMN_NAME as column_name,
                DATA_TYPE as data_type,
                IS_NULLABLE as is_nullable
             FROM INFORMATION_SCHEMA.COLUMNS
             WHERE TABLE_SCHEMA = DATABASE()
             ORDER BY TABLE_NAME, ORDINAL_POSITION"
        )
        .fetch_all(&pool)
        .await?;
        
        let mut schema = json!({
            "tables": {},
            "refreshed_at": chrono::Utc::now().to_rfc3339()
        });
        
        for row in tables {
            let table_name: String = row.get("table_name");
            let column_info = json!({
                "column_name": row.get::<String, _>("column_name"),
                "data_type": row.get::<String, _>("data_type"),
                "is_nullable": row.get::<String, _>("is_nullable"),
            });
            
            if schema["tables"].get(&table_name).is_none() {
                schema["tables"][&table_name] = json!([]);
            }
            schema["tables"][&table_name].as_array_mut().unwrap().push(column_info);
        }
        
        pool.close().await;
        Ok(schema)
    }
    
    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let pool = self.create_pool().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
        
        let tables = sqlx::query("SHOW TABLES")
            .fetch_all(&pool)
            .await?;
        
        let table_names: Vec<String> = tables.iter().map(|row| row.get(0)).collect();
        pool.close().await;
        Ok(table_names)
    }
    
    async fn analyze_database(&self) -> Result<Value, Box<dyn Error>> {
        let pool = self.create_pool().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
        
        // Get basic statistics
        let stats = sqlx::query(
            "SELECT 
                COUNT(*) as table_count,
                SUM(DATA_LENGTH + INDEX_LENGTH) as total_size
             FROM INFORMATION_SCHEMA.TABLES
             WHERE TABLE_SCHEMA = DATABASE()"
        )
        .fetch_one(&pool)
        .await?;
        
        let table_count: i64 = stats.get("table_count");
        let total_size: Option<i64> = stats.get("total_size");
        
        // Get detailed table information
        let tables = sqlx::query(
            "SELECT 
                TABLE_NAME as table_name,
                TABLE_ROWS as row_count,
                DATA_LENGTH + INDEX_LENGTH as size_bytes,
                TABLE_COMMENT as comment
             FROM INFORMATION_SCHEMA.TABLES
             WHERE TABLE_SCHEMA = DATABASE()
             ORDER BY DATA_LENGTH + INDEX_LENGTH DESC"
        )
        .fetch_all(&pool)
        .await?;
        
        // Get foreign key relationships
        let relationships = sqlx::query(
            "SELECT 
                TABLE_NAME as table_name,
                COLUMN_NAME as column_name,
                REFERENCED_TABLE_NAME as foreign_table_name,
                REFERENCED_COLUMN_NAME as foreign_column_name
             FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE
             WHERE TABLE_SCHEMA = DATABASE()
                 AND REFERENCED_TABLE_NAME IS NOT NULL"
        )
        .fetch_all(&pool)
        .await?;
        
        // Build the analysis result
        let mut key_tables = Vec::new();
        let mut largest_tables = Vec::new();
        let mut table_names = Vec::new();
        
        for (idx, table) in tables.iter().enumerate() {
            let table_name: String = table.get("table_name");
            let size_bytes: Option<i64> = table.get("size_bytes");
            let row_count: Option<i64> = table.get("row_count");
            
            table_names.push(table_name.clone());
            
            // Track largest tables
            if idx < 10 {
                largest_tables.push(json!({
                    "name": table_name,
                    "size_bytes": size_bytes.unwrap_or(0),
                    "size_human": super::base::format_bytes(size_bytes.unwrap_or(0) as u64),
                    "row_count": row_count.unwrap_or(0),
                }));
            }
            
            // Count relationships for this table
            let connections = relationships.iter()
                .filter(|r| {
                    let t: String = r.get("table_name");
                    let ft: Option<String> = r.get("foreign_table_name");
                    t == table_name || (ft.is_some() && ft.unwrap() == table_name)
                })
                .count();
            
            // Add to key tables if it has many connections or is large
            if connections > 2 || idx < 5 {
                key_tables.push(json!({
                    "name": table_name,
                    "size_bytes": size_bytes.unwrap_or(0),
                    "row_count": row_count.unwrap_or(0),
                    "connections": connections,
                }));
            }
        }
        
        // Sort key tables by importance
        key_tables.sort_by_key(|t| {
            let connections = t["connections"].as_u64().unwrap_or(0);
            let size = t["size_bytes"].as_i64().unwrap_or(0);
            -(connections as i64 * 1000 + size / 1000000)
        });
        
        let result = json!({
            "statistics": {
                "table_count": table_count,
                "total_size": total_size.unwrap_or(0),
                "total_size_human": super::base::format_bytes(total_size.unwrap_or(0) as u64),
            },
            "table_names": table_names,
            "key_tables": key_tables.into_iter().take(10).collect::<Vec<_>>(),
            "largest_tables": largest_tables,
            "analyzed_at": chrono::Utc::now().to_rfc3339(),
        });
        
        pool.close().await;
        Ok(result)
    }
    
    async fn get_tables_schema(&self, tables: Vec<&str>) -> Result<Value, Box<dyn Error>> {
        let pool = self.create_pool().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
        let mut result = json!({});
        
        for table_name in tables {
            // Get columns
            let columns = sqlx::query(
                "SELECT 
                    COLUMN_NAME as column_name,
                    DATA_TYPE as data_type,
                    IS_NULLABLE as is_nullable,
                    COLUMN_DEFAULT as column_default,
                    CHARACTER_MAXIMUM_LENGTH as max_length,
                    COLUMN_KEY as column_key,
                    EXTRA as extra
                 FROM INFORMATION_SCHEMA.COLUMNS
                 WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ?
                 ORDER BY ORDINAL_POSITION"
            )
            .bind(table_name)
            .fetch_all(&pool)
            .await?;
            
            if columns.is_empty() {
                continue; // Table doesn't exist
            }
            
            // Get primary keys
            let primary_keys = sqlx::query(
                "SELECT COLUMN_NAME as column_name
                 FROM INFORMATION_SCHEMA.COLUMNS
                 WHERE TABLE_SCHEMA = DATABASE()
                     AND TABLE_NAME = ?
                     AND COLUMN_KEY = 'PRI'"
            )
            .bind(table_name)
            .fetch_all(&pool)
            .await?;
            
            // Get foreign keys
            let foreign_keys = sqlx::query(
                "SELECT 
                    COLUMN_NAME as column_name,
                    REFERENCED_TABLE_NAME as foreign_table_name,
                    REFERENCED_COLUMN_NAME as foreign_column_name
                 FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE
                 WHERE TABLE_SCHEMA = DATABASE()
                     AND TABLE_NAME = ?
                     AND REFERENCED_TABLE_NAME IS NOT NULL"
            )
            .bind(table_name)
            .fetch_all(&pool)
            .await?;
            
            // Get row count
            let count_result = sqlx::query(
                "SELECT TABLE_ROWS as row_count
                 FROM INFORMATION_SCHEMA.TABLES
                 WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ?"
            )
            .bind(table_name)
            .fetch_one(&pool)
            .await?;
            let row_count: Option<i64> = count_result.get("row_count");
            
            // Get sample data
            let sample_query = format!("SELECT * FROM `{}` LIMIT 5", table_name.replace('`', "``"));
            let sample_rows = sqlx::query(&sample_query)
                .fetch_all(&pool)
                .await
                .unwrap_or_default();
            
            let mut sample_data = Vec::new();
            for row in sample_rows {
                let mut row_data = json!({});
                for (i, col) in columns.iter().enumerate() {
                    let col_name: String = col.get("column_name");
                    // Try to get value as different types
                    if let Ok(val) = row.try_get::<Option<String>, _>(i) {
                        row_data[&col_name] = json!(val);
                    } else if let Ok(val) = row.try_get::<Option<i32>, _>(i) {
                        row_data[&col_name] = json!(val);
                    } else if let Ok(val) = row.try_get::<Option<i64>, _>(i) {
                        row_data[&col_name] = json!(val);
                    } else if let Ok(val) = row.try_get::<Option<f64>, _>(i) {
                        row_data[&col_name] = json!(val);
                    } else if let Ok(val) = row.try_get::<Option<bool>, _>(i) {
                        row_data[&col_name] = json!(val);
                    } else {
                        row_data[&col_name] = json!(null);
                    }
                }
                sample_data.push(row_data);
            }
            
            result[table_name] = json!({
                "columns": columns.iter().map(|c| json!({
                    "name": c.get::<String, _>("column_name"),
                    "type": c.get::<String, _>("data_type"),
                    "nullable": c.get::<String, _>("is_nullable") == "YES",
                    "default": c.get::<Option<String>, _>("column_default"),
                    "max_length": c.get::<Option<i64>, _>("max_length"),
                    "key": c.get::<String, _>("column_key"),
                    "extra": c.get::<String, _>("extra"),
                })).collect::<Vec<_>>(),
                "primary_keys": primary_keys.iter().map(|pk| 
                    pk.get::<String, _>("column_name")
                ).collect::<Vec<_>>(),
                "foreign_keys": foreign_keys.iter().map(|fk| json!({
                    "column": fk.get::<String, _>("column_name"),
                    "references_table": fk.get::<String, _>("foreign_table_name"),
                    "references_column": fk.get::<String, _>("foreign_column_name"),
                })).collect::<Vec<_>>(),
                "row_count": row_count.unwrap_or(0),
                "sample_data": sample_data,
            });
        }
        
        pool.close().await;
        Ok(result)
    }
    
    async fn search_tables(&self, pattern: &str) -> Result<Value, Box<dyn Error>> {
        let pool = self.create_pool().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
        
        let tables = sqlx::query(
            "SELECT 
                t.TABLE_NAME as table_name,
                t.TABLE_COMMENT as comment,
                COUNT(c.COLUMN_NAME) as column_count
             FROM INFORMATION_SCHEMA.TABLES t
             LEFT JOIN INFORMATION_SCHEMA.COLUMNS c 
                 ON t.TABLE_NAME = c.TABLE_NAME 
                 AND t.TABLE_SCHEMA = c.TABLE_SCHEMA
             WHERE t.TABLE_SCHEMA = DATABASE()
                 AND t.TABLE_NAME LIKE ?
             GROUP BY t.TABLE_NAME, t.TABLE_COMMENT"
        )
        .bind(pattern)
        .fetch_all(&pool)
        .await?;
        
        let results: Vec<Value> = tables.iter().map(|row| {
            json!({
                "name": row.get::<String, _>("table_name"),
                "description": row.get::<Option<String>, _>("comment"),
                "column_count": row.get::<i64, _>("column_count"),
            })
        }).collect();
        
        let result = json!({
            "matches": results,
            "total_matches": results.len(),
        });
        
        pool.close().await;
        Ok(result)
    }
    
    async fn get_related_tables(&self, table: &str) -> Result<Value, Box<dyn Error>> {
        let pool = self.create_pool().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
        
        // Get the main table schema
        let main_schema = self.get_tables_schema(vec![table]).await?;
        
        if main_schema.get(table).is_none() {
            return Err("Table not found".into());
        }
        
        // Get tables that this table references (outgoing foreign keys)
        let references = sqlx::query(
            "SELECT DISTINCT REFERENCED_TABLE_NAME as foreign_table_name
             FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE
             WHERE TABLE_SCHEMA = DATABASE()
                 AND TABLE_NAME = ?
                 AND REFERENCED_TABLE_NAME IS NOT NULL"
        )
        .bind(table)
        .fetch_all(&pool)
        .await?;
        
        // Get tables that reference this table (incoming foreign keys)
        let referenced_by = sqlx::query(
            "SELECT DISTINCT TABLE_NAME as table_name
             FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE
             WHERE TABLE_SCHEMA = DATABASE()
                 AND REFERENCED_TABLE_NAME = ?"
        )
        .bind(table)
        .fetch_all(&pool)
        .await?;
        
        // Collect all related table names
        let mut related_tables = Vec::new();
        for row in references {
            let table_name: String = row.get("foreign_table_name");
            related_tables.push(table_name);
        }
        for row in referenced_by {
            let table_name: String = row.get("table_name");
            if !related_tables.contains(&table_name) {
                related_tables.push(table_name);
            }
        }
        
        // Get schemas for related tables
        let related_schemas = if !related_tables.is_empty() {
            let refs: Vec<&str> = related_tables.iter().map(|s| s.as_str()).collect();
            self.get_tables_schema(refs).await?
        } else {
            json!({})
        };
        
        let result = json!({
            "main_table": main_schema[table],
            "related_tables": related_schemas,
            "relationship_count": related_tables.len(),
        });
        
        pool.close().await;
        Ok(result)
    }
    
    async fn get_database_stats(&self) -> Result<Value, Box<dyn Error>> {
        let pool = self.create_pool().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
        
        // Get overall statistics
        let stats = sqlx::query(
            "SELECT 
                COUNT(*) as table_count,
                SUM(DATA_LENGTH + INDEX_LENGTH) as total_size,
                SUM(TABLE_ROWS) as total_rows
             FROM INFORMATION_SCHEMA.TABLES
             WHERE TABLE_SCHEMA = DATABASE()"
        )
        .fetch_one(&pool)
        .await?;
        
        // Get largest tables
        let largest_tables = sqlx::query(
            "SELECT 
                TABLE_NAME as table_name,
                TABLE_ROWS as row_count,
                DATA_LENGTH + INDEX_LENGTH as size_bytes,
                ROUND((DATA_LENGTH + INDEX_LENGTH) / 1024 / 1024, 2) as size_mb
             FROM INFORMATION_SCHEMA.TABLES
             WHERE TABLE_SCHEMA = DATABASE()
             ORDER BY DATA_LENGTH + INDEX_LENGTH DESC
             LIMIT 10"
        )
        .fetch_all(&pool)
        .await?;
        
        // Get most connected tables
        let connections = sqlx::query(
            "WITH foreign_keys AS (
                SELECT 
                    TABLE_NAME as table_name,
                    COUNT(*) as outgoing_fks
                FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE
                WHERE TABLE_SCHEMA = DATABASE()
                    AND REFERENCED_TABLE_NAME IS NOT NULL
                GROUP BY TABLE_NAME
            ),
            referenced AS (
                SELECT 
                    REFERENCED_TABLE_NAME as table_name,
                    COUNT(*) as incoming_fks
                FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE
                WHERE TABLE_SCHEMA = DATABASE()
                    AND REFERENCED_TABLE_NAME IS NOT NULL
                GROUP BY REFERENCED_TABLE_NAME
            )
            SELECT 
                t.TABLE_NAME as table_name,
                COALESCE(f.outgoing_fks, 0) as references_count,
                COALESCE(r.incoming_fks, 0) as referenced_by_count,
                COALESCE(f.outgoing_fks, 0) + COALESCE(r.incoming_fks, 0) as total_connections
            FROM INFORMATION_SCHEMA.TABLES t
            LEFT JOIN foreign_keys f ON t.TABLE_NAME = f.table_name
            LEFT JOIN referenced r ON t.TABLE_NAME = r.table_name
            WHERE t.TABLE_SCHEMA = DATABASE()
                AND (f.outgoing_fks > 0 OR r.incoming_fks > 0)
            ORDER BY total_connections DESC
            LIMIT 10"
        )
        .fetch_all(&pool)
        .await?;
        
        let result = json!({
            "summary": {
                "total_tables": stats.get::<i64, _>("table_count"),
                "total_size_bytes": stats.get::<Option<i64>, _>("total_size").unwrap_or(0),
                "total_size_human": super::base::format_bytes(stats.get::<Option<i64>, _>("total_size").unwrap_or(0) as u64),
                "total_rows": stats.get::<Option<i64>, _>("total_rows").unwrap_or(0),
            },
            "largest_tables": largest_tables.iter().map(|t| json!({
                "name": t.get::<String, _>("table_name"),
                "row_count": t.get::<Option<i64>, _>("row_count").unwrap_or(0),
                "size_bytes": t.get::<Option<i64>, _>("size_bytes").unwrap_or(0),
                "size_human": format!("{:.2} MB", t.get::<Option<f64>, _>("size_mb").unwrap_or(0.0)),
            })).collect::<Vec<_>>(),
            "most_connected_tables": connections.iter().map(|c| json!({
                "name": c.get::<String, _>("table_name"),
                "references_count": c.get::<i64, _>("references_count"),
                "referenced_by_count": c.get::<i64, _>("referenced_by_count"),
                "total_connections": c.get::<i64, _>("total_connections"),
            })).collect::<Vec<_>>(),
        });
        
        pool.close().await;
        Ok(result)
    }
}