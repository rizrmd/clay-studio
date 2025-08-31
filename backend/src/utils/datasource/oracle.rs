use super::base::{DataSourceConnector, format_bytes};
use serde_json::{json, Value};
use oracle::Connection;
use std::error::Error;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use std::time::{Duration, Instant};
use std::collections::VecDeque;

// Connection pool configuration
#[derive(Clone)]
struct PoolConfig {
    max_connections: usize,
    min_connections: usize,
    connection_timeout: Duration,
    idle_timeout: Duration,
    max_lifetime: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            connection_timeout: Duration::from_secs(3),
            idle_timeout: Duration::from_secs(300), // 5 minutes
            max_lifetime: Duration::from_secs(1800), // 30 minutes
        }
    }
}

// Pooled connection wrapper
struct PooledConnection {
    connection: Connection,
    created_at: Instant,
    last_used: Instant,
}

impl PooledConnection {
    fn new(connection: Connection) -> Self {
        let now = Instant::now();
        Self {
            connection,
            created_at: now,
            last_used: now,
        }
    }

    fn update_last_used(&mut self) {
        self.last_used = Instant::now();
    }

    fn is_expired(&self, config: &PoolConfig) -> bool {
        let now = Instant::now();
        now.duration_since(self.created_at) > config.max_lifetime ||
        now.duration_since(self.last_used) > config.idle_timeout
    }
}

// Connection pool implementation
struct ConnectionPool {
    config: PoolConfig,
    connections: Arc<Mutex<VecDeque<PooledConnection>>>,
    semaphore: Arc<Semaphore>,
    connection_string: String,
    username: String,
    password: String,
}

impl ConnectionPool {
    fn new(username: String, password: String, connection_string: String, config: PoolConfig) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(config.max_connections)),
            config,
            connections: Arc::new(Mutex::new(VecDeque::new())),
            connection_string,
            username,
            password,
        }
    }

    async fn get_connection(&self) -> Result<PooledConnection, Box<dyn Error + Send + Sync>> {
        // Acquire semaphore permit (limits concurrent connections)
        let _permit = self.semaphore.acquire().await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        // Try to get an existing connection from the pool
        {
            let mut pool = self.connections.lock().await;
            while let Some(mut conn) = pool.pop_front() {
                // Check if connection is still valid and not expired
                if !conn.is_expired(&self.config) {
                    conn.update_last_used();
                    return Ok(conn);
                }
                // Connection expired, it will be dropped
            }
        }

        // No valid connection available, create a new one
        let connection = Connection::connect(&self.username, &self.password, &self.connection_string)
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
            
        Ok(PooledConnection::new(connection))
    }

    async fn return_connection(&self, mut conn: PooledConnection) {
        // Check if connection is still valid before returning to pool
        if !conn.is_expired(&self.config) {
            conn.update_last_used();
            let mut pool = self.connections.lock().await;
            
            // Don't exceed max connections
            if pool.len() < self.config.max_connections {
                pool.push_back(conn);
            }
            // If pool is full, connection will be dropped
        }
        // If connection is expired, it will be dropped
    }

    #[allow(dead_code)]
    async fn cleanup_expired_connections(&self) {
        let mut pool = self.connections.lock().await;
        let mut valid_connections = VecDeque::new();
        
        while let Some(conn) = pool.pop_front() {
            if !conn.is_expired(&self.config) {
                valid_connections.push_back(conn);
            }
            // Expired connections are dropped
        }
        
        *pool = valid_connections;
    }
}

pub struct OracleConnector {
    connection_string: String,
    username: String,
    #[allow(dead_code)]
    password: String,
    schema: String,
    pool: Arc<ConnectionPool>,
}

