use super::base::{DataSourceConnector, format_bytes};
use serde_json::{json, Value};
use tiberius::{Client, Config, AuthMethod, EncryptionLevel};
use tokio::net::TcpStream;
use tokio_util::compat::{TokioAsyncWriteCompatExt, Compat};
use std::error::Error;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{info, error, debug};

pub struct SqlServerConnector {
    config: Config,
    server: String,
    database: Option<String>,
    schema: String,
    #[allow(dead_code)]
    original_connection_string: Option<String>,
    ssl_mode_used: Option<String>,
    connection_cache: Arc<Mutex<Option<CachedConnection>>>,
}

struct CachedConnection {
    client: Client<Compat<TcpStream>>,
    created_at: Instant,
    max_lifetime: Duration,
}

impl CachedConnection {
    fn new(client: Client<Compat<TcpStream>>) -> Self {
        Self {
            client,
            created_at: Instant::now(),
            max_lifetime: Duration::from_secs(300), // 5 minutes
        }
    }
    
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.max_lifetime
    }
    
    async fn is_healthy(&mut self) -> bool {
        // Test connection with a simple query
        self.client.query("SELECT 1", &[]).await.is_ok()
    }
}

impl SqlServerConnector {
    pub fn new(config: &Value) -> Result<Self, Box<dyn Error>> {
        // Get the schema name from config, default to 'dbo' if not specified
        let schema = config.get("schema")
            .and_then(|v| v.as_str())
            .unwrap_or("dbo")
            .to_string();
        
        debug!("SQL Server connector using schema: '{}'", schema);
        
        // Check if URL is provided first
        let (host, port, database, username, password) = if let Some(url) = config.get("url").and_then(|v| v.as_str()) {
            // Parse SQL Server connection URL
            Self::parse_connection_url(url)?
        } else {
            // Get connection parameters
            let host = config.get("host").and_then(|v| v.as_str()).unwrap_or("localhost").to_string();
            let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(1433) as u16;
            let database = config.get("database").and_then(|v| v.as_str()).map(|s| s.to_string());
            let username = config.get("username").and_then(|v| v.as_str()).unwrap_or("sa").to_string();
            let password = config.get("password").and_then(|v| v.as_str()).unwrap_or("").to_string();
            (host, port, database, username, password)
        };
        
        // Create Tiberius config
        let mut tiberius_config = Config::new();
        
        // Set authentication method (SQL Server authentication)
        tiberius_config.authentication(AuthMethod::sql_server(&username, &password));
        
        // Set database if provided
        if let Some(ref db) = database {
            tiberius_config.database(db);
        }
        
        // Handle SSL/TLS configuration
        let disable_ssl = config.get("disable_ssl").and_then(|v| v.as_bool()).unwrap_or(false);
        let ssl_mode = config.get("ssl_mode").and_then(|v| v.as_str());
        
        // Set trust settings
        let trust_cert = config.get("trust_server_certificate").and_then(|v| v.as_bool()).unwrap_or(false);
        if trust_cert {
            tiberius_config.trust_cert();
        }
        
        // Set encryption level based on SSL settings
        let encrypt = config.get("encrypt").and_then(|v| v.as_bool()).unwrap_or(false);
        let ssl_mode_used = if disable_ssl || ssl_mode == Some("disable") {
            tiberius_config.encryption(EncryptionLevel::Off);
            debug!("SSL disabled for SQL Server connection");
            Some("disable".to_string())
        } else if !encrypt {
            tiberius_config.encryption(EncryptionLevel::Off);
            None
        } else {
            ssl_mode.map(|s| s.to_string())
        };
        
        let server = format!("{}:{}", host, port);
        
        // Create connection string for debugging (with password masked)
        let connection_string = if password.is_empty() {
            format!("sqlserver://{}@{}:{}/{:?}", username, host, port, database)
        } else {
            format!("sqlserver://{}:***@{}:{}/{:?}", username, host, port, database)
        };
        
        debug!("SQL Server connector configured for: {}", server);
        debug!("Connection string (masked): {}", connection_string);
        debug!("Using database: {:?}", database);
        debug!("Using schema: {}", schema);
        debug!("Trust Server Certificate: {}", trust_cert);
        debug!("Encryption: {}", encrypt);
        
        Ok(Self {
            config: tiberius_config,
            server,
            database,
            schema,
            original_connection_string: Some(connection_string),
            ssl_mode_used,
            connection_cache: Arc::new(Mutex::new(None)),
        })
    }
    
