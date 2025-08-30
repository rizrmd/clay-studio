use super::base::DataSourceConnector;
use serde_json::{json, Value};
use sqlx::{mysql::{MySqlPool, MySqlPoolOptions}, Row as SqlxRow, Column};
use std::error::Error;
use async_trait::async_trait;
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn, error, debug};

pub struct MySQLConnector {
    connection_string: String,
    original_connection_string: String,
    ssl_mode_used: Option<String>,
    pool: Arc<Mutex<Option<MySqlPool>>>,
}

impl MySQLConnector {
    async fn get_pool(&self) -> Result<MySqlPool, sqlx::Error> {
        let mut pool_guard = self.pool.lock().await;
        
        if let Some(ref pool) = *pool_guard {
            // Test if the pool is still valid
            if sqlx::query("SELECT 1 as test").fetch_one(pool).await.is_ok() {
                return Ok(pool.clone());
            }
            // Pool is invalid, will create a new one below
        }
        
        // Create new pool
        let pool_options = MySqlPoolOptions::new()
            .max_connections(5)
            .min_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Some(Duration::from_secs(30)));
        
        let pool = pool_options.connect(&self.connection_string).await?;
        *pool_guard = Some(pool.clone());
        Ok(pool)
    }
    
    pub fn new(config: &Value) -> Result<Self, Box<dyn Error>> {
        // Check for SSL/TLS settings
        let disable_ssl = config.get("disable_ssl").and_then(|v| v.as_bool()).unwrap_or(false);
        let ssl_mode = config.get("ssl_mode").and_then(|v| v.as_str());
        
        // Prefer URL if provided, otherwise construct from individual components
        let mut connection_string = if let Some(url) = config.get("url").and_then(|v| v.as_str()) {
            debug!("🔗 Using provided MySQL URL directly");
            url.to_string()
        } else {
            // Fallback to individual components for backward compatibility
            let host = config.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
            let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(3306);
            let database = config.get("database").and_then(|v| v.as_str()).ok_or("Missing database name")?;
            let username = config.get("username").and_then(|v| v.as_str()).unwrap_or("root");
            let password = config.get("password").and_then(|v| v.as_str()).unwrap_or("");
            
            info!("🔧 Building MySQL URL from components: host={}, port={}, database={}, username={}", 
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
        
        // Handle SSL/TLS configuration
        if disable_ssl || ssl_mode == Some("disabled") {
            // Add SSL disabled parameter if not already present
            if !connection_string.contains("sslmode=") && !connection_string.contains("ssl-mode=") {
                let separator = if connection_string.contains('?') { "&" } else { "?" };
                connection_string.push_str(&format!("{}sslmode=disabled", separator));
                debug!("🔒 SSL disabled for MySQL connection");
            }
        } else if let Some(mode) = ssl_mode {
            // Add specific SSL mode if provided
            if !connection_string.contains("sslmode=") && !connection_string.contains("ssl-mode=") {
                let separator = if connection_string.contains('?') { "&" } else { "?" };
                connection_string.push_str(&format!("{}sslmode={}", separator, mode));
                info!("🔒 SSL mode set to: {}", mode);
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
        info!("🔗 MySQL connection string (masked): {}", masked_conn_str);
        
        Ok(Self { 
            original_connection_string: connection_string.clone(),
            connection_string,
            ssl_mode_used: ssl_mode.map(|s| s.to_string()).or(if disable_ssl { Some("disabled".to_string()) } else { None }),
            pool: Arc::new(Mutex::new(None)),
        })
    }
}

#[async_trait]
impl DataSourceConnector for MySQLConnector {
    async fn test_connection(&mut self) -> Result<bool, Box<dyn Error>> {
        // Try connection with current settings first
        let mut connection_strings_to_try = vec![self.connection_string.clone()];
        
        // If SSL mode is not explicitly set, prepare fallback options
        if self.ssl_mode_used.is_none() {
            // Try with SSL disabled as fallback
            let with_ssl_disabled = if self.original_connection_string.contains("sslmode=") {
                self.original_connection_string.clone()
            } else {
                let separator = if self.original_connection_string.contains('?') { "&" } else { "?" };
                format!("{}{}sslmode=disabled", self.original_connection_string, separator)
            };
            
            if with_ssl_disabled != self.connection_string {
                connection_strings_to_try.push(with_ssl_disabled);
            }
        }
        
        let mut last_error: Option<String> = None;
        let mut attempt = 0;
        
        for conn_str in connection_strings_to_try {
            attempt += 1;
            let ssl_status = if conn_str.contains("sslmode=disabled") {
                "with SSL disabled"
            } else {
                "with SSL enabled (default)"
            };
            
            info!("🔄 MySQL connection attempt {} {}", attempt, ssl_status);
            
            // Create a temporary connector with this connection string
            let temp_self = Self {
                connection_string: conn_str.clone(),
                original_connection_string: self.original_connection_string.clone(),
                ssl_mode_used: self.ssl_mode_used.clone(),
                pool: Arc::new(Mutex::new(None)),
            };
            
            // Try to connect
            match temp_self.get_pool().await {
            Ok(pool) => {
                // Try a simple query
                match sqlx::query("SELECT 1 as test").fetch_one(&pool).await {
                    Ok(_) => {
                        info!("✅ MySQL connection successful {}", ssl_status);
                        
                        // Save the working connection string
                        self.connection_string = conn_str.clone();
                        if conn_str.contains("sslmode=disabled") {
                            self.ssl_mode_used = Some("disabled".to_string());
                            info!("💾 Saved working configuration with SSL disabled");
                        } else {
                            info!("💾 Saved working configuration with SSL enabled");
                        }
                        
                        return Ok(true);
                    }
                    Err(e) => {
                        last_error = Some(e.to_string());
                    }
                }
            }
            Err(e) => {
                last_error = Some(e.to_string());
            }
        }
    }
        
        // All attempts failed, show detailed error diagnostics
        if let Some(e) = last_error {
            error!("💥 ========== MySQL Connection Failed After All Attempts ==========");
            error!("❌ Failed to create MySQL connection pool");
            error!("📄 Error message: {}", e);
            
            // Log connection parameters (without password) for debugging
            let masked_conn_str = if self.connection_string.contains('@') {
                let parts: Vec<&str> = self.connection_string.splitn(2, '@').collect();
                if parts.len() == 2 {
                    if let Some(protocol_end) = parts[0].find("://") {
                        let protocol = &parts[0][..protocol_end+3];
                        let auth = &parts[0][protocol_end+3..];
                        if auth.contains(':') {
                            let user_parts: Vec<&str> = auth.splitn(2, ':').collect();
                            format!("{}{}:****@{}", protocol, user_parts[0], parts[1])
                        } else {
                            self.connection_string.clone()
                        }
                    } else {
                        "mysql://****".to_string()
                    }
                } else {
                    self.connection_string.clone()
                }
            } else {
                self.connection_string.clone()
            };
            debug!("🔗 Connection string (masked): {}", masked_conn_str);
            
            // Extract host and port for better diagnostics
            if let Some(at_pos) = self.connection_string.find('@') {
                if let Some(host_start) = self.connection_string.get(at_pos + 1..) {
                    let host_part = host_start.split('/').next().unwrap_or("");
                    debug!("🎯 Target host/port: {}", host_part);
                }
            }
            
            // Check if it's a specific type of error
            let error_string = e.to_string();
            debug!("🔍 Analyzing error type...");
            
            if error_string.contains("Access denied") {
                error!("🚨 Authentication failed - check username and password");
                warn!("💡 Common causes:");
                warn!("  - Incorrect password");
                warn!("  - User doesn't exist");
                warn!("  - User exists but lacks privileges for the database");
            } else if error_string.contains("Unknown database") {
                error!("🚨 Database does not exist");
                warn!("💡 Create the database first with: CREATE DATABASE <dbname>");
            } else if error_string.contains("Can't connect") || error_string.contains("Connection refused") {
                error!("🚨 Cannot reach MySQL server - check host and port");
                warn!("💡 Common causes:");
                warn!("  - MySQL server is not running");
                warn!("  - Incorrect host or port");
                warn!("  - Firewall blocking the connection");
                warn!("  - MySQL not configured to accept network connections");
            } else if error_string.contains("timeout") {
                error!("🚨 Connection timeout - server may be unreachable or slow");
                warn!("💡 Common causes:");
                warn!("  - Network issues");
                warn!("  - Server is overloaded");
                warn!("  - Incorrect host/port causing connection to hang");
            } else if error_string.contains("SSL") || error_string.contains("TLS") || error_string.contains("2026") {
                error!("🚨 SSL/TLS connection issue");
                warn!("💡 The server requires SSL but may not support it properly");
                info!("📝 Already tried connecting with and without SSL");
            } else if error_string.contains("Host") && error_string.contains("is not allowed") {
                error!("🚨 Host not allowed to connect");
                warn!("💡 Grant access from your host: GRANT ALL ON *.* TO 'user'@'your-host'");
            } else {
                error!("🚨 Unrecognized error type");
                warn!("💡 Check the full error message above for more details");
            }
            
            error!("💥 ========== MySQL Connection Test Failed ==========");
            Err(e.into())
        } else {
            Err("No connection attempts were made".into())
        }
    }
    
    async fn execute_query(&self, query: &str, limit: i32) -> Result<Value, Box<dyn Error>> {
        let pool = self.get_pool().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
        
        let query_with_limit = if query.to_lowercase().contains("limit") {
            query.to_string()
        } else {
            format!("{} LIMIT {}", query, limit)
        };
        
        let start = std::time::Instant::now();
        let rows = sqlx::query(&query_with_limit).fetch_all(&pool).await?;
        let execution_time_ms = start.elapsed().as_millis() as i64;
        
        if rows.is_empty() {
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
        
        Ok(result)
    }
    
    async fn fetch_schema(&self) -> Result<Value, Box<dyn Error>> {
        let pool = self.get_pool().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
        
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
            let table_name: String = row.try_get("table_name")
                .map_err(|e| format!("Failed to get table_name: {}", e))?;
            let column_info = json!({
                "column_name": row.try_get::<String, _>("column_name")
                    .map_err(|e| format!("Failed to get column_name: {}", e))?,
                "data_type": row.try_get::<String, _>("data_type")
                    .map_err(|e| format!("Failed to get data_type: {}", e))?,
                "is_nullable": row.try_get::<String, _>("is_nullable")
                    .map_err(|e| format!("Failed to get is_nullable: {}", e))?,
            });
            
            if schema["tables"].get(&table_name).is_none() {
                schema["tables"][&table_name] = json!([]);
            }
            if let Some(array) = schema["tables"][&table_name].as_array_mut() {
                array.push(column_info);
            } else {
                return Err(format!("Failed to get array for table {}", table_name).into());
            }
        }
        
        Ok(schema)
    }
    
    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let pool = self.get_pool().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
        
        let tables = sqlx::query("SHOW TABLES")
            .fetch_all(&pool)
            .await?;
        
        let table_names: Vec<String> = tables.iter()
            .map(|row| row.try_get::<String, _>(0))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to get table names: {}", e))?;
        Ok(table_names)
    }
    
    async fn analyze_database(&self) -> Result<Value, Box<dyn Error>> {
        let pool = self.get_pool().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
        
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
        
        let table_count: i64 = stats.try_get("table_count")
            .map_err(|e| format!("Failed to get table_count: {}", e))?;
        let total_size: Option<i64> = stats.try_get::<Option<String>, _>("total_size")
            .ok()
            .flatten()
            .and_then(|s| s.parse::<i64>().ok())
            .or_else(|| stats.try_get::<Option<i64>, _>("total_size").ok().flatten());
        
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
            let table_name: String = table.try_get("table_name")
                .map_err(|e| format!("Failed to get table_name: {}", e))?;
            let size_bytes: Option<i64> = table.try_get("size_bytes").ok();
            let row_count: Option<i64> = table.try_get("row_count").ok();
            
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
                    if let (Ok(t), Ok(ft)) = (r.try_get::<String, _>("table_name"), r.try_get::<Option<String>, _>("foreign_table_name")) {
                        t == table_name || (ft.is_some() && ft.unwrap() == table_name)
                    } else {
                        false
                    }
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
        
        Ok(result)
    }
    
    async fn get_tables_schema(&self, tables: Vec<&str>) -> Result<Value, Box<dyn Error>> {
        let pool = self.get_pool().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
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
            let row_count: Option<i64> = count_result.try_get::<Option<u64>, _>("row_count")
                .ok()
                .flatten()
                .map(|u| u as i64)
                .or_else(|| count_result.try_get::<Option<i64>, _>("row_count").ok().flatten());
            
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
                    let col_name: String = col.try_get("column_name")
                        .map_err(|e| format!("Failed to get column_name: {}", e))?;
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
            
            let column_details: Result<Vec<_>, Box<dyn Error>> = columns.iter().map(|c| {
                Ok(json!({
                    "name": c.try_get::<String, _>("column_name")
                        .map_err(|e| format!("Failed to get column_name: {}", e))?,
                    "type": c.try_get::<String, _>("data_type")
                        .map_err(|e| format!("Failed to get data_type: {}", e))?,
                    "nullable": c.try_get::<String, _>("is_nullable")
                        .map_err(|e| format!("Failed to get is_nullable: {}", e))? == "YES",
                    "default": c.try_get::<Option<String>, _>("column_default").ok().flatten(),
                    "max_length": c.try_get::<Option<i64>, _>("max_length").ok().flatten(),
                    "key": c.try_get::<String, _>("column_key").ok().unwrap_or_default(),
                    "extra": c.try_get::<String, _>("extra").ok().unwrap_or_default(),
                }))
            }).collect();
            
            let pk_list: Result<Vec<_>, Box<dyn Error>> = primary_keys.iter().map(|pk| {
                pk.try_get::<String, _>("column_name")
                    .map_err(|e| Box::new(e) as Box<dyn Error>)
            }).collect();
            
            let fk_list: Result<Vec<_>, Box<dyn Error>> = foreign_keys.iter().map(|fk| {
                Ok(json!({
                    "column": fk.try_get::<String, _>("column_name")
                        .map_err(|e| format!("Failed to get column_name: {}", e))?,
                    "references_table": fk.try_get::<String, _>("foreign_table_name")
                        .map_err(|e| format!("Failed to get foreign_table_name: {}", e))?,
                    "references_column": fk.try_get::<String, _>("foreign_column_name")
                        .map_err(|e| format!("Failed to get foreign_column_name: {}", e))?,
                }))
            }).collect();
            
            result[table_name] = json!({
                "columns": column_details?,
                "primary_keys": pk_list?,
                "foreign_keys": fk_list?,
                "row_count": row_count.unwrap_or(0),
                "sample_data": sample_data,
            });
        }
        
        Ok(result)
    }
    
    async fn search_tables(&self, pattern: &str) -> Result<Value, Box<dyn Error>> {
        let pool = self.get_pool().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
        
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
        
        let results: Result<Vec<Value>, Box<dyn Error>> = tables.iter().map(|row| {
            Ok(json!({
                "name": row.try_get::<String, _>("table_name")
                    .map_err(|e| format!("Failed to get table_name: {}", e))?,
                "description": row.try_get::<Option<String>, _>("comment").ok().flatten().unwrap_or_default(),
                "column_count": row.try_get::<i64, _>("column_count")
                    .map_err(|e| format!("Failed to get column_count: {}", e))?,
            }))
        }).collect();
        
        let results = results?;
        
        let result = json!({
            "matches": results,
            "total_matches": results.len(),
        });
        
        Ok(result)
    }
    
    async fn get_related_tables(&self, table: &str) -> Result<Value, Box<dyn Error>> {
        let pool = self.get_pool().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
        
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
            let table_name: String = row.try_get("foreign_table_name")
                .map_err(|e| format!("Failed to get foreign_table_name: {}", e))?;
            related_tables.push(table_name);
        }
        for row in referenced_by {
            let table_name: String = row.try_get("table_name")
                .map_err(|e| format!("Failed to get table_name: {}", e))?;
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
        
        Ok(result)
    }
    
    async fn get_database_stats(&self) -> Result<Value, Box<dyn Error>> {
        let pool = self.get_pool().await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
        
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
        
        Ok(result)
    }
}