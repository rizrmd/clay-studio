use super::base::{DataSourceConnector, format_bytes};
use serde_json::{json, Value};
use sqlx::{postgres::PgPool, Row as SqlxRow, Column};
use std::error::Error;
use async_trait::async_trait;

pub struct PostgreSQLConnector {
    connection_string: String,
    original_connection_string: String,
    schema: String,
    ssl_mode_used: Option<String>,
}

impl PostgreSQLConnector {
    pub fn new(config: &Value) -> Result<Self, Box<dyn Error>> {
        // Get the schema name from config, default to 'public' if not specified
        let schema = config.get("schema")
            .and_then(|v| v.as_str())
            .unwrap_or("public")
            .to_string();
        
        eprintln!("[DEBUG] PostgreSQL connector using schema: '{}'", schema);
        
        // Check for SSL/TLS settings
        let disable_ssl = config.get("disable_ssl").and_then(|v| v.as_bool()).unwrap_or(false);
        let ssl_mode = config.get("ssl_mode").and_then(|v| v.as_str());
        
        // Prefer URL if provided, otherwise construct from individual components
        let mut connection_string = if let Some(url) = config.get("url").and_then(|v| v.as_str()) {
            url.to_string()
        } else {
            // Fallback to individual components for backward compatibility
            let host = config.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
            let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(5432);
            let database = config.get("database").and_then(|v| v.as_str()).ok_or("Missing database name")?;
            let username = config.get("username").and_then(|v| v.as_str()).unwrap_or("postgres");
            let password = config.get("password").and_then(|v| v.as_str()).unwrap_or("");
            
            // URL encode username and password to handle special characters
            let encoded_username = urlencoding::encode(username);
            let encoded_password = if password.is_empty() {
                String::new()
            } else {
                urlencoding::encode(password).to_string()
            };
            
            if encoded_password.is_empty() {
                format!("postgres://{}@{}:{}/{}", encoded_username, host, port, database)
            } else {
                format!("postgres://{}:{}@{}:{}/{}", encoded_username, encoded_password, host, port, database)
            }
        };
        
        // Handle SSL/TLS configuration for PostgreSQL
        if disable_ssl || ssl_mode == Some("disable") {
            // Add SSL disabled parameter if not already present
            if !connection_string.contains("sslmode=") {
                let separator = if connection_string.contains('?') { "&" } else { "?" };
                connection_string.push_str(&format!("{}sslmode=disable", separator));
                eprintln!("[DEBUG] SSL disabled for PostgreSQL connection");
            }
        } else if let Some(mode) = ssl_mode {
            // Add specific SSL mode if provided (PostgreSQL modes: disable, allow, prefer, require, verify-ca, verify-full)
            if !connection_string.contains("sslmode=") {
                let separator = if connection_string.contains('?') { "&" } else { "?" };
                connection_string.push_str(&format!("{}sslmode={}", separator, mode));
                eprintln!("[DEBUG] PostgreSQL SSL mode set to: {}", mode);
            }
        };
        
        // Debug: Log the connection string (with password masked)
        let masked_string = if connection_string.contains('@') {
            let parts: Vec<&str> = connection_string.splitn(2, "://").collect();
            if parts.len() == 2 {
                let auth_and_rest: Vec<&str> = parts[1].splitn(2, '@').collect();
                if auth_and_rest.len() == 2 {
                    let auth_parts: Vec<&str> = auth_and_rest[0].splitn(2, ':').collect();
                    if auth_parts.len() == 2 {
                        format!("{}://{}:***@{}", parts[0], auth_parts[0], auth_and_rest[1])
                    } else {
                        format!("{}://{}@{}", parts[0], auth_parts[0], auth_and_rest[1])
                    }
                } else {
                    connection_string.clone()
                }
            } else {
                connection_string.clone()
            }
        } else {
            connection_string.clone()
        };
        eprintln!("[DEBUG] PostgreSQL connection string (masked): {}", masked_string);
        eprintln!("[DEBUG] Raw connection string length: {} chars", connection_string.len());
        
        Ok(Self { 
            original_connection_string: connection_string.clone(),
            connection_string, 
            schema,
            ssl_mode_used: ssl_mode.map(|s| s.to_string()).or(if disable_ssl { Some("disable".to_string()) } else { None })
        })
    }
}

