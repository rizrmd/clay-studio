use serde_json::{json, Value};
use sqlx::{postgres::PgPool, mysql::MySqlPool, sqlite::SqlitePool, Row as SqlxRow, Column};
use std::error::Error;
use async_trait::async_trait;

#[derive(Debug)]
pub enum DataSourceType {
    PostgreSQL,
    MySQL,
    SQLite,
    CSV,
}

impl From<&str> for DataSourceType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "postgresql" | "postgres" => DataSourceType::PostgreSQL,
            "mysql" => DataSourceType::MySQL,
            "sqlite" => DataSourceType::SQLite,
            "csv" => DataSourceType::CSV,
            _ => DataSourceType::PostgreSQL, // default
        }
    }
}

#[async_trait]
pub trait DataSourceConnector: Send + Sync {
    #[allow(dead_code)]
    async fn test_connection(&self) -> Result<bool, Box<dyn Error>>;
    #[allow(dead_code)]
    async fn fetch_schema(&self) -> Result<Value, Box<dyn Error>>;
    async fn execute_query(&self, query: &str, limit: i32) -> Result<Value, Box<dyn Error>>;
    #[allow(dead_code)]
    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error>>;
}

pub struct PostgreSQLConnector {
    connection_string: String,
}

impl PostgreSQLConnector {
    pub fn new(config: &Value) -> Result<Self, Box<dyn Error>> {
        let host = config.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
        let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(5432);
        let database = config.get("database").and_then(|v| v.as_str()).ok_or("Missing database name")?;
        let username = config.get("username").and_then(|v| v.as_str()).unwrap_or("postgres");
        let password = config.get("password").and_then(|v| v.as_str()).unwrap_or("");
        
        let connection_string = if password.is_empty() {
            format!("postgres://{}@{}:{}/{}", username, host, port, database)
        } else {
            format!("postgres://{}:{}@{}:{}/{}", username, password, host, port, database)
        };
        
        Ok(Self { connection_string })
    }
}

#[async_trait]
impl DataSourceConnector for PostgreSQLConnector {
    async fn test_connection(&self) -> Result<bool, Box<dyn Error>> {
        let pool = PgPool::connect(&self.connection_string).await?;
        sqlx::query("SELECT 1").fetch_one(&pool).await?;
        Ok(true)
    }
    
    async fn fetch_schema(&self) -> Result<Value, Box<dyn Error>> {
        let pool = PgPool::connect(&self.connection_string).await?;
        
        // Fetch table and column information
        let tables = sqlx::query(
            "SELECT 
                t.table_name,
                array_agg(
                    json_build_object(
                        'column_name', c.column_name,
                        'data_type', c.data_type,
                        'is_nullable', c.is_nullable
                    ) ORDER BY c.ordinal_position
                ) as columns
             FROM information_schema.tables t
             JOIN information_schema.columns c ON t.table_name = c.table_name
             WHERE t.table_schema = 'public' 
             AND t.table_type = 'BASE TABLE'
             GROUP BY t.table_name
             ORDER BY t.table_name"
        )
        .fetch_all(&pool)
        .await?;
        
        let mut schema = json!({
            "tables": {},
            "refreshed_at": chrono::Utc::now().to_rfc3339()
        });
        
        for row in tables {
            let table_name: String = row.get("table_name");
            let columns: Value = row.get("columns");
            schema["tables"][table_name] = columns;
        }
        
        Ok(schema)
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
    
    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let pool = PgPool::connect(&self.connection_string).await?;
        
        let tables = sqlx::query(
            "SELECT table_name 
             FROM information_schema.tables 
             WHERE table_schema = 'public' 
             AND table_type = 'BASE TABLE'
             ORDER BY table_name"
        )
        .fetch_all(&pool)
        .await?;
        
        let table_names: Vec<String> = tables.iter().map(|row| row.get("table_name")).collect();
        Ok(table_names)
    }
}

pub struct MySQLConnector {
    connection_string: String,
}

impl MySQLConnector {
    pub fn new(config: &Value) -> Result<Self, Box<dyn Error>> {
        let host = config.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
        let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(3306);
        let database = config.get("database").and_then(|v| v.as_str()).ok_or("Missing database name")?;
        let username = config.get("username").and_then(|v| v.as_str()).unwrap_or("root");
        let password = config.get("password").and_then(|v| v.as_str()).unwrap_or("");
        
        let connection_string = if password.is_empty() {
            format!("mysql://{}@{}:{}/{}", username, host, port, database)
        } else {
            format!("mysql://{}:{}@{}:{}/{}", username, password, host, port, database)
        };
        
        Ok(Self { connection_string })
    }
}

#[async_trait]
impl DataSourceConnector for MySQLConnector {
    async fn test_connection(&self) -> Result<bool, Box<dyn Error>> {
        let pool = MySqlPool::connect(&self.connection_string).await?;
        sqlx::query("SELECT 1").fetch_one(&pool).await?;
        Ok(true)
    }
    
    async fn fetch_schema(&self) -> Result<Value, Box<dyn Error>> {
        let pool = MySqlPool::connect(&self.connection_string).await?;
        
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
            
            if !schema["tables"].get(&table_name).is_some() {
                schema["tables"][&table_name] = json!([]);
            }
            schema["tables"][&table_name].as_array_mut().unwrap().push(column_info);
        }
        
