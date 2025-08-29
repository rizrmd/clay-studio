use super::base::{DataSourceConnector, format_bytes};
use serde_json::{json, Value};
use tiberius::{Client, Config, AuthMethod, EncryptionLevel};
use tokio::net::TcpStream;
use tokio_util::compat::{TokioAsyncWriteCompatExt, Compat};
use std::error::Error;
use async_trait::async_trait;

pub struct SqlServerConnector {
    config: Config,
    server: String,
}

impl SqlServerConnector {
    pub fn new(config: &Value) -> Result<Self, Box<dyn Error>> {
        // Get connection parameters
        let host = config.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
        let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(1433) as u16;
        let database = config.get("database").and_then(|v| v.as_str());
        let username = config.get("username").and_then(|v| v.as_str()).unwrap_or("sa");
        let password = config.get("password").and_then(|v| v.as_str()).unwrap_or("");
        
        // Create Tiberius config
        let mut tiberius_config = Config::new();
        
        // Set authentication method (SQL Server authentication)
        tiberius_config.authentication(AuthMethod::sql_server(username, password));
        
        // Set database if provided
        if let Some(db) = database {
            tiberius_config.database(db);
        }
        
        // Set trust settings
        let trust_cert = config.get("trust_server_certificate").and_then(|v| v.as_bool()).unwrap_or(false);
        if trust_cert {
            tiberius_config.trust_cert();
        }
        
        // Set encryption level
        let encrypt = config.get("encrypt").and_then(|v| v.as_bool()).unwrap_or(false);
        if !encrypt {
            tiberius_config.encryption(EncryptionLevel::Off);
        }
        
        let server = format!("{}:{}", host, port);
        
        eprintln!("[DEBUG] SQL Server connector configured for: {}", server);
        eprintln!("[DEBUG] Using database: {:?}", database);
        eprintln!("[DEBUG] Trust Server Certificate: {}", trust_cert);
        eprintln!("[DEBUG] Encryption: {}", encrypt);
        
        Ok(Self {
            config: tiberius_config,
            server,
        })
    }
    
    async fn connect(&self) -> Result<Client<Compat<TcpStream>>, Box<dyn Error>> {
        let tcp = TcpStream::connect(&self.server).await?;
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
        eprintln!("[DEBUG] ========== SQL Server Connection Test Started ==========");
        eprintln!("[DEBUG] Attempting SQL Server connection to: {}", self.server);
        
        match self.connect().await {
            Ok(mut client) => {
                eprintln!("[DEBUG] SQL Server connection established successfully");
                
                // Try a simple query
                eprintln!("[DEBUG] Executing test query: SELECT 1 as test");
                let test_result = client.query("SELECT 1 as test", &[]).await;
                
                match test_result {
                    Ok(stream) => {
                        let _rows: Vec<_> = stream.into_results().await?;
                        eprintln!("[SUCCESS] SQL Server test query successful - connection is working!");
                        eprintln!("[DEBUG] ========== SQL Server Connection Test Completed Successfully ==========");
                        Ok(true)
                    }
                    Err(e) => {
                        eprintln!("[ERROR] SQL Server test query failed: {}", e);
                        eprintln!("[DEBUG] ========== SQL Server Connection Test Failed ==========");
                        Err(format!("SQL Server test query failed: {}", e).into())
                    }
                }
            }
            Err(e) => {
                eprintln!("[ERROR] Failed to connect to SQL Server");
                eprintln!("[ERROR] Error message: {}", e);
                eprintln!("[ERROR] Error type: {:?}", e);
                
                let error_string = e.to_string();
                eprintln!("[DEBUG] Analyzing error type...");
                
                if error_string.contains("Login failed") {
                    eprintln!("[DIAGNOSIS] Authentication failed - check username and password");
                    eprintln!("[HINT] Common causes:");
                    eprintln!("  - Incorrect password");
                    eprintln!("  - User doesn't exist");
                    eprintln!("  - SQL Server authentication not enabled");
                } else if error_string.contains("Cannot open database") {
                    eprintln!("[DIAGNOSIS] Database does not exist or user lacks permission");
                    eprintln!("[HINT] Create the database first or check user permissions");
                } else if error_string.contains("Connection refused") || error_string.contains("No connection") {
                    eprintln!("[DIAGNOSIS] Cannot reach SQL Server - check host and port");
                    eprintln!("[HINT] Common causes:");
                    eprintln!("  - SQL Server is not running");
                    eprintln!("  - Incorrect host or port");
                    eprintln!("  - Firewall blocking the connection");
                    eprintln!("  - SQL Server not configured for TCP/IP connections");
                } else if error_string.contains("timeout") {
                    eprintln!("[DIAGNOSIS] Connection timeout - server may be unreachable");
                    eprintln!("[HINT] Check network connectivity and server status");
                } else if error_string.contains("certificate") || error_string.contains("TLS") {
                    eprintln!("[DIAGNOSIS] SSL/TLS certificate issue");
                    eprintln!("[HINT] Try setting \"trust_server_certificate\": true in your config");
                }
                
                eprintln!("[DEBUG] ========== SQL Server Connection Test Failed ==========");
                Err(format!("SQL Server test query failed: {}", e).into())
            }
        }
    }
    