impl OracleConnector {
    pub fn new(config: &Value) -> Result<Self, Box<dyn Error>> {
        // Get the schema name from config, default to username if not specified
        let username = config.get("username")
            .and_then(|v| v.as_str())
            .ok_or("Missing username")?
            .to_string();
            
        let schema = config.get("schema")
            .and_then(|v| v.as_str())
            .unwrap_or(&username)
            .to_uppercase(); // Oracle schemas are typically uppercase
        
        eprintln!("[DEBUG] Oracle connector using schema: '{}'", schema);
        
        // Build connection string
        let connection_string = if let Some(url) = config.get("url").and_then(|v| v.as_str()) {
            url.to_string()
        } else {
            // Build from components
            let host = config.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
            let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(1521);
            let service_name = config.get("service_name").and_then(|v| v.as_str());
            let sid = config.get("sid").and_then(|v| v.as_str());
            
            if let Some(service) = service_name {
                format!("{}:{}/{}", host, port, service)
            } else if let Some(sid) = sid {
                format!("{}:{}/{}", host, port, sid)
            } else {
                return Err("Must provide either 'service_name' or 'sid' for Oracle connection".into());
            }
        };
        
        let password = config.get("password")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Create pool configuration from config
        let mut pool_config = PoolConfig::default();
        
        if let Some(max_conn) = config.get("max_connections").and_then(|v| v.as_u64()) {
            pool_config.max_connections = max_conn as usize;
        }
        if let Some(min_conn) = config.get("min_connections").and_then(|v| v.as_u64()) {
            pool_config.min_connections = min_conn as usize;
        }
        if let Some(timeout) = config.get("connection_timeout_secs").and_then(|v| v.as_u64()) {
            pool_config.connection_timeout = Duration::from_secs(timeout);
        }
        if let Some(idle_timeout) = config.get("idle_timeout_secs").and_then(|v| v.as_u64()) {
            pool_config.idle_timeout = Duration::from_secs(idle_timeout);
        }
        if let Some(max_lifetime) = config.get("max_lifetime_secs").and_then(|v| v.as_u64()) {
            pool_config.max_lifetime = Duration::from_secs(max_lifetime);
        }

        // Validate pool configuration
        if pool_config.min_connections > pool_config.max_connections {
            pool_config.min_connections = pool_config.max_connections;
        }
        
        // Debug: Log the connection string (with password masked)
        eprintln!("[DEBUG] Oracle connection string: {}@{}", username, connection_string);
        eprintln!("[DEBUG] Oracle pool config: max={}, min={}, timeout={}s", 
                  pool_config.max_connections, pool_config.min_connections, 
                  pool_config.connection_timeout.as_secs());
        
        let pool = Arc::new(ConnectionPool::new(
            username.clone(),
            password.clone(),
            connection_string.clone(),
            pool_config,
        ));
        
        Ok(Self { 
            connection_string,
            username: username.clone(),
            password,
            schema,
            pool,
        })
    }
    
    #[allow(dead_code)]
    async fn with_connection<F, R>(&self, operation: F) -> Result<R, Box<dyn Error + Send + Sync>>
    where
        F: FnOnce(&Connection) -> Result<R, Box<dyn Error + Send + Sync>> + Send,
        R: Send,
    {
        // Get connection from pool
        let pooled_conn = self.pool.get_connection().await?;
        
        // Perform operation
        let result = operation(&pooled_conn.connection);
        
        // Return connection to pool
        self.pool.return_connection(pooled_conn).await;
        
        result
    }

    // Background task to cleanup expired connections
    #[allow(dead_code)]
    pub async fn start_maintenance_task(&self) {
        let pool = Arc::clone(&self.pool);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Run every minute
            
            loop {
                interval.tick().await;
                pool.cleanup_expired_connections().await;
            }
        });
    }
}