    fn parse_connection_url(url: &str) -> Result<(String, u16, Option<String>, String, String), Box<dyn Error>> {
        // Parse SQL Server URL format: sqlserver://[user[:password]@]host[:port][/database][?params]
        // or mssql://[user[:password]@]host[:port][/database][?params]
        
        let url = if url.starts_with("sqlserver://") {
            &url[12..]
        } else if url.starts_with("mssql://") {
            &url[8..]
        } else {
            return Err("Invalid SQL Server URL format. Must start with sqlserver:// or mssql://".into());
        };
        
        // Split by @ to separate auth from host
        let parts: Vec<&str> = url.splitn(2, '@').collect();
        let (auth, host_and_rest) = if parts.len() == 2 {
            (Some(parts[0]), parts[1])
        } else {
            (None, parts[0])
        };
        
        // Parse auth
        let (username, password) = if let Some(auth_str) = auth {
            let auth_parts: Vec<&str> = auth_str.splitn(2, ':').collect();
            if auth_parts.len() == 2 {
                (auth_parts[0].to_string(), auth_parts[1].to_string())
            } else {
                (auth_parts[0].to_string(), String::new())
            }
        } else {
            ("sa".to_string(), String::new())
        };
        
        // Parse host, port, and database
        let (host_port, database) = if let Some(slash_pos) = host_and_rest.find('/') {
            let host_port = &host_and_rest[..slash_pos];
            let rest = &host_and_rest[slash_pos + 1..];
            let database = if let Some(query_pos) = rest.find('?') {
                &rest[..query_pos]
            } else {
                rest
            };
            (host_port, if database.is_empty() { None } else { Some(database.to_string()) })
        } else if let Some(query_pos) = host_and_rest.find('?') {
            (&host_and_rest[..query_pos], None)
        } else {
            (host_and_rest, None)
        };
        
        // Parse host and port
        let (host, port) = if let Some(colon_pos) = host_port.rfind(':') {
            let host = &host_port[..colon_pos];
            let port = host_port[colon_pos + 1..].parse::<u16>().unwrap_or(1433);
            (host.to_string(), port)
        } else {
            (host_port.to_string(), 1433)
        };
        
        Ok((host, port, database, username, password))
    }
    
    async fn get_connection(&self) -> Result<Client<Compat<TcpStream>>, Box<dyn Error>> {
        let mut cache = self.connection_cache.lock().await;
        
        // Check if we have a cached connection that's still valid
        if let Some(cached) = cache.as_mut() {
            if !cached.is_expired() && cached.is_healthy().await {
                // Move the client out and return it (this is a workaround since Client doesn't implement Clone)
                // We'll create a new connection instead of trying to reuse
                // This approach trades some performance for simplicity and safety
            }
        }
        
        // Create new connection
        let client = self.create_new_connection().await?;
        
        // Cache the connection (note: this is simplified - in a real pool we'd handle this better)
        *cache = Some(CachedConnection::new(client));
        
        // Since we can't clone the client, we'll create a new one for the caller
        // This is not ideal but works with Tiberius limitations
        self.create_new_connection().await
    }
    
    async fn create_new_connection(&self) -> Result<Client<Compat<TcpStream>>, Box<dyn Error>> {
        // Connect with 3-second timeout
        let tcp = tokio::time::timeout(
            Duration::from_secs(3),
            TcpStream::connect(&self.server)
        ).await
        .map_err(|_| Box::<dyn Error>::from("Connection timeout after 3 seconds"))??;
        tcp.set_nodelay(true)?;
        
        let mut client = Client::connect(self.config.clone(), tcp.compat_write()).await?;
        
        // Enable ANSI warnings
        client.execute("SET ANSI_WARNINGS ON", &[]).await?;
        
        Ok(client)
    }
}