    async fn execute_query(&self, query: &str, limit: i32) -> Result<Value, Box<dyn Error>> {
        let mut client = self.connect().await?;
        
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
        let mut client = self.connect().await?;
        
        let query = "
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
                AND t.TABLE_SCHEMA NOT IN ('sys', 'INFORMATION_SCHEMA')
            ORDER BY t.TABLE_NAME, c.ORDINAL_POSITION
        ";
        
        let stream = client.query(query, &[]).await?;
        let results = stream.into_results().await?;
        
        let mut schema = json!({
            "tables": {},
            "refreshed_at": chrono::Utc::now().to_rfc3339()
        });
        
        if !results.is_empty() {
            for row in &results[0] {
                let table_name: &str = row.get(0).unwrap_or("");
                let column_info = json!({
                    "column_name": row.get::<&str, _>(1).unwrap_or(""),
                    "data_type": row.get::<&str, _>(2).unwrap_or(""),
                    "is_nullable": row.get::<&str, _>(3).unwrap_or(""),
                });
                
                if schema["tables"].get(table_name).is_none() {
                    schema["tables"][table_name] = json!([]);
                }
                schema["tables"][table_name].as_array_mut().unwrap().push(column_info);
            }
        }
        
        Ok(schema)
    }
    
    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut client = self.connect().await?;
        
        let query = "
            SELECT TABLE_NAME 
            FROM INFORMATION_SCHEMA.TABLES 
            WHERE TABLE_TYPE = 'BASE TABLE' 
                AND TABLE_SCHEMA NOT IN ('sys', 'INFORMATION_SCHEMA')
            ORDER BY TABLE_NAME
        ";
        
        let stream = client.query(query, &[]).await?;
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
        let mut client = self.connect().await?;
        
        // Get basic statistics
        let stats_query = "
            SELECT 
                COUNT(*) as table_count,
                SUM(p.rows) as total_rows
            FROM sys.tables t
            INNER JOIN sys.partitions p ON t.object_id = p.object_id
            WHERE p.index_id IN (0, 1)
                AND t.is_ms_shipped = 0
        ";
        
        let stats_stream = client.query(stats_query, &[]).await?;
        let stats_results = stats_stream.into_results().await?;
        
        let mut table_count: i64 = 0;
        let mut total_rows: i64 = 0;
        
        if !stats_results.is_empty() && !stats_results[0].is_empty() {
            let row = &stats_results[0][0];
            table_count = row.get::<i32, _>(0).unwrap_or(0) as i64;
            total_rows = row.get::<i64, _>(1).unwrap_or(0);
        }
        
        // Get detailed table information
        let tables_query = "
            SELECT TOP 50
                t.name as table_name,
                p.rows as row_count,
                SUM(a.total_pages) * 8 * 1024 as size_bytes
            FROM sys.tables t
            INNER JOIN sys.partitions p ON t.object_id = p.object_id
            INNER JOIN sys.allocation_units a ON p.partition_id = a.container_id
            WHERE p.index_id IN (0, 1)
                AND t.is_ms_shipped = 0
            GROUP BY t.name, p.rows
            ORDER BY SUM(a.total_pages) DESC
        ";
        
        let tables_stream = client.query(tables_query, &[]).await?;
        let tables_results = tables_stream.into_results().await?;
        
        let mut key_tables = Vec::new();
        let mut largest_tables = Vec::new();
        let mut table_names = Vec::new();
        
        if !tables_results.is_empty() {
            for (idx, row) in tables_results[0].iter().enumerate() {
                let table_name = row.get::<&str, _>(0).unwrap_or("").to_string();
                let row_count = row.get::<i64, _>(1).unwrap_or(0);
                let size_bytes = row.get::<i64, _>(2).unwrap_or(0);
                
                table_names.push(table_name.clone());
                
                if idx < 10 {
                    largest_tables.push(json!({
                        "name": table_name.clone(),
                        "size_bytes": size_bytes,
                        "size_human": format_bytes(size_bytes as u64),
                        "row_count": row_count,
                    }));
                }
                
                if idx < 5 {
                    key_tables.push(json!({
                        "name": table_name,
                        "size_bytes": size_bytes,
                        "row_count": row_count,
                        "connections": 0,  // Would need additional query for foreign keys
                    }));
                }
            }
        }
        
        // Calculate total size
        let total_size: i64 = largest_tables.iter()
            .map(|t| t["size_bytes"].as_i64().unwrap_or(0))
            .sum();
        