#[async_trait]
impl DataSourceConnector for PostgreSQLConnector {
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
                format!("{}{}sslmode=disable", self.original_connection_string, separator)
            };
            
            if with_ssl_disabled != self.connection_string {
                connection_strings_to_try.push(with_ssl_disabled);
            }
        }
        
        let mut last_error: Option<Box<dyn Error + Send + Sync>> = None;
        let mut attempt = 0;
        
        for conn_str in connection_strings_to_try {
            attempt += 1;
            let ssl_status = if conn_str.contains("sslmode=disable") {
                "with SSL disabled"
            } else {
                "with SSL enabled (default)"
            };
            
            eprintln!("[INFO] PostgreSQL connection attempt {} {}", attempt, ssl_status);
            
            // Try to connect
            match PgPool::connect(&conn_str).await {
                Ok(pool) => {
                    // Try a simple query
                    match sqlx::query("SELECT 1").fetch_one(&pool).await {
                        Ok(_) => {
                            eprintln!("[SUCCESS] PostgreSQL connection successful {}", ssl_status);
                            
                            // Save the working connection string
                            self.connection_string = conn_str.clone();
                            if conn_str.contains("sslmode=disable") {
                                self.ssl_mode_used = Some("disable".to_string());
                                eprintln!("[INFO] Saved working configuration with SSL disabled");
                            } else {
                                eprintln!("[INFO] Saved working configuration with SSL enabled");
                            }
                            
                            pool.close().await;
                            return Ok(true);
                        }
                        Err(e) => {
                            pool.close().await;
                            last_error = Some(Box::new(e) as Box<dyn Error + Send + Sync>);
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(Box::new(e) as Box<dyn Error + Send + Sync>);
                }
            }
        }
        
        // All attempts failed, show detailed error diagnostics
        if let Some(e) = last_error {
            eprintln!("[ERROR] ========== PostgreSQL Connection Failed After All Attempts ==========");
            eprintln!("[ERROR] Failed to create PostgreSQL connection pool");
            eprintln!("[ERROR] Error message: {}", e);
            eprintln!("[ERROR] Error type: {:?}", e);
            eprintln!("[ERROR] Error source chain:");
            
            // Print full error chain
            let mut current_error = &*e as &dyn std::error::Error;
            let mut level = 1;
            while let Some(source) = current_error.source() {
                eprintln!("[ERROR]   Level {}: {}", level, source);
                current_error = source;
                level += 1;
            }
            
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
                        "postgres://****".to_string()
                    }
                } else {
                    self.connection_string.clone()
                }
            } else {
                self.connection_string.clone()
            };
            eprintln!("[DEBUG] Connection string (masked): {}", masked_conn_str);
            
            // Extract host and port for better diagnostics
            if let Some(at_pos) = self.connection_string.find('@') {
                if let Some(host_start) = self.connection_string.get(at_pos + 1..) {
                    let host_part = host_start.split('/').next().unwrap_or("");
                    eprintln!("[DEBUG] Target host/port: {}", host_part);
                }
            }
            
            // Check if it's a specific type of error
            let error_string = e.to_string();
            eprintln!("[DEBUG] Analyzing error type...");
            
            if error_string.contains("password authentication failed") || error_string.contains("FATAL: password") {
                eprintln!("[DIAGNOSIS] Authentication failed - check username and password");
                eprintln!("[HINT] Common causes:");
                eprintln!("  - Incorrect password");
                eprintln!("  - User doesn't exist");
                eprintln!("  - pg_hba.conf not configured for password authentication");
            } else if error_string.contains("database") && error_string.contains("does not exist") {
                eprintln!("[DIAGNOSIS] Database does not exist");
                eprintln!("[HINT] Create the database first with: CREATE DATABASE <dbname>");
            } else if error_string.contains("Connection refused") || error_string.contains("could not connect") {
                eprintln!("[DIAGNOSIS] Cannot reach PostgreSQL server - check host and port");
                eprintln!("[HINT] Common causes:");
                eprintln!("  - PostgreSQL server is not running");
                eprintln!("  - Incorrect host or port");
                eprintln!("  - PostgreSQL not configured to accept network connections");
                eprintln!("  - Check postgresql.conf for listen_addresses setting");
            } else if error_string.contains("timeout") {
                eprintln!("[DIAGNOSIS] Connection timeout - server may be unreachable or slow");
                eprintln!("[HINT] Common causes:");
                eprintln!("  - Network issues");
                eprintln!("  - Server is overloaded");
                eprintln!("  - Firewall blocking the connection");
            } else if error_string.contains("SSL") || error_string.contains("TLS") || error_string.contains("SSLMODE") {
                eprintln!("[DIAGNOSIS] SSL/TLS connection issue");
                eprintln!("[HINT] The server may require or reject SSL connections");
                eprintln!("[SOLUTION] Add one of these to your datasource config:");
                eprintln!("  - \"disable_ssl\": true (to disable SSL)");
                eprintln!("  - \"ssl_mode\": \"disable\" (to disable SSL)");
                eprintln!("  - \"ssl_mode\": \"require\" (to require SSL)");
                eprintln!("  - Or append ?sslmode=disable to your connection URL");
                eprintln!("[EXAMPLE] postgres://user:pass@host:port/db?sslmode=disable");
            } else if error_string.contains("pg_hba.conf") || error_string.contains("no pg_hba.conf entry") {
                eprintln!("[DIAGNOSIS] Host-based authentication configuration issue");
                eprintln!("[HINT] The server's pg_hba.conf doesn't allow connections from your host");
                eprintln!("[SOLUTION] Update pg_hba.conf to allow connections from your IP/network");
            } else if error_string.contains("too many connections") {
                eprintln!("[DIAGNOSIS] Server has reached maximum connection limit");
                eprintln!("[HINT] Wait and retry, or increase max_connections in postgresql.conf");
            } else {
                eprintln!("[DIAGNOSIS] Unrecognized error type");
                eprintln!("[HINT] Check the full error message above for more details");
            }
            
            eprintln!("[DEBUG] ========== PostgreSQL Connection Test Failed ==========");
            Err(e)
        } else {
            Err("No connection attempts were made".into())
        }
    }
    
    async fn execute_query(&self, query: &str, limit: i32) -> Result<Value, Box<dyn Error>> {
        let pool = PgPool::connect(&self.connection_string).await?;
        
        // Add LIMIT if not present
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
                } else if let Ok(val) = row.try_get::<bool, _>(i) {
                    row_data.push(val.to_string());
                } else {
                    row_data.push("NULL".to_string());
                }
            }
            result_rows.push(row_data);
        }
        
        Ok(json!({
            "columns": columns,
            "rows": result_rows,
            "row_count": result_rows.len(),
            "execution_time_ms": execution_time_ms
        }))
    }
    
    async fn fetch_schema(&self) -> Result<Value, Box<dyn Error>> {
        let pool = PgPool::connect(&self.connection_string).await?;
        
        // Fetch table and column information
        // Use json_agg instead of array_agg to return JSONB type
        let tables = sqlx::query(
            "SELECT 
                t.table_name,
                json_agg(
                    json_build_object(
                        'column_name', c.column_name,
                        'data_type', c.data_type,
                        'is_nullable', c.is_nullable
                    ) ORDER BY c.ordinal_position
                ) as columns
             FROM information_schema.tables t
             JOIN information_schema.columns c ON t.table_name = c.table_name AND t.table_schema = c.table_schema
             WHERE t.table_schema = $1 
             AND t.table_type = 'BASE TABLE'
             GROUP BY t.table_name
             ORDER BY t.table_name"
        )
        .bind(&self.schema)
        .fetch_all(&pool)
        .await?;
        
        let mut schema = json!({
            "database_schema": &self.schema,
            "tables": {},
            "refreshed_at": chrono::Utc::now().to_rfc3339()
        });
        
        for row in tables {
            let table_name: String = row.try_get("table_name")
                .map_err(|e| format!("Failed to get table_name: {}", e))?;
            let columns: Value = row.try_get("columns")
                .map_err(|e| format!("Failed to get columns for table '{}': {}. This may be due to a PostgreSQL type compatibility issue with JSON/JSONB columns.", table_name, e))?;
            schema["tables"][table_name] = columns;
        }
        
        Ok(schema)
    }
    
    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let pool = PgPool::connect(&self.connection_string).await?;
        
        let tables = sqlx::query(
            "SELECT table_name 
             FROM information_schema.tables 
             WHERE table_schema = $1 
             AND table_type = 'BASE TABLE'
             ORDER BY table_name"
        )
        .bind(&self.schema)
        .fetch_all(&pool)
        .await?;
        
        let mut table_names = Vec::new();
        for row in &tables {
            let table_name: String = row.try_get("table_name")
                .map_err(|e| format!("Failed to get table_name: {}", e))?;
            table_names.push(table_name);
        }
        Ok(table_names)
    }
    
    async fn analyze_database(&self) -> Result<Value, Box<dyn Error>> {
        let pool = PgPool::connect(&self.connection_string).await?;
        
        // Get basic statistics
        let stats = sqlx::query(
            "SELECT 
                COUNT(DISTINCT table_name) as table_count,
                SUM(pg_total_relation_size(quote_ident(table_schema)||'.'||quote_ident(table_name)::text)) as total_size
             FROM information_schema.tables
             WHERE table_schema = $1 AND table_type = 'BASE TABLE'"
        )
        .bind(&self.schema)
        .fetch_one(&pool)
        .await?;
        
        let table_count: i64 = stats.try_get("table_count")
            .map_err(|e| format!("Failed to get table_count: {}", e))?;
        let total_size: Option<i64> = match stats.try_get("total_size") {
            Ok(size) => size,
            Err(e) => {
                tracing::warn!("Failed to decode total_size column: {}", e);
                // Return user-friendly error instead of panicking
                return Err(format!("Database type compatibility issue: Unable to read table size statistics. This is likely due to a PostgreSQL version or configuration difference. Error: {}", e).into());
            }
        };
        
        // Get detailed table information with safer query
        let tables = sqlx::query(
            "SELECT 
                t.table_name,
                pg_total_relation_size(quote_ident(t.table_schema)||'.'||quote_ident(t.table_name)::text) as size_bytes,
                obj_description((quote_ident(t.table_schema)||'.'||quote_ident(t.table_name))::regclass) as description
             FROM information_schema.tables t
             WHERE t.table_schema = $1 AND t.table_type = 'BASE TABLE'
             ORDER BY pg_total_relation_size(quote_ident(t.table_schema)||'.'||quote_ident(t.table_name)::text) DESC NULLS LAST"
        )
        .bind(&self.schema)
        .fetch_all(&pool)
        .await?;
        
        // Get table relationships (foreign keys)
        let relationships = sqlx::query(
            "SELECT 
                tc.table_name,
                kcu.column_name,
                ccu.table_name AS foreign_table_name,
                ccu.column_name AS foreign_column_name
             FROM information_schema.table_constraints AS tc
             JOIN information_schema.key_column_usage AS kcu
                 ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema
             JOIN information_schema.constraint_column_usage AS ccu
                 ON ccu.constraint_name = tc.constraint_name AND ccu.table_schema = tc.table_schema
             WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_schema = $1"
        )
        .bind(&self.schema)
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
            
            table_names.push(table_name.clone());
            
            // Track largest tables
            if idx < 10 {
                if let Some(size) = size_bytes {
                    largest_tables.push(json!({
                        "name": table_name,
                        "size_bytes": size,
                        "size_human": format_bytes(size as u64),
                    }));
                }
            }
            
            // Count relationships for this table
            let connections = relationships.iter()
                .filter_map(|r| {
                    if let (Ok(t), Ok(ft)) = (r.try_get::<String, _>("table_name"), r.try_get::<String, _>("foreign_table_name")) {
                        Some((t, ft))
                    } else {
                        None
                    }
                })
                .filter(|(t, ft)| t == &table_name || ft == &table_name)
                .count();
            
            // Add to key tables if it has many connections or is large
            if connections > 2 || idx < 5 {
                key_tables.push(json!({
                    "name": table_name,
                    "size_bytes": size_bytes.unwrap_or(0),
                    "connections": connections,
                }));
            }
        }
        
        // Sort key tables by importance (connections + size)
        key_tables.sort_by_key(|t| {
            let connections = t["connections"].as_u64().unwrap_or(0);
            let size = t["size_bytes"].as_i64().unwrap_or(0);
            -(connections as i64 * 1000 + size / 1000000)
        });
        
        Ok(json!({
            "database_schema": &self.schema,
            "statistics": {
                "table_count": table_count,
                "total_size": total_size.unwrap_or(0),
                "total_size_human": format_bytes(total_size.unwrap_or(0) as u64),
                "total_rows": 0, // Would need to query each table
            },
            "table_names": table_names,
            "key_tables": key_tables.into_iter().take(10).collect::<Vec<_>>(),
            "largest_tables": largest_tables,
            "analyzed_at": chrono::Utc::now().to_rfc3339(),
        }))
    }
    
    async fn get_tables_schema(&self, tables: Vec<&str>) -> Result<Value, Box<dyn Error>> {
        let pool = PgPool::connect(&self.connection_string).await?;
        let mut result = json!({});
        
        for table_name in tables {
            // Get columns
            let columns = sqlx::query(
                "SELECT 
                    column_name,
                    data_type,
                    is_nullable,
                    column_default,
                    character_maximum_length
                 FROM information_schema.columns
                 WHERE table_schema = $1 AND table_name = $2
                 ORDER BY ordinal_position"
            )
            .bind(&self.schema)
            .bind(table_name)
            .fetch_all(&pool)
            .await?;
            
            if columns.is_empty() {
                continue; // Table doesn't exist, skip it
            }
            
            // Get primary keys
            let primary_keys = sqlx::query(
                "SELECT kcu.column_name
                 FROM information_schema.table_constraints tc
                 JOIN information_schema.key_column_usage kcu
                     ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema
                 WHERE tc.table_schema = $1 
                     AND tc.table_name = $2
                     AND tc.constraint_type = 'PRIMARY KEY'"
            )
            .bind(&self.schema)
            .bind(table_name)
            .fetch_all(&pool)
            .await?;
            
            // Get foreign keys
            let foreign_keys = sqlx::query(
                "SELECT 
                    kcu.column_name,
                    ccu.table_name AS foreign_table_name,
                    ccu.column_name AS foreign_column_name
                 FROM information_schema.table_constraints AS tc
                 JOIN information_schema.key_column_usage AS kcu
                     ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema
                 JOIN information_schema.constraint_column_usage AS ccu
                     ON ccu.constraint_name = tc.constraint_name AND ccu.table_schema = tc.table_schema
                 WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_schema = $1 AND tc.table_name = $2"
            )
            .bind(&self.schema)
            .bind(table_name)
            .fetch_all(&pool)
            .await?;
            
            // Get row count (safely with proper quoting)
            let count_query = format!(
                "SELECT COUNT(*) as count FROM {}.{}",
                quote_ident(&self.schema),
                quote_ident(table_name)
            );
            let row_count: i64 = sqlx::query(&count_query)
                .fetch_one(&pool)
                .await
                .map(|r| r.try_get("count").unwrap_or(0))
                .unwrap_or(0);
            
            // Get sample data (safely with proper quoting)
            let sample_query = format!(
                "SELECT * FROM {}.{} LIMIT 5",
                quote_ident(&self.schema),
                quote_ident(table_name)
            );
            let sample_rows = sqlx::query(&sample_query)
                .fetch_all(&pool)
                .await
                .unwrap_or_default();
            
            let mut sample_data = Vec::new();
            for row in sample_rows {
                let mut row_data = json!({});
                for col in columns.iter() {
                    let col_name: String = col.try_get("column_name")
                        .map_err(|e| format!("Failed to get column_name: {}", e))?;
                    // Try to get value as string (simplified)
                    if let Ok(val) = row.try_get::<Option<String>, _>(col_name.as_str()) {
                        row_data[&col_name] = json!(val);
                    } else if let Ok(val) = row.try_get::<Option<i32>, _>(col_name.as_str()) {
                        row_data[&col_name] = json!(val);
                    } else if let Ok(val) = row.try_get::<Option<i64>, _>(col_name.as_str()) {
                        row_data[&col_name] = json!(val);
                    } else if let Ok(val) = row.try_get::<Option<bool>, _>(col_name.as_str()) {
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
                    "max_length": c.get::<Option<i32>, _>("character_maximum_length"),
                })).collect::<Vec<_>>(),
                "primary_keys": primary_keys.iter().map(|pk| 
                    pk.get::<String, _>("column_name")
                ).collect::<Vec<_>>(),
                "foreign_keys": foreign_keys.iter().map(|fk| json!({
                    "column": fk.get::<String, _>("column_name"),
                    "references_table": fk.get::<String, _>("foreign_table_name"),
                    "references_column": fk.get::<String, _>("foreign_column_name"),
                })).collect::<Vec<_>>(),
                "row_count": row_count,
                "sample_data": sample_data,
            });
        }
        
        Ok(result)
    }
    
    async fn search_tables(&self, pattern: &str) -> Result<Value, Box<dyn Error>> {
        let pool = PgPool::connect(&self.connection_string).await?;
        
        let tables = sqlx::query(
            "SELECT 
                table_name,
                obj_description((quote_ident(table_schema)||'.'||quote_ident(table_name))::regclass) as description
             FROM information_schema.tables
             WHERE table_schema = $1 
                 AND table_type = 'BASE TABLE'
                 AND table_name LIKE $2
             ORDER BY table_name"
        )
        .bind(&self.schema)
        .bind(pattern)
        .fetch_all(&pool)
        .await?;
        
        let mut results = Vec::new();
        for table in tables {
            let table_name: String = table.try_get("table_name")
                .map_err(|e| format!("Failed to get table_name: {}", e))?;
            let description: Option<String> = table.try_get("description").ok();
            
            // Get column count
            let col_count: i64 = sqlx::query(
                "SELECT COUNT(*) as count FROM information_schema.columns 
                 WHERE table_schema = $1 AND table_name = $2"
            )
            .bind(&self.schema)
            .bind(&table_name)
            .fetch_one(&pool)
            .await
            .map(|r| r.try_get("count").unwrap_or(0))
            .unwrap_or(0);
            
            results.push(json!({
                "name": table_name,
                "description": description,
                "column_count": col_count,
            }));
        }
        
        Ok(json!({
            "matches": results,
            "total_matches": results.len(),
        }))
    }
    
    async fn get_related_tables(&self, table: &str) -> Result<Value, Box<dyn Error>> {
        // Get the main table schema
        let main_schema = self.get_tables_schema(vec![table]).await?;
        
        if main_schema.get(table).is_none() {
            return Err("Table not found".into());
        }
        
        let pool = PgPool::connect(&self.connection_string).await?;
        
        // Get tables that this table references (outgoing foreign keys)
        let references = sqlx::query(
            "SELECT DISTINCT ccu.table_name AS foreign_table_name
             FROM information_schema.table_constraints AS tc
             JOIN information_schema.key_column_usage AS kcu
                 ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema
             JOIN information_schema.constraint_column_usage AS ccu
                 ON ccu.constraint_name = tc.constraint_name AND ccu.table_schema = tc.table_schema
             WHERE tc.constraint_type = 'FOREIGN KEY' 
                 AND tc.table_name = $2
                 AND tc.table_schema = $1"
        )
        .bind(&self.schema)
        .bind(table)
        .fetch_all(&pool)
        .await?;
        
        // Get tables that reference this table (incoming foreign keys)
        let referenced_by = sqlx::query(
            "SELECT DISTINCT tc.table_name
             FROM information_schema.table_constraints AS tc
             JOIN information_schema.key_column_usage AS kcu
                 ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema
             JOIN information_schema.constraint_column_usage AS ccu
                 ON ccu.constraint_name = tc.constraint_name AND ccu.table_schema = tc.table_schema
             WHERE tc.constraint_type = 'FOREIGN KEY' 
                 AND ccu.table_name = $2
                 AND tc.table_schema = $1"
        )
        .bind(&self.schema)
        .bind(table)
        .fetch_all(&pool)
        .await?;
        
        // Collect all related table names
        let mut related_tables = Vec::new();
        for row in references {
            related_tables.push(row.get::<String, _>("foreign_table_name"));
        }
        for row in referenced_by {
            related_tables.push(row.get::<String, _>("table_name"));
        }
        
        // Get schemas for related tables
        let related_schemas = if !related_tables.is_empty() {
            let refs: Vec<&str> = related_tables.iter().map(|s| s.as_str()).collect();
            self.get_tables_schema(refs).await?
        } else {
            json!({})
        };
        
        Ok(json!({
            "main_table": main_schema[table],
            "related_tables": related_schemas,
            "relationship_count": related_tables.len(),
        }))
    }
    
    async fn get_database_stats(&self) -> Result<Value, Box<dyn Error>> {
        let pool = PgPool::connect(&self.connection_string).await?;
        
        // Get overall statistics
        let stats = sqlx::query(
            "SELECT 
                COUNT(DISTINCT table_name) as table_count,
                COALESCE(SUM(pg_total_relation_size(quote_ident(table_schema)||'.'||quote_ident(table_name)::text))::bigint, 0) as total_size
             FROM information_schema.tables
             WHERE table_schema = $1 AND table_type = 'BASE TABLE'"
        )
        .bind(&self.schema)
        .fetch_one(&pool)
        .await?;
        
        // Get largest tables
        let largest_tables = sqlx::query(
            "SELECT 
                table_name,
                COALESCE(pg_total_relation_size(quote_ident(table_schema)||'.'||quote_ident(table_name)::text)::bigint, 0) as size_bytes,
                pg_size_pretty(pg_total_relation_size(quote_ident(table_schema)||'.'||quote_ident(table_name)::text)) as size_human
             FROM information_schema.tables
             WHERE table_schema = $1 AND table_type = 'BASE TABLE'
             ORDER BY pg_total_relation_size(quote_ident(table_schema)||'.'||quote_ident(table_name)::text) DESC NULLS LAST
             LIMIT 10"
        )
        .bind(&self.schema)
        .fetch_all(&pool)
        .await?;
        
        // Get most connected tables
        let connections = sqlx::query(
            "WITH foreign_keys AS (
                SELECT 
                    tc.table_name,
                    COUNT(*) as outgoing_fks
                FROM information_schema.table_constraints AS tc
                WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_schema = $1
                GROUP BY tc.table_name
            ),
            referenced AS (
                SELECT 
                    ccu.table_name,
                    COUNT(*) as incoming_fks
                FROM information_schema.table_constraints AS tc
                JOIN information_schema.constraint_column_usage AS ccu
                    ON ccu.constraint_name = tc.constraint_name AND ccu.table_schema = tc.table_schema
                WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_schema = $1
                GROUP BY ccu.table_name
            )
            SELECT 
                t.table_name,
                COALESCE(f.outgoing_fks, 0) as references_count,
                COALESCE(r.incoming_fks, 0) as referenced_by_count,
                COALESCE(f.outgoing_fks, 0) + COALESCE(r.incoming_fks, 0) as total_connections
            FROM information_schema.tables t
            LEFT JOIN foreign_keys f ON t.table_name = f.table_name
            LEFT JOIN referenced r ON t.table_name = r.table_name
            WHERE t.table_schema = $1 AND t.table_type = 'BASE TABLE'
                AND (f.outgoing_fks > 0 OR r.incoming_fks > 0)
            ORDER BY total_connections DESC
            LIMIT 10"
        )
        .bind(&self.schema)
        .fetch_all(&pool)
        .await?;
        
        Ok(json!({
            "summary": {
                "total_tables": stats.get::<i64, _>("table_count"),
                "total_size_bytes": stats.get::<i64, _>("total_size"),
                "total_size_human": format_bytes(stats.get::<i64, _>("total_size") as u64),
            },
            "largest_tables": largest_tables.iter().map(|t| json!({
                "name": t.get::<String, _>("table_name"),
                "size_bytes": t.get::<i64, _>("size_bytes"),
                "size_human": t.get::<String, _>("size_human"),
            })).collect::<Vec<_>>(),
            "most_connected_tables": connections.iter().map(|c| json!({
                "name": c.get::<String, _>("table_name"),
                "references_count": c.get::<i64, _>("references_count"),
                "referenced_by_count": c.get::<i64, _>("referenced_by_count"),
                "total_connections": c.get::<i64, _>("total_connections"),
            })).collect::<Vec<_>>(),
        }))
    }
}

// Helper function to quote identifiers safely
fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}