#[async_trait]
impl DataSourceConnector for SqlServerConnector {
    async fn test_connection(&mut self) -> Result<bool, Box<dyn Error>> {
        debug!("SQL Server Connection Test Started");
        debug!("Attempting SQL Server connection to: {}", self.server);
        
        // Try connection with current settings first
        let mut attempts = vec![(self.config.clone(), self.ssl_mode_used.clone())];
        
        // If SSL mode is not explicitly set, prepare fallback options
        if self.ssl_mode_used.is_none() {
            // Try with SSL disabled as fallback
            let mut fallback_config = self.config.clone();
            fallback_config.encryption(EncryptionLevel::Off);
            fallback_config.trust_cert();
            attempts.push((fallback_config, Some("disable".to_string())));
        }
        
        let mut last_error: Option<Box<dyn Error + Send + Sync>> = None;
        let mut attempt_num = 0;
        
        for (config, ssl_mode) in attempts {
            attempt_num += 1;
            let ssl_status = if ssl_mode == Some("disable".to_string()) {
                "with SSL disabled"
            } else {
                "with SSL enabled (default)"
            };
            
            info!("SQL Server connection attempt {} {}", attempt_num, ssl_status);
            
            // Try to connect with 3-second timeout
            match tokio::time::timeout(
                Duration::from_secs(3),
                TcpStream::connect(&self.server)
            ).await {
                Ok(Ok(tcp)) => {
                    tcp.set_nodelay(true).ok();
                    match Client::connect(config.clone(), tcp.compat_write()).await {
                        Ok(mut client) => {
                            // Enable ANSI warnings
                            if let Err(e) = client.execute("SET ANSI_WARNINGS ON", &[]).await {
                                last_error = Some(Box::new(e) as Box<dyn Error + Send + Sync>);
                                continue;
                            }
                            
                            // Try a simple query
                            debug!("Executing test query: SELECT 1 as test");
                            match client.query("SELECT 1 as test", &[]).await {
                                Ok(stream) => {
                                    match stream.into_results().await {
                                        Ok(_) => {
                                            info!("SQL Server connection successful {}", ssl_status);
                                            
                                            // Save the working configuration
                                            self.config = config;
                                            self.ssl_mode_used = ssl_mode.clone();
                                            if ssl_mode == Some("disable".to_string()) {
                                                info!("Saved working configuration with SSL disabled");
                                            } else {
                                                info!("Saved working configuration with SSL enabled");
                                            }
                                            
                                            debug!("SQL Server Connection Test Completed Successfully");
                                            return Ok(true);
                                        }
                                        Err(e) => {
                                            last_error = Some(Box::new(e) as Box<dyn Error + Send + Sync>);
                                        }
                                    }
                                }
                                Err(e) => {
                                    last_error = Some(Box::new(e) as Box<dyn Error + Send + Sync>);
                                }
                            }
                        }
                        Err(e) => {
                            last_error = Some(Box::new(e) as Box<dyn Error + Send + Sync>);
                        }
                    }
                }
                Ok(Err(e)) => {
                    last_error = Some(Box::new(e) as Box<dyn Error + Send + Sync>);
                    error!("TCP connection failed: {}", last_error.as_ref().unwrap());
                    continue;
                }
                Err(_) => {
                    last_error = Some(Box::<dyn Error + Send + Sync>::from("Connection timeout after 3 seconds"));
                    error!("Connection timeout after 3 seconds");
                    continue;
                }
            }
        }
        
        // All attempts failed, show detailed error diagnostics
        if let Some(e) = last_error {
            error!("SQL Server Connection Failed After All Attempts");
            error!("Failed to connect to SQL Server");
            error!("Error message: {}", e);
            error!("Error type: {:?}", e);
            error!("Error source chain:");
            
            // Print full error chain
            let mut current_error = &*e as &dyn std::error::Error;
            let mut level = 1;
            while let Some(source) = current_error.source() {
                error!("  Level {}: {}", level, source);
                current_error = source;
                level += 1;
            }
            
            let error_string = e.to_string();
            debug!("Analyzing error type...");
            
            if error_string.contains("Login failed") {
                error!("Authentication failed - check username and password");
                info!("Common causes:");
                info!("  - Incorrect password");
                info!("  - User doesn't exist");
                info!("  - SQL Server authentication not enabled");
            } else if error_string.contains("Cannot open database") {
                error!("Database does not exist or user lacks permission");
                info!("Create the database first or check user permissions");
            } else if error_string.contains("Connection refused") || error_string.contains("No connection") {
                error!("Cannot reach SQL Server - check host and port");
                info!("Common causes:");
                info!("  - SQL Server is not running");
                info!("  - Incorrect host or port");
                info!("  - Firewall blocking the connection");
                info!("  - SQL Server not configured for TCP/IP connections");
            } else if error_string.contains("timeout") {
                error!("Connection timeout - server may be unreachable");
                info!("Check network connectivity and server status");
            } else if error_string.contains("certificate") || error_string.contains("TLS") || error_string.contains("SSL") {
                error!("SSL/TLS certificate issue");
                info!("The server may require or reject SSL connections");
                info!("Add one of these to your datasource config:");
                info!("  - \"disable_ssl\": true (to disable SSL)");
                info!("  - \"ssl_mode\": \"disable\" (to disable SSL)");
                info!("  - \"trust_server_certificate\": true (to trust any certificate)");
            } else {
                error!("Unrecognized error type");
                info!("Check the full error message above for more details");
            }
            
            debug!("SQL Server Connection Test Failed");
            Err(e)
        } else {
            Err("No connection attempts were made".into())
        }
    }
    