#[async_trait]
impl DataSourceConnector for OracleConnector {
    async fn test_connection(&mut self) -> Result<bool, Box<dyn Error>> {
        // Run in blocking context since oracle crate is synchronous
        let pool = Arc::clone(&self.pool);
        
        tokio::task::spawn_blocking(move || -> Result<bool, Box<dyn Error + Send + Sync>> {
            // Create a test connection directly (not from pool for testing)
            match Connection::connect(&pool.username, &pool.password, &pool.connection_string) {
                Ok(conn) => {
                    // Test with a simple query
                    let sql = "SELECT 1 FROM DUAL";
                    match conn.query(sql, &[]) {
                        Ok(_) => {
                            eprintln!("[SUCCESS] Oracle connection successful");
                            Ok(true)
                        },
                        Err(e) => {
                            eprintln!("[DEBUG] Oracle query test failed: {}", e);
                            Err(Box::new(e) as Box<dyn Error + Send + Sync>)
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[ERROR] ========== Oracle Connection Failed ==========");
                    eprintln!("[ERROR] Failed to create Oracle connection");
                    eprintln!("[ERROR] Error message: {}", e);
                    
                    // Analyze error type
                    let error_string = e.to_string();
                    if error_string.contains("ORA-01017") {
                        eprintln!("[DIAGNOSIS] Invalid username/password");
                        eprintln!("[HINT] Check your username and password");
                    } else if error_string.contains("ORA-12154") {
                        eprintln!("[DIAGNOSIS] TNS could not resolve the connect identifier");
                        eprintln!("[HINT] Check your service_name or sid in the connection config");
                    } else if error_string.contains("ORA-12541") {
                        eprintln!("[DIAGNOSIS] TNS no listener");
                        eprintln!("[HINT] Oracle listener is not running or wrong port specified");
                    } else if error_string.contains("ORA-12514") {
                        eprintln!("[DIAGNOSIS] Service not available");
                        eprintln!("[HINT] The specified service is not running");
                    } else {
                        eprintln!("[DIAGNOSIS] Unrecognized Oracle error");
                    }
                    
                    Err(Box::new(e) as Box<dyn Error + Send + Sync>)
                }
            }
        }).await.map_err(|e| Box::new(e) as Box<dyn Error>)?
            .map_err(|e| e as Box<dyn Error>)
    }
    
    async fn execute_query(&self, query: &str, limit: i32) -> Result<Value, Box<dyn Error>> {
        let query = query.to_string();
        let pool = Arc::clone(&self.pool);
        
        tokio::task::spawn_blocking(move || -> Result<Value, Box<dyn Error + Send + Sync>> {
            // Use the connection pool
            let runtime = tokio::runtime::Handle::current();
            let pooled_conn = runtime.block_on(pool.get_connection())?;
            
            // Add ROWNUM limit if not already present and limit is specified
            let modified_query = if limit > 0 && !query.to_uppercase().contains("ROWNUM") && !query.to_uppercase().contains("FETCH") {
                format!("SELECT * FROM ({}) WHERE ROWNUM <= {}", query, limit)
            } else {
                query
            };
            
            let start = std::time::Instant::now();
            let rows = pooled_conn.connection.query(&modified_query, &[])
                .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
            let execution_time_ms = start.elapsed().as_millis() as i64;
            
            let mut results = Vec::new();
            let columns: Vec<String> = rows.column_info()
                .iter()
                .map(|col| col.name().to_string())
                .collect();
            
            for row_result in rows {
                let row = row_result.map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
                let mut record = serde_json::Map::new();
                
                for (i, col_name) in columns.iter().enumerate() {
                    // Get value as a string representation for JSON
                    let json_value = if let Ok(v) = row.get::<_, Option<String>>(i) {
                        match v {
                            Some(s) => json!(s),
                            None => Value::Null,
                        }
                    } else if let Ok(v) = row.get::<_, Option<i64>>(i) {
                        match v {
                            Some(n) => json!(n),
                            None => Value::Null,
                        }
                    } else if let Ok(v) = row.get::<_, Option<f64>>(i) {
                        match v {
                            Some(n) => json!(n),
                            None => Value::Null,
                        }
                    } else {
                        Value::Null
                    };
                    
                    record.insert(col_name.clone(), json_value);
                }
                
                results.push(Value::Object(record));
            }
            
            // Return connection to pool
            let result = json!({
                "columns": columns,
                "rows": results,
                "row_count": results.len(),
                "execution_time_ms": execution_time_ms
            });
            
            runtime.block_on(pool.return_connection(pooled_conn));
            Ok(result)
        }).await.map_err(|e| Box::new(e) as Box<dyn Error>)?
            .map_err(|e| e as Box<dyn Error>)
    }
    
    async fn fetch_schema(&self) -> Result<Value, Box<dyn Error>> {
        let schema = self.schema.clone();
        let pool = Arc::clone(&self.pool);
        
        tokio::task::spawn_blocking(move || -> Result<Value, Box<dyn Error + Send + Sync>> {
            let runtime = tokio::runtime::Handle::current();
            let pooled_conn = runtime.block_on(pool.get_connection())?;
            
            let sql = format!(r#"
                SELECT 
                    t.table_name,
                    t.table_type,
                    c.column_name,
                    c.data_type,
                    c.data_length,
                    c.nullable,
                    c.column_id
                FROM all_tables t
                JOIN all_tab_columns c ON t.owner = c.owner AND t.table_name = c.table_name
                WHERE t.owner = '{}'
                ORDER BY t.table_name, c.column_id
            "#, schema);
            
            let rows = pooled_conn.connection.query(&sql, &[])
                .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
            
            let mut schema_map: std::collections::HashMap<String, Vec<Value>> = std::collections::HashMap::new();
            
            for row_result in rows {
                let row = row_result.map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
                let table_name: String = row.get(0).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
                let column_info = json!({
                    "name": row.get::<_, String>(2).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?,
                    "type": row.get::<_, String>(3).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?,
                    "max_length": row.get::<_, Option<i32>>(4).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?,
                    "nullable": row.get::<_, String>(5).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)? == "Y",
                    "position": row.get::<_, i32>(6).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?
                });
                
                schema_map.entry(table_name)
                    .or_insert_with(Vec::new)
                    .push(column_info);
            }
            
            let mut tables = Vec::new();
            for (table_name, columns) in schema_map {
                tables.push(json!({
                    "name": table_name,
                    "columns": columns
                }));
            }
            
            let result = json!({
                "database": schema,
                "tables": tables
            });
            
            runtime.block_on(pool.return_connection(pooled_conn));
            Ok(result)
        }).await.map_err(|e| Box::new(e) as Box<dyn Error>)?
            .map_err(|e| e as Box<dyn Error>)
    }
    
    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let schema = self.schema.clone();
        let pool = Arc::clone(&self.pool);
        
        tokio::task::spawn_blocking(move || -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
            let runtime = tokio::runtime::Handle::current();
            let pooled_conn = runtime.block_on(pool.get_connection())?;
            
            let sql = format!(
                "SELECT table_name FROM all_tables WHERE owner = '{}' ORDER BY table_name",
                schema
            );
            
            let rows = pooled_conn.connection.query(&sql, &[])
                .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
            
            let mut tables = Vec::new();
            for row_result in rows {
                let row = row_result.map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
                let table_name: String = row.get(0).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
                tables.push(table_name);
            }
            
            runtime.block_on(pool.return_connection(pooled_conn));
            Ok(tables)
        }).await.map_err(|e| Box::new(e) as Box<dyn Error>)?
            .map_err(|e| e as Box<dyn Error>)
    }
    
