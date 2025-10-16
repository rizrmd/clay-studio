//! DuckDB wrapper for file datasource queries
//! Provides SQL support for CSV, Excel, and JSON files using DuckDB's direct file reading capabilities

use duckdb::{Connection, params};
use serde_json::json;
use std::error::Error;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error, info, warn};
use chrono;

/// DuckDB wrapper for file datasources
pub struct DuckDBWrapper {
    connection: Connection,
    file_path: String,
    file_type: String,
    table_name: String,
    // Optional: Excel sheet name
    sheet_name: Option<String>,
}

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
    pub async fn execute_query(&self, query: &str, limit: i32) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        let start = Instant::now();

        debug!("Executing DuckDB query: {}", query);

        let adjusted_query = if limit > 0 && !query.to_lowercase().contains("limit") {
            format!("{} LIMIT {}", query, limit)
        } else {
            query.to_string()
        };

        let mut stmt = self.connection
            .prepare(&self.convert_file_query(&adjusted_query))
            .map_err(|e| format!("Failed to prepare DuckDB query: {}", e))?;

        let mut rows = Vec::new();
        let mut columns = Vec::new();

        // Execute query and collect results
        while let Ok(row) = stmt.next() {
            if columns.is_empty() {
                // Get column names
                for i in 0..row.len() {
                    columns.push(row.column_name(i).unwrap_or_else(|| format!("column_{}", i + 1)));
                }
            }

            let mut row_data = Vec::new();
            for i in 0..row.len() {
                let value: duckdb::Value = row.get(i)?;
                let json_value = match value {
                    duckdb::Value::Null => serde_json::Value::Null,
                    duckdb::Value::Boolean(b) => serde_json::Value::Bool(b),
                    duckdb::Value::TinyInt(i) => serde_json::Value::Number(serde_json::Number::from(i)),
                    duckdb::Value::SmallInt(i) => serde_json::Value::Number(serde_json::Number::from(i)),
                    duckdb::Value::Int(i) => serde_json::Value::Number(serde_json::Number::from(i)),
                    duckdb::Value::BigInt(i) => serde_json::Value::Number(serde_json::Number::from(i)),
                    duckdb::Value::UTinyInt(i) => serde_json::Value::Number(serde_json::Number::from(i)),
                    duckdb::Value::USmallInt(i) => serde_json::Value::Number(serde_json::Number::from(i)),
                    duckdb::Value::UInt(i) => serde_json::Value::Number(serde_json::Number::from(i)),
                    duckdb::Value::UBigInt(i) => serde_json::Value::String(i.to_string()),
                    duckdb::Value::Float(f) => serde_json::Value::Number(serde_json::Number::from_f64(f)),
                    duckdb::Value::Double(d) => serde_json::Value::Number(serde_json::Number::from_f64(d)),
                    duckdb::Value::Text(s) => serde_json::Value::String(s),
                    duckdb::Value::Blob(_) => serde_json::Value::String("[BLOB]".to_string()),
                    duckdb::Value::Date(d) => serde_json::Value::String(d.to_string()),
                    duckdb::Value::Time(t) => serde_json::Value::String(t.to_string()),
                    duckdb::Value::Timestamp(ts) => serde_json::Value::String(ts.to_string()),
                    duckdb::Value::Interval(_) => serde_json::Value::String("[INTERVAL]".to_string()),
                    duckdb::Value::HugeInt(_) => serde_json::Value::String("[HUGEINT]".to_string()),
                    duckdb::Value::Decimal(_) => serde_json::Value::String("[DECIMAL]".to_string()),
                    duckdb::Value::Uuid(_) => serde_json::Value::String("[UUID]".to_string()),
                };
                row_data.push(json_value);
            }
            rows.push(row_data);
        }

        let execution_time_ms = start.elapsed().as_millis() as i64;
        let row_count = rows.len();

        debug!("DuckDB query executed successfully: {} rows in {}ms", row_count, execution_time_ms);

        Ok(json!({
            "columns": columns,
            "rows": rows,
            "row_count": row_count,
            "execution_time_ms": execution_time_ms,
            "query": query,
            "file_type": self.file_type,
            "file_path": self.file_path
        }))
    }

    /// Get the table schema from the file
    pub async fn fetch_schema(&self) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        let start = Instant::now();

        let query = match self.file_type.as_str() {
            "excel" => {
                if let Some(sheet) = &self.sheet_name {
                    format!("SELECT * FROM read_excel('{}', sheet_name='{}') LIMIT 1", self.file_path, sheet)
                } else {
                    format!("SELECT * FROM '{}' LIMIT 1", self.file_path)
                }
            },
            "json" => {
                format!("SELECT * FROM read_json('{}', auto_detect=true, records=true) LIMIT 1", self.file_path)
            },
            "csv" => {
                format!("SELECT * FROM '{}' LIMIT 1", self.file_path)
            },
            _ => {
                return Err(format!("Unsupported file type for DuckDB: {}", self.file_type).into());
            }
        };

        let mut stmt = self.connection
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare schema query: {}", e))?;

        let mut columns = Vec::new();

        if let Ok(row) = stmt.next() {
            for i in 0..row.len() {
                let column_name = row.column_name(i).unwrap_or_else(|| format!("column_{}", i + 1));

                // Infer data type by sampling more data
                let inferred_type = self.infer_column_type(&column_name).await;

                columns.push(json!({
                    "column_name": column_name,
                    "data_type": inferred_type,
                    "is_nullable": "YES",
                    "sample_values": vec!["sample_value"] // Could be enhanced later
                }));
            }
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(json!({
            "database_schema": self.file_type,
            "tables": {
                self.table_name: columns
            },
            "file_metadata": {
                "file_path": self.file_path,
                "file_type": self.file_type,
                "sheet_name": self.sheet_name
            },
            "refreshed_at": chrono::Utc::now().to_rfc3339(),
            "generation_time_ms": execution_time_ms
        }))
    }

    /// Get table names from the file
    pub async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        match self.file_type.as_str() {
            "excel" => {
                // For Excel, we can use DuckDB to list sheets
                let query = format!("PRAGMA table_info('{}')", self.file_path);
                if let Ok(_) = self.connection.prepare(&query) {
                    // Try to query sheets
                    if let Some(sheet) = &self.sheet_name {
                        Ok(vec![sheet.clone()])
                    } else {
                        // Default to filename as table name
                        Ok(vec![self.table_name.clone()])
                    }
                } else {
                    Ok(vec![self.table_name.clone()])
                }
            },
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
        if self.file_type == "csv" {
            if normalized_query.contains(&format!("from {}", self.table_name.to_lowercase())) {
                return query.replace(&format!("FROM {}", self.table_name), &format!("FROM '{}'", self.file_path));
            }
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
            } else {
                if normalized_query.contains(&format!("from {}", self.table_name.to_lowercase())) {
                    return query.replace(&format!("FROM {}", self.table_name), &format!("FROM '{}'", self.file_path));
                }
            }
        }

        // For JSON files
        if self.file_type == "json" {
            if normalized_query.contains(&format!("from {}", self.table_name.to_lowercase())) {
                return query.replace(
                    &format!("FROM {}", self.table_name),
                    &format!("FROM read_json('{}', auto_detect=true, records=true)", self.file_path)
                );
            }
        }

        // If no conversion needed, return as-is
        query.to_string()
    }

    /// Infer column data type by sampling the data
    async fn infer_column_type(&self, column_name: &str) -> String {
        let query = format!(
            "SELECT {} FROM {} LIMIT 100",
            column_name,
            self.convert_file_query(&format!("SELECT * FROM {}", self.table_name))
        );

        if let Ok(mut stmt) = self.connection.prepare(&query) {
            let mut int_count = 0;
            let mut float_count = 0;
            let mut bool_count = 0;
            let mut date_count = 0;
            let mut text_count = 0;
            let mut total_count = 0;

            while let Ok(row) = stmt.next() {
                let value: duckdb::Value = row.get(0).unwrap_or(duckdb::Value::Text("".to_string()));

                total_count += 1;

                match value {
                    duckdb::Value::Text(s) => {
                        if s.to_lowercase() == "true" || s.to_lowercase() == "false" || s == "1" || s == "0" {
                            bool_count += 1;
                        } else if s.parse::<i64>().is_ok() {
                            int_count += 1;
                        } else if s.parse::<f64>().is_ok() {
                            float_count += 1;
                        } else if chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").is_ok() ||
                                  chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").is_ok() {
                            date_count += 1;
                        } else {
                            text_count += 1;
                        }
                    },
                    duckdb::Value::Boolean(_) => bool_count += 1,
                    duckdb::Value::Int(_) | duckdb::Value::BigInt(_) => int_count += 1,
                    duckdb::Value::Float(_) | duckdb::Value::Double(_) => float_count += 1,
                    duckdb::Value::Date(_) | duckdb::Value::Timestamp(_) => date_count += 1,
                    _ => text_count += 1,
                }

                if total_count >= 100 {
                    break;
                }
            }

            // Determine dominant type
            if total_count > 0 {
                if date_count as f64 / total_count as f64 > 0.7 {
                    "date".to_string()
                } else if bool_count as f64 / total_count as f64 > 0.7 {
                    "boolean".to_string()
                } else if int_count as f64 / total_count as f64 > 0.7 {
                    "integer".to_string()
                } else if float_count as f64 / total_count as f64 > 0.7 {
                    "float".to_string()
                } else {
                    "text".to_string()
                }
            } else {
                "text".to_string()
            }
        } else {
            "text".to_string()
        }
    }

    /// Test if the file can be accessed by DuckDB
    pub async fn test_connection(&mut self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        info!("Testing DuckDB connection to file: {}", self.file_path);

        // Check if file exists
        if !Path::new(&self.file_path).exists() {
            return Err(format!("File not found: {}", self.file_path).into());
        }

        // Try a simple query to test if DuckDB can read the file
        let test_query = match self.file_type.as_str() {
            "excel" => {
                if let Some(sheet) = &self.sheet_name {
                    format!("SELECT COUNT(*) FROM read_excel('{}', sheet_name='{}')", self.file_path, sheet)
                } else {
                    format!("SELECT COUNT(*) FROM '{}'", self.file_path)
                }
            },
            "json" => {
                format!("SELECT COUNT(*) FROM read_json('{}', auto_detect=true, records=true)", self.file_path)
            },
            "csv" => {
                format!("SELECT COUNT(*) FROM '{}'", self.file_path)
            },
            _ => {
                return Err(format!("Unsupported file type for DuckDB test: {}", self.file_type).into());
            }
        };

        match self.connection.prepare(&test_query) {
            Ok(mut stmt) => {
                match stmt.next() {
                    Ok(Some(row)) => {
                        let count: i64 = row.get(0).unwrap_or(0);
                        info!("DuckDB file test successful: {} records found", count);
                        Ok(true)
                    },
                    Ok(None) => {
                        warn!("DuckDB file test: no data found in file");
                        Ok(true) // Empty file is still valid
                    },
                    Err(e) => {
                        error!("DuckDB file test failed: {}", e);
                        Err(format!("Failed to read file with DuckDB: {}", e).into())
                    }
                }
            },
            Err(e) => {
                error!("DuckDB file test failed: {}", e);
                Err(format!("Failed to prepare test query: {}", e).into())
            }
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