    async fn execute_query(&self, query: &str, limit: i32) -> Result<Value, Box<dyn Error>> {
        let mut client = self.get_connection().await?;
        
        // SQL Server approach: Execute a USE statement with the database and then queries will use the user's default schema
        // Or we can try to use EXECUTE AS to switch schema context, but that requires permissions
        // The most reliable approach is to create the login/user with the correct default schema
        
        // For now, we'll add a helper query to check current schema context
        if self.schema != "dbo" {
            // Try to switch default schema context using dynamic SQL
            // This works if the user has appropriate permissions
            let schema_context = format!("EXEC('USE [{}]')", self.database.as_ref().unwrap_or(&"master".to_string()));
            match client.execute(&schema_context, &[]).await {
                Ok(_) => debug!("Set database context for schema operations"),
                Err(e) => debug!("Could not set database context: {}. Tables may need to be fully qualified.", e)
            }
        }
        
        // Add TOP clause if not present (SQL Server specific)
        let query_with_limit = if query.to_lowercase().contains("top ") || query.to_lowercase().contains("limit ") {
            query.to_string()
        } else {
            // SQL Server uses TOP instead of LIMIT
            if query.to_lowercase().starts_with("select ") {
                format!("SELECT TOP {} {}", limit, &query[7..])
            } else {
                query.to_string()
            }
        };
        
        let start = std::time::Instant::now();
        let stream = client.query(&query_with_limit, &[]).await?;
        let results = stream.into_results().await?;
        let execution_time_ms = start.elapsed().as_millis() as i64;
        
        if results.is_empty() || results[0].is_empty() {
            return Ok(json!({
                "columns": [],
                "rows": [],
                "row_count": 0,
                "execution_time_ms": execution_time_ms
            }));
        }
        
        let rows = &results[0];
        
        // Get column names
        let columns: Vec<String> = rows[0].columns().iter().map(|c| c.name().to_string()).collect();
        
        // Convert rows to JSON
        let mut result_rows = Vec::new();
        for row in rows {
            let mut row_data = Vec::new();
            for (i, _col) in columns.iter().enumerate() {
                // Try to get value as different types
                if let Some(val) = row.get::<&str, _>(i) {
                    row_data.push(val.to_string());
                } else if let Some(val) = row.get::<i32, _>(i) {
                    row_data.push(val.to_string());
                } else if let Some(val) = row.get::<i64, _>(i) {
                    row_data.push(val.to_string());
                } else if let Some(val) = row.get::<f64, _>(i) {
                    row_data.push(val.to_string());
                } else if let Some(val) = row.get::<f32, _>(i) {
                    row_data.push(val.to_string());
                } else if let Some(val) = row.get::<bool, _>(i) {
                    row_data.push(if val { "1" } else { "0" }.to_string());
                } else if let Some(val) = row.get::<chrono::NaiveDateTime, _>(i) {
                    row_data.push(val.to_string());
                } else if let Some(val) = row.get::<chrono::NaiveDate, _>(i) {
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
        let mut client = self.get_connection().await?;
        
        let query = format!("
            SELECT 
                t.TABLE_NAME as table_name,
                c.COLUMN_NAME as column_name,
                c.DATA_TYPE as data_type,
                c.IS_NULLABLE as is_nullable
            FROM INFORMATION_SCHEMA.TABLES t
            JOIN INFORMATION_SCHEMA.COLUMNS c 
                ON t.TABLE_NAME = c.TABLE_NAME 
                AND t.TABLE_SCHEMA = c.TABLE_SCHEMA
            WHERE t.TABLE_TYPE = 'BASE TABLE'
                AND t.TABLE_SCHEMA = '{}'
            ORDER BY t.TABLE_NAME, c.ORDINAL_POSITION
        ", self.schema.replace('\'', "''"));
        
        let stream = client.query(&query, &[]).await?;
        let results = stream.into_results().await?;
        
        let mut schema = json!({
            "database_schema": &self.schema,
            "tables": {},
            "refreshed_at": chrono::Utc::now().to_rfc3339()
        });
        
        if !results.is_empty() {
            for row in &results[0] {
                let table_name: &str = row.try_get::<&str, _>(0)
                    .map_err(|e| format!("Failed to get table_name: {}", e))?
                    .unwrap_or("");
                let column_info = json!({
                    "column_name": row.try_get::<&str, _>(1)
                        .map_err(|e| format!("Failed to get column_name: {}", e))?
                        .unwrap_or(""),
                    "data_type": row.try_get::<&str, _>(2)
                        .map_err(|e| format!("Failed to get data_type: {}", e))?
                        .unwrap_or(""),
                    "is_nullable": row.try_get::<&str, _>(3)
                        .map_err(|e| format!("Failed to get is_nullable: {}", e))?
                        .unwrap_or(""),
                });
                
                if schema["tables"].get(table_name).is_none() {
                    schema["tables"][table_name] = json!([]);
                }
                if let Some(array) = schema["tables"][table_name].as_array_mut() {
                    array.push(column_info);
                } else {
                    return Err(format!("Failed to get array for table {}", table_name).into());
                }
            }
        }
        
        Ok(schema)
    }
    
    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut client = self.get_connection().await?;
        
        let query = format!("
            SELECT TABLE_NAME 
            FROM INFORMATION_SCHEMA.TABLES 
            WHERE TABLE_TYPE = 'BASE TABLE' 
                AND TABLE_SCHEMA = '{}'
            ORDER BY TABLE_NAME
        ", self.schema.replace('\'', "''"));
        
        let stream = client.query(&query, &[]).await?;
        let results = stream.into_results().await?;
        
        let mut table_names = Vec::new();
        if !results.is_empty() {
            for row in &results[0] {
                if let Some(name) = row.get::<&str, _>(0) {
                    table_names.push(name.to_string());
                }
            }
        }
        
        Ok(table_names)
    }
    
    async fn analyze_database(&self) -> Result<Value, Box<dyn Error>> {
        let mut client = self.get_connection().await?;
        
        // Get basic statistics with schema filter
        let stats_query = format!("
            SELECT 
                COUNT(DISTINCT t.name) as table_count,
                SUM(p.rows) as total_rows,
                SUM(a.total_pages) * 8 * 1024 as total_size
            FROM sys.tables t
            INNER JOIN sys.schemas s ON t.schema_id = s.schema_id
            INNER JOIN sys.partitions p ON t.object_id = p.object_id
            INNER JOIN sys.allocation_units a ON p.partition_id = a.container_id
            WHERE p.index_id IN (0, 1)
                AND t.is_ms_shipped = 0
                AND s.name = '{}'
        ", self.schema.replace('\'', "''"));
        
        let stats_stream = client.query(&stats_query, &[]).await?;
        let stats_results = stats_stream.into_results().await?;
        
        let mut table_count: i64 = 0;
        let mut total_rows: i64 = 0;
        let mut total_size: i64 = 0;
        
        if !stats_results.is_empty() && !stats_results[0].is_empty() {
            let row = &stats_results[0][0];
            table_count = row.get::<i32, _>(0).unwrap_or(0) as i64;
            total_rows = row.get::<i64, _>(1).unwrap_or(0);
            total_size = row.get::<i64, _>(2).unwrap_or(0);
        }
        
        // Get detailed table information with foreign key counts
        let tables_query = format!("
            WITH FKCounts AS (
                SELECT 
                    tc.TABLE_NAME,
                    COUNT(DISTINCT tc2.CONSTRAINT_NAME) as outgoing_fks
                FROM INFORMATION_SCHEMA.TABLES tc
                LEFT JOIN INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc2
                    ON tc.TABLE_NAME = tc2.TABLE_NAME 
                    AND tc.TABLE_SCHEMA = tc2.TABLE_SCHEMA
                    AND tc2.CONSTRAINT_TYPE = 'FOREIGN KEY'
                WHERE tc.TABLE_SCHEMA = '{}'
                GROUP BY tc.TABLE_NAME
            ),
            RefCounts AS (
                SELECT 
                    ccu.TABLE_NAME,
                    COUNT(DISTINCT tc.CONSTRAINT_NAME) as incoming_fks
                FROM INFORMATION_SCHEMA.CONSTRAINT_COLUMN_USAGE ccu
                JOIN INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc
                    ON ccu.CONSTRAINT_NAME = tc.CONSTRAINT_NAME 
                    AND ccu.TABLE_SCHEMA = tc.TABLE_SCHEMA
                    AND tc.CONSTRAINT_TYPE = 'FOREIGN KEY'
                WHERE ccu.TABLE_SCHEMA = '{}'
                GROUP BY ccu.TABLE_NAME
            )
            SELECT TOP 50
                t.name as table_name,
                p.rows as row_count,
                SUM(a.total_pages) * 8 * 1024 as size_bytes,
                COALESCE(fk.outgoing_fks, 0) + COALESCE(rc.incoming_fks, 0) as connections
            FROM sys.tables t
            INNER JOIN sys.schemas s ON t.schema_id = s.schema_id
            INNER JOIN sys.partitions p ON t.object_id = p.object_id
            INNER JOIN sys.allocation_units a ON p.partition_id = a.container_id
            LEFT JOIN FKCounts fk ON t.name = fk.TABLE_NAME
            LEFT JOIN RefCounts rc ON t.name = rc.TABLE_NAME
            WHERE p.index_id IN (0, 1)
                AND t.is_ms_shipped = 0
                AND s.name = '{}'
            GROUP BY t.name, p.rows, fk.outgoing_fks, rc.incoming_fks
            ORDER BY SUM(a.total_pages) DESC
        ", self.schema.replace('\'', "''"), self.schema.replace('\'', "''"), self.schema.replace('\'', "''"));
        
        let tables_stream = client.query(&tables_query, &[]).await?;
        let tables_results = tables_stream.into_results().await?;
        
        let mut key_tables = Vec::new();
        let mut largest_tables = Vec::new();
        let mut table_names = Vec::new();
        
        if !tables_results.is_empty() {
            for (idx, row) in tables_results[0].iter().enumerate() {
                let table_name = row.get::<&str, _>(0).unwrap_or("").to_string();
                let _row_count = row.get::<i64, _>(1).unwrap_or(0);
                let size_bytes = row.get::<i64, _>(2).unwrap_or(0);
                let connections = row.get::<i32, _>(3).unwrap_or(0);
                
                table_names.push(table_name.clone());
                
                if idx < 10 {
                    largest_tables.push(json!({
                        "name": table_name.clone(),
                        "size_bytes": size_bytes,
                        "size_human": format_bytes(size_bytes as u64),
                    }));
                }
                
                // Add to key tables if it has many connections or is large
                if connections > 2 || idx < 5 {
                    key_tables.push(json!({
                        "name": table_name.clone(),
                        "size_bytes": size_bytes,
                        "connections": connections,
                    }));
                }
            }
        }
        
        // Sort key tables by importance (connections + size)
        key_tables.sort_by_key(|t| {
            let connections = t["connections"].as_i64().unwrap_or(0);
            let size = t["size_bytes"].as_i64().unwrap_or(0);
            -(connections * 1000 + size / 1000000)
        });
        
        Ok(json!({
            "database_schema": &self.schema,
            "statistics": {
                "table_count": table_count,
                "total_size": total_size,
                "total_size_human": format_bytes(total_size as u64),
                "total_rows": total_rows,
            },
            "table_names": table_names,
            "key_tables": key_tables.into_iter().take(10).collect::<Vec<_>>(),
            "largest_tables": largest_tables,
            "analyzed_at": chrono::Utc::now().to_rfc3339(),
        }))
    }
    
    async fn get_tables_schema(&self, tables: Vec<&str>) -> Result<Value, Box<dyn Error>> {
        let mut client = self.get_connection().await?;
        let mut result = json!({});
        
        for table_name in tables {
            // Get columns
            let columns_query = format!("
                SELECT 
                    COLUMN_NAME as column_name,
                    DATA_TYPE as data_type,
                    IS_NULLABLE as is_nullable,
                    COLUMN_DEFAULT as column_default,
                    CHARACTER_MAXIMUM_LENGTH as max_length
                FROM INFORMATION_SCHEMA.COLUMNS
                WHERE TABLE_NAME = '{}' AND TABLE_SCHEMA = '{}'
                ORDER BY ORDINAL_POSITION
            ", table_name.replace('\'', "''"), self.schema.replace('\'', "''"));
            
            let columns_stream = client.query(&columns_query, &[]).await?;
            let columns_results = columns_stream.into_results().await?;
            
            if columns_results.is_empty() || columns_results[0].is_empty() {
                continue; // Table doesn't exist
            }
            
            let mut columns_info = Vec::new();
            for row in &columns_results[0] {
                columns_info.push(json!({
                    "name": row.get::<&str, _>(0).unwrap_or(""),
                    "type": row.get::<&str, _>(1).unwrap_or(""),
                    "nullable": row.get::<&str, _>(2).unwrap_or("") == "YES",
                    "default": row.get::<&str, _>(3).map(|s| s.to_string()),
                    "max_length": row.get::<i32, _>(4),
                }));
            }
            
            // Get primary keys
            let pk_query = format!("
                SELECT kcu.COLUMN_NAME
                FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc
                JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu
                    ON tc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME 
                    AND tc.TABLE_SCHEMA = kcu.TABLE_SCHEMA
                WHERE tc.TABLE_NAME = '{}' 
                    AND tc.TABLE_SCHEMA = '{}'
                    AND tc.CONSTRAINT_TYPE = 'PRIMARY KEY'
            ", table_name.replace('\'', "''"), self.schema.replace('\'', "''"));
            
            let pk_stream = client.query(&pk_query, &[]).await?;
            let pk_results = pk_stream.into_results().await?;
            
            let mut primary_keys = Vec::new();
            if !pk_results.is_empty() {
                for row in &pk_results[0] {
                    if let Some(col_name) = row.get::<&str, _>(0) {
                        primary_keys.push(col_name.to_string());
                    }
                }
            }
            
            // Get row count using schema-qualified name
            let count_query = format!("
                SELECT COUNT(*) as row_count
                FROM [{}.{}]
            ", self.schema.replace(']', "]]"), table_name.replace(']', "]]"));
            
            let count_stream = client.query(&count_query, &[]).await?;
            let count_results = count_stream.into_results().await?;
            
            let mut row_count: i64 = 0;
            if !count_results.is_empty() && !count_results[0].is_empty() {
                row_count = count_results[0][0].get::<i32, _>(0).unwrap_or(0) as i64;
            }
            
            // Get sample data using schema-qualified name
            let sample_query = format!("SELECT TOP 5 * FROM [{}.{}]", 
                self.schema.replace(']', "]]"), table_name.replace(']', "]]"));
            let sample_stream = client.query(&sample_query, &[]).await?;
            let sample_results = sample_stream.into_results().await?;
            
            let mut sample_data = Vec::new();
            if !sample_results.is_empty() {
                for row in &sample_results[0] {
                    let mut row_data = json!({});
                    for (i, col) in columns_info.iter().enumerate() {
                        let col_name = col["name"].as_str().unwrap_or("");
                        // Try to get value as different types  
                        if let Some(val) = row.get::<&str, _>(i) {
                            row_data[col_name] = json!(val);
                        } else if let Some(val) = row.get::<i32, _>(i) {
                            row_data[col_name] = json!(val);
                        } else if let Some(val) = row.get::<i64, _>(i) {
                            row_data[col_name] = json!(val);
                        } else if let Some(val) = row.get::<f64, _>(i) {
                            row_data[col_name] = json!(val);
                        } else if let Some(val) = row.get::<bool, _>(i) {
                            row_data[col_name] = json!(val);
                        } else {
                            row_data[col_name] = json!(null);
                        }
                    }
                    sample_data.push(row_data);
                }
            }
            
            // Get foreign keys
            let fk_query = format!("
                SELECT 
                    kcu.COLUMN_NAME,
                    ccu.TABLE_NAME AS foreign_table_name,
                    ccu.COLUMN_NAME AS foreign_column_name
                FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS AS tc
                JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE AS kcu
                    ON tc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME 
                    AND tc.TABLE_SCHEMA = kcu.TABLE_SCHEMA
                JOIN INFORMATION_SCHEMA.CONSTRAINT_COLUMN_USAGE AS ccu
                    ON ccu.CONSTRAINT_NAME = tc.CONSTRAINT_NAME 
                    AND ccu.TABLE_SCHEMA = tc.TABLE_SCHEMA
                WHERE tc.CONSTRAINT_TYPE = 'FOREIGN KEY' 
                    AND tc.TABLE_NAME = '{}'
                    AND tc.TABLE_SCHEMA = '{}'
            ", table_name.replace('\'', "''"), self.schema.replace('\'', "''"));
            
            let fk_stream = client.query(&fk_query, &[]).await?;
            let fk_results = fk_stream.into_results().await?;
            
            let mut foreign_keys = Vec::new();
            if !fk_results.is_empty() {
                for row in &fk_results[0] {
                    foreign_keys.push(json!({
                        "column": row.get::<&str, _>(0).unwrap_or(""),
                        "references_table": row.get::<&str, _>(1).unwrap_or(""),
                        "references_column": row.get::<&str, _>(2).unwrap_or(""),
                    }));
                }
            }
            
            result[table_name] = json!({
                "columns": columns_info,
                "primary_keys": primary_keys,
                "foreign_keys": foreign_keys,
                "row_count": row_count,
                "sample_data": sample_data,
            });
        }
        
        Ok(result)
    }
    
    async fn search_tables(&self, pattern: &str) -> Result<Value, Box<dyn Error>> {
        let mut client = self.get_connection().await?;
        
        let query = format!("
            SELECT 
                t.TABLE_NAME as table_name,
                ep.value as description,
                COUNT(c.COLUMN_NAME) as column_count
            FROM INFORMATION_SCHEMA.TABLES t
            LEFT JOIN INFORMATION_SCHEMA.COLUMNS c
                ON t.TABLE_NAME = c.TABLE_NAME 
                AND t.TABLE_SCHEMA = c.TABLE_SCHEMA
            LEFT JOIN sys.extended_properties ep
                ON ep.major_id = OBJECT_ID(t.TABLE_SCHEMA + '.' + t.TABLE_NAME)
                AND ep.minor_id = 0
                AND ep.name = 'MS_Description'
            WHERE t.TABLE_TYPE = 'BASE TABLE' 
                AND t.TABLE_SCHEMA = '{}'
                AND t.TABLE_NAME LIKE '{}'
            GROUP BY t.TABLE_NAME, ep.value
            ORDER BY t.TABLE_NAME
        ", self.schema.replace('\'', "''"), pattern.replace('\'', "''"));
        
        let stream = client.query(&query, &[]).await?;
        let results = stream.into_results().await?;
        
        let mut matches = Vec::new();
        if !results.is_empty() {
            for row in &results[0] {
                matches.push(json!({
                    "name": row.get::<&str, _>(0).unwrap_or(""),
                    "description": row.get::<&str, _>(1).map(|s| s.to_string()),
                    "column_count": row.get::<i32, _>(2).unwrap_or(0),
                }));
            }
        }
        
        Ok(json!({
            "matches": matches,
            "total_matches": matches.len(),
        }))
    }
    
    async fn get_related_tables(&self, table: &str) -> Result<Value, Box<dyn Error>> {
        // Get the main table schema
        let main_schema = self.get_tables_schema(vec![table]).await?;
        
        if main_schema.get(table).is_none() {
            return Err("Table not found".into());
        }
        
        let mut client = self.get_connection().await?;
        
        // Get tables that this table references (outgoing foreign keys)
        let references_query = format!("
            SELECT DISTINCT ccu.TABLE_NAME AS foreign_table_name
            FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS AS tc
            JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE AS kcu
                ON tc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME 
                AND tc.TABLE_SCHEMA = kcu.TABLE_SCHEMA
            JOIN INFORMATION_SCHEMA.CONSTRAINT_COLUMN_USAGE AS ccu
                ON ccu.CONSTRAINT_NAME = tc.CONSTRAINT_NAME 
                AND ccu.TABLE_SCHEMA = tc.TABLE_SCHEMA
            WHERE tc.CONSTRAINT_TYPE = 'FOREIGN KEY' 
                AND tc.TABLE_NAME = '{}'
                AND tc.TABLE_SCHEMA = '{}'
        ", table.replace('\'', "''"), self.schema.replace('\'', "''"));
        
        let references_stream = client.query(&references_query, &[]).await?;
        let references_results = references_stream.into_results().await?;
        
        // Get tables that reference this table (incoming foreign keys)
        let referenced_by_query = format!("
            SELECT DISTINCT tc.TABLE_NAME
            FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS AS tc
            JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE AS kcu
                ON tc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME 
                AND tc.TABLE_SCHEMA = kcu.TABLE_SCHEMA
            JOIN INFORMATION_SCHEMA.CONSTRAINT_COLUMN_USAGE AS ccu
                ON ccu.CONSTRAINT_NAME = tc.CONSTRAINT_NAME 
                AND ccu.TABLE_SCHEMA = tc.TABLE_SCHEMA
            WHERE tc.CONSTRAINT_TYPE = 'FOREIGN KEY' 
                AND ccu.TABLE_NAME = '{}'
                AND tc.TABLE_SCHEMA = '{}'
        ", table.replace('\'', "''"), self.schema.replace('\'', "''"));
        
        let referenced_by_stream = client.query(&referenced_by_query, &[]).await?;
        let referenced_by_results = referenced_by_stream.into_results().await?;
        
        // Collect all related table names
        let mut related_tables = Vec::new();
        if !references_results.is_empty() {
            for row in &references_results[0] {
                if let Some(name) = row.get::<&str, _>(0) {
                    related_tables.push(name.to_string());
                }
            }
        }
        if !referenced_by_results.is_empty() {
            for row in &referenced_by_results[0] {
                if let Some(name) = row.get::<&str, _>(0) {
                    related_tables.push(name.to_string());
                }
            }
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
        let mut client = self.get_connection().await?;
        
        // Get overall statistics
        let stats_query = format!("
            SELECT 
                COUNT(DISTINCT t.name) as table_count,
                SUM(p.rows) as total_rows,
                SUM(a.total_pages) * 8 * 1024 as total_size
            FROM sys.tables t
            INNER JOIN sys.schemas s ON t.schema_id = s.schema_id
            INNER JOIN sys.partitions p ON t.object_id = p.object_id
            INNER JOIN sys.allocation_units a ON p.partition_id = a.container_id
            WHERE p.index_id IN (0, 1)
                AND t.is_ms_shipped = 0
                AND s.name = '{}'
        ", self.schema.replace('\'', "''"));
        
        let stats_stream = client.query(&stats_query, &[]).await?;
        let stats_results = stats_stream.into_results().await?;
        
        let mut summary = json!({
            "total_tables": 0,
            "total_size_bytes": 0,
            "total_size_human": "0 B",
            "total_rows": 0,
        });
        
        if !stats_results.is_empty() && !stats_results[0].is_empty() {
            let row = &stats_results[0][0];
            let table_count = row.get::<i32, _>(0).unwrap_or(0) as i64;
            let total_rows = row.get::<i64, _>(1).unwrap_or(0);
            let total_size = row.get::<i64, _>(2).unwrap_or(0);
            
            summary = json!({
                "total_tables": table_count,
                "total_size_bytes": total_size,
                "total_size_human": format_bytes(total_size as u64),
                "total_rows": total_rows,
            });
        }
        
        // Get largest tables
        let largest_query = format!("
            SELECT TOP 10
                t.name as table_name,
                p.rows as row_count,
                SUM(a.total_pages) * 8 * 1024 as size_bytes
            FROM sys.tables t
            INNER JOIN sys.schemas s ON t.schema_id = s.schema_id
            INNER JOIN sys.partitions p ON t.object_id = p.object_id
            INNER JOIN sys.allocation_units a ON p.partition_id = a.container_id
            WHERE p.index_id IN (0, 1)
                AND t.is_ms_shipped = 0
                AND s.name = '{}'
            GROUP BY t.name, p.rows
            ORDER BY SUM(a.total_pages) DESC
        ", self.schema.replace('\'', "''"));
        
        let largest_stream = client.query(&largest_query, &[]).await?;
        let largest_results = largest_stream.into_results().await?;
        
        let mut largest_tables = Vec::new();
        if !largest_results.is_empty() {
            for row in &largest_results[0] {
                let size_bytes = row.get::<i64, _>(2).unwrap_or(0);
                largest_tables.push(json!({
                    "name": row.get::<&str, _>(0).unwrap_or(""),
                    "row_count": row.get::<i64, _>(1).unwrap_or(0),
                    "size_bytes": size_bytes,
                    "size_human": format_bytes(size_bytes as u64),
                }));
            }
        }
        
        // Get most connected tables
        let connections_query = format!("
            WITH foreign_keys AS (
                SELECT 
                    tc.TABLE_NAME,
                    COUNT(*) as outgoing_fks
                FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS AS tc
                WHERE tc.CONSTRAINT_TYPE = 'FOREIGN KEY' 
                    AND tc.TABLE_SCHEMA = '{}'
                GROUP BY tc.TABLE_NAME
            ),
            referenced AS (
                SELECT 
                    ccu.TABLE_NAME,
                    COUNT(*) as incoming_fks
                FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS AS tc
                JOIN INFORMATION_SCHEMA.CONSTRAINT_COLUMN_USAGE AS ccu
                    ON ccu.CONSTRAINT_NAME = tc.CONSTRAINT_NAME 
                    AND ccu.TABLE_SCHEMA = tc.TABLE_SCHEMA
                WHERE tc.CONSTRAINT_TYPE = 'FOREIGN KEY' 
                    AND tc.TABLE_SCHEMA = '{}'
                GROUP BY ccu.TABLE_NAME
            )
            SELECT TOP 10
                t.TABLE_NAME,
                COALESCE(f.outgoing_fks, 0) as references_count,
                COALESCE(r.incoming_fks, 0) as referenced_by_count,
                COALESCE(f.outgoing_fks, 0) + COALESCE(r.incoming_fks, 0) as total_connections
            FROM INFORMATION_SCHEMA.TABLES t
            LEFT JOIN foreign_keys f ON t.TABLE_NAME = f.TABLE_NAME
            LEFT JOIN referenced r ON t.TABLE_NAME = r.TABLE_NAME
            WHERE t.TABLE_SCHEMA = '{}' 
                AND t.TABLE_TYPE = 'BASE TABLE'
                AND (f.outgoing_fks > 0 OR r.incoming_fks > 0)
            ORDER BY total_connections DESC
        ", self.schema.replace('\'', "''"), self.schema.replace('\'', "''"), self.schema.replace('\'', "''"));
        
        let connections_stream = client.query(&connections_query, &[]).await?;
        let connections_results = connections_stream.into_results().await?;
        
        let mut most_connected_tables = Vec::new();
        if !connections_results.is_empty() {
            for row in &connections_results[0] {
                most_connected_tables.push(json!({
                    "name": row.get::<&str, _>(0).unwrap_or(""),
                    "references_count": row.get::<i32, _>(1).unwrap_or(0),
                    "referenced_by_count": row.get::<i32, _>(2).unwrap_or(0),
                    "total_connections": row.get::<i32, _>(3).unwrap_or(0),
                }));
            }
        }
        
        Ok(json!({
            "summary": summary,
            "largest_tables": largest_tables,
            "most_connected_tables": most_connected_tables,
        }))
    }
}