    async fn analyze_database(&self) -> Result<Value, Box<dyn Error>> {
        let schema = self.schema.clone();
        let username = self.username.clone();
        let connection_string = self.connection_string.clone();
        let pool = Arc::clone(&self.pool);
        
        tokio::task::spawn_blocking(move || -> Result<Value, Box<dyn Error + Send + Sync>> {
            let runtime = tokio::runtime::Handle::current();
            let pooled_conn = runtime.block_on(pool.get_connection())?;
            
            // Get database size - using user_segments if dba_segments is not accessible
            let size_sql = format!(r#"
                SELECT SUM(bytes) as total_bytes
                FROM user_segments
            "#);
            
            let mut total_size: u64 = 0;
            if let Ok(rows) = pooled_conn.connection.query(&size_sql, &[]) {
                for row_result in rows {
                    if let Ok(row) = row_result {
                        if let Ok(size) = row.get::<_, Option<u64>>(0) {
                            total_size = size.unwrap_or(0);
                            break;
                        }
                    }
                }
            }
            
            // Get table count
            let table_count_sql = format!(
                "SELECT COUNT(*) FROM all_tables WHERE owner = '{}'",
                schema
            );
            
            let rows = pooled_conn.connection.query(&table_count_sql, &[])
                .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
            let mut table_count = 0;
            for row_result in rows {
                let row = row_result.map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
                table_count = row.get::<_, i32>(0).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
                break;
            }
            
            // Get top tables by row count
            let top_tables_sql = format!(r#"
                SELECT * FROM (
                    SELECT 
                        table_name,
                        num_rows,
                        blocks * 8192 as estimated_size
                    FROM all_tables
                    WHERE owner = '{}' AND num_rows IS NOT NULL
                    ORDER BY num_rows DESC
                ) WHERE ROWNUM <= 10
            "#, schema);
            
            let mut top_tables = Vec::new();
            if let Ok(rows) = pooled_conn.connection.query(&top_tables_sql, &[]) {
                for row_result in rows {
                    if let Ok(row) = row_result {
                        if let (Ok(name), Ok(row_count), Ok(size)) = (
                            row.get::<_, String>(0),
                            row.get::<_, Option<i64>>(1),
                            row.get::<_, Option<u64>>(2)
                        ) {
                            top_tables.push(json!({
                                "name": name,
                                "row_count": row_count,
                                "size": format_bytes(size.unwrap_or(0))
                            }));
                        }
                    }
                }
            }
            
            let result = json!({
                "database": schema,
                "total_size": format_bytes(total_size),
                "table_count": table_count,
                "top_tables": top_tables,
                "connection_info": {
                    "host": connection_string,
                    "user": username,
                    "schema": schema
                }
            });
            
            runtime.block_on(pool.return_connection(pooled_conn));
            Ok(result)
        }).await.map_err(|e| Box::new(e) as Box<dyn Error>)?
            .map_err(|e| e as Box<dyn Error>)
    }
    
    async fn get_tables_schema(&self, tables: Vec<&str>) -> Result<Value, Box<dyn Error>> {
        let schema = self.schema.clone();
        let tables = tables.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let pool = Arc::clone(&self.pool);
        
        tokio::task::spawn_blocking(move || -> Result<Value, Box<dyn Error + Send + Sync>> {
            let runtime = tokio::runtime::Handle::current();
            let pooled_conn = runtime.block_on(pool.get_connection())?;
            
            let table_list = tables.iter()
                .map(|t| format!("'{}'", t.to_uppercase()))
                .collect::<Vec<_>>()
                .join(",");
            
            let sql = format!(r#"
                SELECT 
                    t.table_name,
                    c.column_name,
                    c.data_type,
                    c.data_length,
                    c.nullable,
                    c.column_id
                FROM all_tables t
                JOIN all_tab_columns c ON t.owner = c.owner AND t.table_name = c.table_name
                WHERE t.owner = '{}' AND t.table_name IN ({})
                ORDER BY t.table_name, c.column_id
            "#, schema, table_list);
            
            let rows = pooled_conn.connection.query(&sql, &[])
                .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
            
            let mut schema_map: std::collections::HashMap<String, Vec<Value>> = std::collections::HashMap::new();
            
            for row_result in rows {
                let row = row_result.map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
                let table_name: String = row.get(0).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
                let column_info = json!({
                    "name": row.get::<_, String>(1).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?,
                    "type": row.get::<_, String>(2).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?,
                    "max_length": row.get::<_, Option<i32>>(3).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?,
                    "nullable": row.get::<_, String>(4).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)? == "Y",
                    "position": row.get::<_, i32>(5).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?
                });
                
                schema_map.entry(table_name)
                    .or_insert_with(Vec::new)
                    .push(column_info);
            }
            
            let mut result = Vec::new();
            for (table_name, columns) in schema_map {
                result.push(json!({
                    "name": table_name,
                    "columns": columns
                }));
            }
            
            let json_result = json!(result);
            runtime.block_on(pool.return_connection(pooled_conn));
            Ok(json_result)
        }).await.map_err(|e| Box::new(e) as Box<dyn Error>)?
            .map_err(|e| e as Box<dyn Error>)
    }
    
    async fn search_tables(&self, pattern: &str) -> Result<Value, Box<dyn Error>> {
        let schema = self.schema.clone();
        let pattern = pattern.to_string();
        let pool = Arc::clone(&self.pool);
        
        tokio::task::spawn_blocking(move || -> Result<Value, Box<dyn Error + Send + Sync>> {
            let runtime = tokio::runtime::Handle::current();
            let pooled_conn = runtime.block_on(pool.get_connection())?;
            
            let sql = format!(
                "SELECT table_name FROM all_tables WHERE owner = '{}' AND UPPER(table_name) LIKE UPPER('%{}%') ORDER BY table_name",
                schema, pattern
            );
            
            let rows = pooled_conn.connection.query(&sql, &[])
                .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
            
            let mut tables = Vec::new();
            for row_result in rows {
                let row = row_result.map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
                tables.push(json!({
                    "name": row.get::<_, String>(0).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?
                }));
            }
            
            let result = json!({
                "pattern": pattern,
                "matches": tables
            });
            
            runtime.block_on(pool.return_connection(pooled_conn));
            Ok(result)
        }).await.map_err(|e| Box::new(e) as Box<dyn Error>)?
            .map_err(|e| e as Box<dyn Error>)
    }
    
    async fn get_related_tables(&self, table: &str) -> Result<Value, Box<dyn Error>> {
        let schema = self.schema.clone();
        let table = table.to_string();
        let pool = Arc::clone(&self.pool);
        
        tokio::task::spawn_blocking(move || -> Result<Value, Box<dyn Error + Send + Sync>> {
            let runtime = tokio::runtime::Handle::current();
            let pooled_conn = runtime.block_on(pool.get_connection())?;
            
            // Get foreign key relationships
            let sql = format!(r#"
                SELECT 
                    a.constraint_name,
                    a.table_name as from_table,
                    c.column_name as from_column,
                    b.table_name as to_table,
                    d.column_name as to_column
                FROM all_constraints a
                JOIN all_constraints b ON a.r_constraint_name = b.constraint_name AND a.r_owner = b.owner
                JOIN all_cons_columns c ON a.constraint_name = c.constraint_name AND a.owner = c.owner
                JOIN all_cons_columns d ON b.constraint_name = d.constraint_name AND b.owner = d.owner
                WHERE a.owner = '{}' 
                    AND a.constraint_type = 'R'
                    AND (a.table_name = '{}' OR b.table_name = '{}')
            "#, schema, table.to_uppercase(), table.to_uppercase());
            
            let rows = pooled_conn.connection.query(&sql, &[])
                .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
            
            let mut relationships = Vec::new();
            for row_result in rows {
                let row = row_result.map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
                relationships.push(json!({
                    "constraint_name": row.get::<_, String>(0).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?,
                    "from_table": row.get::<_, String>(1).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?,
                    "from_column": row.get::<_, String>(2).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?,
                    "to_table": row.get::<_, String>(3).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?,
                    "to_column": row.get::<_, String>(4).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?
                }));
            }
            
            let result = json!({
                "table": table,
                "relationships": relationships
            });
            
            runtime.block_on(pool.return_connection(pooled_conn));
            Ok(result)
        }).await.map_err(|e| Box::new(e) as Box<dyn Error>)?
            .map_err(|e| e as Box<dyn Error>)
    }
    
    async fn get_database_stats(&self) -> Result<Value, Box<dyn Error>> {
        let schema = self.schema.clone();
        let pool = Arc::clone(&self.pool);
        
        tokio::task::spawn_blocking(move || -> Result<Value, Box<dyn Error + Send + Sync>> {
            let runtime = tokio::runtime::Handle::current();
            let pooled_conn = runtime.block_on(pool.get_connection())?;
            
            // Get various statistics
            let stats_sql = format!(r#"
                SELECT 
                    (SELECT COUNT(*) FROM all_tables WHERE owner = '{}') as table_count,
                    (SELECT COUNT(*) FROM all_views WHERE owner = '{}') as view_count,
                    (SELECT COUNT(*) FROM all_indexes WHERE owner = '{}') as index_count,
                    (SELECT COUNT(*) FROM all_procedures WHERE owner = '{}') as procedure_count
                FROM DUAL
            "#, schema, schema, schema, schema);
            
            let rows = pooled_conn.connection.query(&stats_sql, &[])
                .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
            
            let mut stats = json!({});
            for row_result in rows {
                let row = row_result.map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
                stats = json!({
                    "table_count": row.get::<_, i32>(0).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?,
                    "view_count": row.get::<_, i32>(1).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?,
                    "index_count": row.get::<_, i32>(2).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?,
                    "procedure_count": row.get::<_, i32>(3).map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?
                });
                break;
            }
            
            let result = json!({
                "database": schema,
                "stats": stats
            });
            
            runtime.block_on(pool.return_connection(pooled_conn));
            Ok(result)
        }).await.map_err(|e| Box::new(e) as Box<dyn Error>)?
            .map_err(|e| e as Box<dyn Error>)
    }
}