        Ok(schema)
    }
    
    async fn execute_query(&self, query: &str, limit: i32) -> Result<Value, Box<dyn Error>> {
        let pool = MySqlPool::connect(&self.connection_string).await?;
        
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
        
        Ok(json!({
            "columns": columns,
            "rows": result_rows,
            "row_count": result_rows.len(),
            "execution_time_ms": execution_time_ms
        }))
    }
    
    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let pool = MySqlPool::connect(&self.connection_string).await?;
        
        let tables = sqlx::query("SHOW TABLES")
            .fetch_all(&pool)
            .await?;
        
        let table_names: Vec<String> = tables.iter().map(|row| row.get(0)).collect();
        Ok(table_names)
    }
}

pub struct SQLiteConnector {
    connection_string: String,
}

impl SQLiteConnector {
    pub fn new(config: &Value) -> Result<Self, Box<dyn Error>> {
        let path = config.get("path").and_then(|v| v.as_str()).ok_or("Missing database path")?;
        let connection_string = format!("sqlite://{}", path);
        Ok(Self { connection_string })
    }
}

#[async_trait]
impl DataSourceConnector for SQLiteConnector {
    async fn test_connection(&self) -> Result<bool, Box<dyn Error>> {
        let pool = SqlitePool::connect(&self.connection_string).await?;
        sqlx::query("SELECT 1").fetch_one(&pool).await?;
        Ok(true)
    }
    
    async fn fetch_schema(&self) -> Result<Value, Box<dyn Error>> {
        let pool = SqlitePool::connect(&self.connection_string).await?;
        
        let tables = sqlx::query("SELECT name FROM sqlite_master WHERE type='table'")
            .fetch_all(&pool)
            .await?;
        
        let mut schema = json!({
            "tables": {},
            "refreshed_at": chrono::Utc::now().to_rfc3339()
        });
        
        for table_row in tables {
            let table_name: String = table_row.get("name");
            let columns = sqlx::query(&format!("PRAGMA table_info({})", table_name))
                .fetch_all(&pool)
                .await?;
            
            let mut column_info = Vec::new();
            for col in columns {
                column_info.push(json!({
                    "column_name": col.get::<String, _>("name"),
                    "data_type": col.get::<String, _>("type"),
                    "is_nullable": col.get::<i32, _>("notnull") == 0,
                }));
            }
            schema["tables"][table_name] = json!(column_info);
        }
        
        Ok(schema)
    }
    
    async fn execute_query(&self, query: &str, limit: i32) -> Result<Value, Box<dyn Error>> {
        let pool = SqlitePool::connect(&self.connection_string).await?;
        
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
                    row_data.push(if val { "true" } else { "false" }.to_string());
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
    
    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let pool = SqlitePool::connect(&self.connection_string).await?;
        
        let tables = sqlx::query("SELECT name FROM sqlite_master WHERE type='table'")
            .fetch_all(&pool)
            .await?;
        
        let table_names: Vec<String> = tables.iter().map(|row| row.get("name")).collect();
        Ok(table_names)
    }
}

pub struct CSVConnector {
    file_path: String,
}

impl CSVConnector {
    pub fn new(config: &Value) -> Result<Self, Box<dyn Error>> {
        let file_path = config.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or("Missing file_path")?
            .to_string();
        Ok(Self { file_path })
    }
}

#[async_trait]
impl DataSourceConnector for CSVConnector {
    async fn test_connection(&self) -> Result<bool, Box<dyn Error>> {
        // Check if file exists
        std::fs::metadata(&self.file_path)?;
        Ok(true)
    }
    
    async fn fetch_schema(&self) -> Result<Value, Box<dyn Error>> {
        let mut reader = csv::Reader::from_path(&self.file_path)?;
        let headers = reader.headers()?.clone();
        
        let columns: Vec<Value> = headers.iter().map(|h| {
            json!({
                "column_name": h,
                "data_type": "text",
                "is_nullable": true
            })
        }).collect();
        
        Ok(json!({
            "tables": {
                "csv_data": columns
            },
            "refreshed_at": chrono::Utc::now().to_rfc3339()
        }))
    }
    
    async fn execute_query(&self, _query: &str, limit: i32) -> Result<Value, Box<dyn Error>> {
        let start = std::time::Instant::now();
        let mut reader = csv::Reader::from_path(&self.file_path)?;
        let headers = reader.headers()?.clone();
        
        let columns: Vec<String> = headers.iter().map(|h| h.to_string()).collect();
        let mut rows = Vec::new();
        
        for (i, result) in reader.records().enumerate() {
            if i >= limit as usize {
                break;
            }
            let record = result?;
            let row: Vec<String> = record.iter().map(|f| f.to_string()).collect();
            rows.push(row);
        }
        
        let execution_time_ms = start.elapsed().as_millis() as i64;
        
        Ok(json!({
            "columns": columns,
            "rows": rows,
            "row_count": rows.len(),
            "execution_time_ms": execution_time_ms
        }))
    }
    
    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error>> {
        // CSV files represent a single table
        Ok(vec!["csv_data".to_string()])
    }
}

pub async fn create_connector(source_type: &str, config: &Value) -> Result<Box<dyn DataSourceConnector>, Box<dyn Error>> {
    match DataSourceType::from(source_type) {
        DataSourceType::PostgreSQL => {
            Ok(Box::new(PostgreSQLConnector::new(config)?))
        },
        DataSourceType::MySQL => {
            Ok(Box::new(MySQLConnector::new(config)?))
        },
        DataSourceType::SQLite => {
            Ok(Box::new(SQLiteConnector::new(config)?))
        },
        DataSourceType::CSV => {
            Ok(Box::new(CSVConnector::new(config)?))
        }
    }
}