        Ok(json!({
            "statistics": {
                "table_count": table_count,
                "total_size": total_size,
                "total_size_human": format_bytes(total_size as u64),
                "total_rows": total_rows,
            },
            "table_names": table_names,
            "key_tables": key_tables,
            "largest_tables": largest_tables,
            "analyzed_at": chrono::Utc::now().to_rfc3339(),
        }))
    }
    
    async fn get_tables_schema(&self, tables: Vec<&str>) -> Result<Value, Box<dyn Error>> {
        let mut client = self.connect().await?;
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
                WHERE TABLE_NAME = '{}'
                ORDER BY ORDINAL_POSITION
            ", table_name.replace('\'', "''"));
            
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
                SELECT COLUMN_NAME
                FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE
                WHERE TABLE_NAME = '{}'
                    AND CONSTRAINT_NAME LIKE 'PK_%'
            ", table_name.replace('\'', "''"));
            
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
            
            // Get row count
            let count_query = format!("
                SELECT COUNT(*) as row_count
                FROM [{}]
            ", table_name.replace(']', "]]"));
            
            let count_stream = client.query(&count_query, &[]).await?;
            let count_results = count_stream.into_results().await?;
            
            let mut row_count: i64 = 0;
            if !count_results.is_empty() && !count_results[0].is_empty() {
                row_count = count_results[0][0].get::<i32, _>(0).unwrap_or(0) as i64;
            }
            
            // Get sample data
            let sample_query = format!("SELECT TOP 5 * FROM [{}]", table_name.replace(']', "]]"));
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
            
            result[table_name] = json!({
                "columns": columns_info,
                "primary_keys": primary_keys,
                "foreign_keys": [],  // Would need additional query for foreign keys
                "row_count": row_count,
                "sample_data": sample_data,
            });
        }
        
        Ok(result)
    }
    
    async fn search_tables(&self, pattern: &str) -> Result<Value, Box<dyn Error>> {
        let mut client = self.connect().await?;
        
        let query = format!("
            SELECT 
                t.name as table_name,
                p.rows as row_count
            FROM sys.tables t
            INNER JOIN sys.partitions p ON t.object_id = p.object_id
            WHERE p.index_id IN (0, 1)
                AND t.is_ms_shipped = 0
                AND t.name LIKE '{}'
            ORDER BY t.name
        ", pattern.replace('\'', "''"));
        
        let stream = client.query(&query, &[]).await?;
        let results = stream.into_results().await?;
        
        let mut matches = Vec::new();
        if !results.is_empty() {
            for row in &results[0] {
                matches.push(json!({
                    "name": row.get::<&str, _>(0).unwrap_or(""),
                    "row_count": row.get::<i64, _>(1).unwrap_or(0),
                }));
            }
        }
        
        Ok(json!({
            "matches": matches,
            "total_matches": matches.len(),
        }))
    }
    
    async fn get_related_tables(&self, table: &str) -> Result<Value, Box<dyn Error>> {
        let main_schema = self.get_tables_schema(vec![table]).await?;
        
        if main_schema.get(table).is_none() {
            return Err("Table not found".into());
        }
        
        // For SQL Server, we would need to query foreign key relationships
        // For now, return just the main table
        Ok(json!({
            "main_table": main_schema[table],
            "related_tables": json!({}),
            "relationship_count": 0,
        }))
    }
    
    async fn get_database_stats(&self) -> Result<Value, Box<dyn Error>> {
        let mut client = self.connect().await?;
        
        // Get overall statistics
        let stats_query = "
            SELECT 
                COUNT(DISTINCT t.name) as table_count,
                SUM(p.rows) as total_rows,
                SUM(a.total_pages) * 8 * 1024 as total_size
            FROM sys.tables t
            INNER JOIN sys.partitions p ON t.object_id = p.object_id
            INNER JOIN sys.allocation_units a ON p.partition_id = a.container_id
            WHERE p.index_id IN (0, 1)
                AND t.is_ms_shipped = 0
        ";
        
        let stats_stream = client.query(stats_query, &[]).await?;
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
        let largest_query = "
            SELECT TOP 10
                t.name as table_name,
                p.rows as row_count,
                SUM(a.total_pages) * 8 * 1024 as size_bytes
            FROM sys.tables t
            INNER JOIN sys.partitions p ON t.object_id = p.object_id
            INNER JOIN sys.allocation_units a ON p.partition_id = a.container_id
            WHERE p.index_id IN (0, 1)
                AND t.is_ms_shipped = 0
            GROUP BY t.name, p.rows
            ORDER BY SUM(a.total_pages) DESC
        ";
        
        let largest_stream = client.query(largest_query, &[]).await?;
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
        
        Ok(json!({
            "summary": summary,
            "largest_tables": largest_tables,
            "most_connected_tables": [],  // Would need additional query for foreign keys
        }))
    }
}