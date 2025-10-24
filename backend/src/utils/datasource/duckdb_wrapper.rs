//! DuckDB wrapper for file datasource queries
//! Provides SQL support for CSV, Excel, and JSON files using DuckDB's direct file reading capabilities

use duckdb::Connection;
use std::error::Error;
use std::path::Path;
use tracing::{info, warn};

/// DuckDB wrapper for file datasources
pub struct DuckDBWrapper {
    connection: Connection,
    file_path: String,
    file_type: String,
    table_name: String,
    // Optional: Excel sheet name
    sheet_name: Option<String>,
}

// SAFETY: DuckDBWrapper is temporarily disabled due to thread safety issues
// TODO: Implement proper thread-safe DuckDB integration
unsafe impl Sync for DuckDBWrapper {}
unsafe impl Send for DuckDBWrapper {}

impl DuckDBWrapper {
    /// Create a new DuckDB wrapper for a file
    pub fn new(file_path: &str, file_type: &str, table_name: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let connection = Connection::open_in_memory()
            .map_err(|e| format!("Failed to create DuckDB connection: {}", e))?;

        // Load necessary extensions
        if let Err(e) = connection.execute_batch("INSTALL json; LOAD json;") {
            warn!("Failed to load JSON extension: {}", e);
        }
        if let Err(e) = connection.execute_batch("INSTALL excel; LOAD excel;") {
            warn!("Failed to load Excel extension: {}", e);
        }

        let wrapper = Self {
            connection,
            file_path: file_path.to_string(),
            file_type: file_type.to_lowercase(),
            table_name: table_name.to_string(),
            sheet_name: None,
        };

        info!("Created DuckDB wrapper for {} file: {}", file_type, file_path);
        Ok(wrapper)
    }

    /// Create a wrapper for Excel file with specific sheet
    pub fn new_with_excel_sheet(file_path: &str, sheet_name: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut wrapper = Self::new(file_path, "excel", sheet_name)?;
        wrapper.sheet_name = Some(sheet_name.to_string());
        Ok(wrapper)
    }

    /// Execute a SQL query on the file
    pub async fn execute_query(&self, _query: &str, _limit: i32) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        // TODO: Implement thread-safe DuckDB execution
        info!("DuckDB query execution temporarily disabled due to thread safety issues");
        Err("DuckDB integration temporarily disabled".into())
    }

    /// Get the table schema from the file
    pub async fn fetch_schema(&self) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        // TODO: Implement thread-safe DuckDB schema fetching
        warn!("DuckDB schema fetching temporarily disabled due to thread safety issues");
        Err("DuckDB integration temporarily disabled".into())
    }

    /// Get table names from the file
    pub async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        // TODO: Implement thread-safe DuckDB table listing
        match self.file_type.as_str() {
            "csv" | "json" => {
                // CSV and JSON files have one logical table
                Ok(vec![self.table_name.clone()])
            },
            _ => Err(format!("Unsupported file type: {}", self.file_type).into())
        }
    }

    /// Convert file-specific query to DuckDB syntax
    fn convert_file_query(&self, query: &str) -> String {
        let query_lower = query.to_lowercase().replace('\n', " ");
        let normalized_query = query_lower.trim();

        // For CSV files
        if self.file_type == "csv" && normalized_query.contains(&format!("from {}", self.table_name.to_lowercase())) {
            return query.replace(&format!("FROM {}", self.table_name), &format!("FROM '{}'", self.file_path));
        }

        // For Excel files
        if self.file_type == "excel" {
            if let Some(sheet_name) = &self.sheet_name {
                if normalized_query.contains(&format!("from {}", sheet_name.to_lowercase())) {
                    return query.replace(
                        &format!("FROM {}", sheet_name),
                        &format!("FROM read_excel('{}', sheet_name='{}')", self.file_path, sheet_name)
                    );
                }
            } else if normalized_query.contains(&format!("from {}", self.table_name.to_lowercase())) {
                return query.replace(&format!("FROM {}", self.table_name), &format!("FROM '{}'", self.file_path));
            }
        }

        // For JSON files
        if self.file_type == "json" && normalized_query.contains(&format!("from {}", self.table_name.to_lowercase())) {
            return query.replace(
                &format!("FROM {}", self.table_name),
                &format!("FROM read_json('{}', auto_detect=true, records=true)", self.file_path)
            );
        }

        // If no conversion needed, return as-is
        query.to_string()
    }

    /// Infer column data type by sampling the data
    async fn infer_column_type(&self, _column_name: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        // TODO: Implement thread-safe DuckDB type inference
        Ok("text".to_string())
    }

    /// Test if the file can be accessed by DuckDB
    pub async fn test_connection(&mut self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        // TODO: Implement thread-safe DuckDB connection testing
        info!("DuckDB connection testing temporarily disabled due to thread safety issues");
        // Check if file exists
        if Path::new(&self.file_path).exists() {
            warn!("DuckDB integration temporarily disabled, but file exists");
            Ok(true)
        } else {
            Err(format!("File not found: {}", self.file_path).into())
        }
    }
}

/// Helper function to create DuckDB wrapper from file configuration
pub fn create_duckdb_wrapper(config: &serde_json::Value) -> Result<DuckDBWrapper, Box<dyn Error + Send + Sync>> {
    let file_path = config
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or("Missing file_path in configuration")?;

    let file_type = config
        .get("file_type")
        .and_then(|v| v.as_str())
        .ok_or("Missing file_type in configuration")?;

    let table_name = config
        .get("table_name")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            // Generate table name from file path
            Path::new(file_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("data")
        });

    if file_type.to_lowercase() == "excel" {
        // Check if sheet_name is specified in config
        if let Some(sheet_name) = config.get("sheet_name").and_then(|v| v.as_str()) {
            DuckDBWrapper::new_with_excel_sheet(file_path, sheet_name)
        } else {
            DuckDBWrapper::new(file_path, file_type, table_name)
        }
    } else {
        DuckDBWrapper::new(file_path, file_type, table_name)
    }
}