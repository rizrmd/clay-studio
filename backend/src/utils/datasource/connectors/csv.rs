use super::super::core::base::{format_bytes, DataSourceConnector};
use super::duckdb_wrapper::{create_duckdb_wrapper, DuckDBWrapper};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufRead, Read};
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error, info};
use csv::ReaderBuilder;

pub struct CsvConnector {
    file_path: String,
    delimiter: u8,
    has_header: bool,
    encoding: String,
    skip_rows: usize,
    quote_char: u8,
    flexible: bool,
    schema_cache: Option<Value>,
    file_metadata: FileMetadata,
    duckdb_wrapper: Option<DuckDBWrapper>,
    use_duckdb: bool,
}

#[derive(Debug, Clone)]
struct FileMetadata {
    size_bytes: u64,
    last_modified: chrono::DateTime<chrono::Utc>,
    column_count: usize,
    estimated_row_count: Option<usize>,
}

impl CsvConnector {
    pub fn new(config: &Value) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let file_path = config
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or("Missing file_path in configuration")?;

        // Validate file exists
        if !Path::new(file_path).exists() {
            return Err(format!("File not found: {}", file_path).into());
        }

        // Parse configuration with defaults
        let delimiter = config
            .get("delimiter")
            .and_then(|v| v.as_str())
            .and_then(|s| s.chars().next())
            .unwrap_or(',') as u8;

        let has_header = config
            .get("has_header")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let encoding = config
            .get("encoding")
            .and_then(|v| v.as_str())
            .unwrap_or("utf-8")
            .to_string();

        let skip_rows = config
            .get("skip_rows")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let quote_char = config
            .get("quote_char")
            .and_then(|v| v.as_str())
            .and_then(|s| s.chars().next())
            .unwrap_or('"') as u8;

        let flexible = config
            .get("flexible")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        info!("CSV connector configured for file: {}", file_path);
        debug!("Delimiter: '{}' (ASCII {})", delimiter as char, delimiter);
        debug!("Has header: {}", has_header);
        debug!("Skip rows: {}", skip_rows);

        let mut connector = Self {
            file_path: file_path.to_string(),
            delimiter,
            has_header,
            encoding,
            skip_rows,
            quote_char,
            flexible,
            schema_cache: None,
            file_metadata: FileMetadata {
                size_bytes: 0,
                last_modified: chrono::Utc::now(),
                column_count: 0,
                estimated_row_count: None,
            },
        };

        // Load file metadata
        connector.load_file_metadata()?;

        // Initialize DuckDB wrapper for full SQL support
        connector.init_duckdb()?;

        Ok(connector)
    }

    fn load_file_metadata(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let path = Path::new(&self.file_path);
        let metadata = std::fs::metadata(path)?;

        self.file_metadata.size_bytes = metadata.len();
        self.file_metadata.last_modified = metadata.modified()?.into();

        // Quick scan to estimate structure
        let file = File::open(&self.file_path)?;
        let mut reader = BufReader::new(file);
        let mut buffer = String::new();
        let mut lines_read = 0;
        let mut sample_size = 0;

        // Read first few KB to estimate structure
        while reader.read_line(&mut buffer)? > 0 && sample_size < 10240 {
            sample_size += buffer.len();
            lines_read += 1;
            buffer.clear();
        }

        // Estimate column count from first data line
        if self.has_header {
            // Skip header line for column count estimation
            let file = File::open(&self.file_path)?;
            let mut reader = ReaderBuilder::new()
                .delimiter(self.delimiter)
                .has_headers(false)
                .from_reader(file);

            if let Some(Ok(record)) = reader.records().next() {
                self.file_metadata.column_count = record.len();
            }
        } else {
            // Use the first line as column reference
            let file = File::open(&self.file_path)?;
            let mut reader = ReaderBuilder::new()
                .delimiter(self.delimiter)
                .has_headers(false)
                .from_reader(file);

            if let Some(Ok(record)) = reader.records().next() {
                self.file_metadata.column_count = record.len();
            }
        }

        // Rough row count estimation based on file size and sample
        if lines_read > 0 {
            let avg_line_size = sample_size as f64 / lines_read as f64;
            let estimated_rows = (self.file_metadata.size_bytes as f64 / avg_line_size) as usize;
            self.file_metadata.estimated_row_count = Some(estimated_rows);
        }

        debug!("File metadata loaded: {} bytes, {} columns, ~{} rows",
               self.file_metadata.size_bytes,
               self.file_metadata.column_count,
               self.file_metadata.estimated_row_count.unwrap_or(0));

        Ok(())
    }

    fn create_reader(&self) -> Result<csv::Reader<File>, Box<dyn Error + Send + Sync>> {
        let file = File::open(&self.file_path)?;
        let mut reader = ReaderBuilder::new()
            .delimiter(self.delimiter)
            .quote(self.quote_char)
            .flexible(self.flexible)
            .has_headers(false)
            .from_reader(file);

        // Skip rows if configured
        for _ in 0..self.skip_rows {
            if !reader.records().next().is_some() {
                break;
            }
        }

        Ok(reader)
    }

    fn infer_data_type(values: &[String]) -> String {
        if values.is_empty() {
            return "text".to_string();
        }

        let mut int_count = 0;
        let mut float_count = 0;
        let mut bool_count = 0;
        let mut date_count = 0;
        let null_count = 0;
        let total_non_null = values.iter().filter(|v| !v.is_empty() && *v != "NULL" && *v != "null").count();

        for value in values {
            if value.is_empty() || value == "NULL" || value == "null" {
                continue;
            }

            // Check for boolean
            if value.to_lowercase() == "true" || value.to_lowercase() == "false" || value == "1" || value == "0" {
                bool_count += 1;
                continue;
            }

            // Check for integer
            if value.parse::<i64>().is_ok() {
                int_count += 1;
                continue;
            }

            // Check for float
            if value.parse::<f64>().is_ok() {
                float_count += 1;
                continue;
            }

            // Basic date detection (ISO format)
            if chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok() ||
               chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").is_ok() {
                date_count += 1;
                continue;
            }
        }

        // Determine type based on majority of non-null values
        if total_non_null > 0 {
            let int_ratio = int_count as f64 / total_non_null as f64;
            let float_ratio = float_count as f64 / total_non_null as f64;
            let bool_ratio = bool_count as f64 / total_non_null as f64;
            let date_ratio = date_count as f64 / total_non_null as f64;

            if date_ratio > 0.8 {
                "date".to_string()
            } else if bool_ratio > 0.8 {
                "boolean".to_string()
            } else if int_ratio > 0.8 {
                "integer".to_string()
            } else if float_ratio > 0.8 {
                "float".to_string()
            } else {
                "text".to_string()
            }
        } else {
            "text".to_string()
        }
    }

    async fn get_sample_data(&self, limit: usize) -> Result<(Vec<String>, Vec<Vec<String>>), Box<dyn Error + Send + Sync>> {
        let mut reader = self.create_reader()?;
        let mut headers = Vec::new();
        let mut rows = Vec::new();

        // Get headers if configured
        if self.has_header {
            if let Some(Ok(record)) = reader.records().next() {
                headers = record.iter().map(|s| s.to_string()).collect();
            }
        } else {
            // Generate default column names
            let first_record = reader.records().next();
            if let Some(Ok(record)) = first_record {
                for i in 0..record.len() {
                    headers.push(format!("column_{}", i + 1));
                }
            }
        }

        // Collect sample data
        for (index, result) in reader.records().enumerate() {
            if index >= limit {
                break;
            }

            if let Ok(record) = result {
                let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
                rows.push(row);
            }
        }

        Ok((headers, rows))
    }

    /// Initialize DuckDB wrapper for SQL support
    fn init_duckdb(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Try to initialize DuckDB wrapper
        let table_name = Path::new(&self.file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("csv_data");

        let config = json!({
            "file_path": self.file_path,
            "file_type": "csv",
            "table_name": table_name,
            "delimiter": self.delimiter as char,
            "has_header": self.has_header,
            "encoding": self.encoding,
            "skip_rows": self.skip_rows,
            "quote_char": self.quote_char as char,
            "flexible": self.flexible
        });

        match create_duckdb_wrapper(&config) {
            Ok(wrapper) => {
                info!("DuckDB wrapper initialized for CSV file: {}", self.file_path);
                self.duckdb_wrapper = Some(wrapper);
                self.use_duckdb = true;
                Ok(())
            },
            Err(e) => {
                warn!("Failed to initialize DuckDB wrapper for CSV: {}. Falling back to basic CSV operations.", e);
                self.duckdb_wrapper = None;
                self.use_duckdb = false;
                Ok(())
            }
        }
    }
}

#[async_trait]
impl DataSourceConnector for CsvConnector {
    async fn test_connection(&mut self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        info!("Testing CSV file connection: {}", self.file_path);

        // First try DuckDB if available
        if self.use_duckdb {
            if let Some(ref mut wrapper) = self.duckdb_wrapper {
                match wrapper.test_connection().await {
                    Ok(success) => {
                        if success {
                            info!("DuckDB CSV connection test successful: {}", self.file_path);
                        } else {
                            info!("DuckDB CSV connection test returned false: {}", self.file_path);
                        }
                        return Ok(success);
                    },
                    Err(e) => {
                        warn!("DuckDB CSV connection test failed: {}. Falling back to basic CSV operations.", e);
                        // Fall back to basic CSV testing
                    }
                }
            }
        }

        // Fallback to basic CSV testing
        match self.create_reader() {
            Ok(mut reader) => {
                // Try to read first record
                match reader.records().next() {
                    Some(Ok(_)) => {
                        info!("CSV file test successful: {}", self.file_path);
                        Ok(true)
                    },
                    Some(Err(e)) => {
                        error!("CSV file test failed - parsing error: {}", e);
                        Err(format!("Failed to parse CSV file: {}", e).into())
                    },
                    None => {
                        info!("CSV file is empty: {}", self.file_path);
                        Ok(true) // Empty file is still a valid connection
                    }
                }
            },
            Err(e) => {
                error!("CSV file test failed - cannot open file: {}", e);
                Err(e)
            }
        }
    }

    async fn execute_query(&self, query: &str, limit: i32) -> Result<Value, Box<dyn Error + Send + Sync>> {
        // Use DuckDB for full SQL support if available
        if self.use_duckdb {
            if let Some(ref mut wrapper) = self.duckdb_wrapper {
                debug!("Executing CSV query with DuckDB: {}", query);
                match wrapper.execute_query(query, limit).await {
                    Ok(result) => {
                        info!("DuckDB CSV query executed successfully: {} rows", result["row_count"].as_u64().unwrap_or(0));
                        return Ok(result);
                    },
                    Err(e) => {
                        warn!("DuckDB CSV query failed: {}. Falling back to basic CSV operations.", e);
                        // Fall back to basic CSV execution
                    }
                }
            }
        }

        // Fallback to basic CSV functionality (limited SELECT * queries)
        let filename = Path::new(&self.file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("data");

        let normalized_query = query.to_lowercase().replace('\n', " ").trim().to_string();

        if !normalized_query.starts_with("select") || !normalized_query.contains(&filename.to_lowercase()) {
            return Err("Only basic SELECT * queries are supported for CSV files. Consider upgrading to DuckDB for full SQL support.".into());
        }

        let start = Instant::now();
        let (headers, rows) = self.get_sample_data(limit as usize).await?;
        let execution_time_ms = start.elapsed().as_millis() as i64;

        if rows.is_empty() {
            return Ok(json!({
                "columns": headers,
                "rows": [],
                "row_count": 0,
                "execution_time_ms": execution_time_ms,
                "query": query,
                "note": "Limited functionality - upgrade to DuckDB for full SQL support"
            }));
        }

        Ok(json!({
            "columns": headers,
            "rows": rows,
            "row_count": rows.len(),
            "execution_time_ms": execution_time_ms,
            "query": query,
            "note": "Limited functionality - upgrade to DuckDB for full SQL support"
        }))
    }

    async fn get_table_data_with_pagination(
        &self,
        table_name: &str,
        page: i32,
        limit: i32,
        sort_column: Option<&str>,
        sort_direction: Option<&str>
    ) -> Result<Value, Box<dyn Error + Send + Sync>> {
        // Validate table name matches filename
        let filename = Path::new(&self.file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("data");

        if table_name != filename {
            return Err(format!("Table '{}' not found. Available table: '{}'", table_name, filename).into());
        }

        let start = Instant::now();
        let offset = (page - 1) * limit;

        // For CSV, we need to read all data and then paginate
        let (headers, all_rows) = self.get_sample_data(usize::MAX).await?;

        // Apply sorting if specified
        let mut rows = all_rows;
        if let Some(sort_col) = sort_column {
            if let Some(col_index) = headers.iter().position(|h| h == sort_col) {
                let direction = sort_direction.unwrap_or("ASC");
                let default_str = "".to_string();
                rows.sort_by(|a, b| {
                    let a_val = a.get(col_index).unwrap_or(&default_str);
                    let b_val = b.get(col_index).unwrap_or(&default_str);

                    if direction == "DESC" {
                        b_val.cmp(a_val)
                    } else {
                        a_val.cmp(b_val)
                    }
                });
            }
        }

        // Apply pagination
        let total_rows = rows.len();
        let start_idx = offset as usize;
        let end_idx = std::cmp::min(start_idx + limit as usize, total_rows);

        let paginated_rows = if start_idx < total_rows {
            rows[start_idx..end_idx].to_vec()
        } else {
            Vec::new()
        };

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(json!({
            "columns": headers,
            "rows": paginated_rows,
            "row_count": paginated_rows.len(),
            "total_rows": total_rows,
            "execution_time_ms": execution_time_ms,
            "page": page,
            "page_size": limit
        }))
    }

    async fn fetch_schema(&self) -> Result<Value, Box<dyn Error + Send + Sync>> {
        if let Some(cached_schema) = &self.schema_cache {
            return Ok(cached_schema.clone());
        }

        // Use DuckDB for better schema analysis if available
        if self.use_duckdb {
            if let Some(ref wrapper) = self.duckdb_wrapper {
                match wrapper.fetch_schema().await {
                    Ok(schema) => {
                        info!("DuckDB CSV schema fetched successfully");
                        self.schema_cache = Some(schema.clone());
                        return Ok(schema);
                    },
                    Err(e) => {
                        warn!("DuckDB CSV schema fetch failed: {}. Falling back to basic CSV schema.", e);
                        // Fall back to basic CSV schema
                    }
                }
            }
        }

        // Fallback to basic CSV schema
        let start = Instant::now();
        let (headers, sample_rows) = self.get_sample_data(1000).await?;

        let mut columns = Vec::new();

        for (col_index, col_name) in headers.iter().enumerate() {
            let mut values = Vec::new();

            for row in &sample_rows {
                if let Some(value) = row.get(col_index) {
                    values.push(value.clone());
                }
            }

            let data_type = Self::infer_data_type(&values);
            let nullable = values.iter().filter(|v| v.is_empty() || *v == "NULL" || *v == "null").count() > 0;

            columns.push(json!({
                "column_name": col_name,
                "data_type": data_type,
                "is_nullable": if nullable { "YES" } else { "NO" },
                "sample_values": values.iter().take(5).cloned().collect::<Vec<_>>()
            }));
        }

        let filename = Path::new(&self.file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("data");

        let schema = json!({
            "database_schema": "file",
            "tables": {
                filename: columns
            },
            "file_metadata": {
                "size_bytes": self.file_metadata.size_bytes,
                "size_human": format_bytes(self.file_metadata.size_bytes),
                "last_modified": self.file_metadata.last_modified.to_rfc3339(),
                "estimated_rows": self.file_metadata.estimated_row_count
            },
            "refreshed_at": chrono::Utc::now().to_rfc3339(),
            "generation_time_ms": start.elapsed().as_millis(),
            "sql_support": if self.use_duckdb { "full" } else { "basic" }
        });

        Ok(schema)
    }

    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        let filename = Path::new(&self.file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("data");

        Ok(vec![filename.to_string()])
    }

    async fn analyze_database(&self) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let filename = Path::new(&self.file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("data");

        let schema = self.fetch_schema().await?;

        Ok(json!({
            "database_schema": "file",
            "statistics": {
                "table_count": 1,
                "total_size": self.file_metadata.size_bytes,
                "total_size_human": format_bytes(self.file_metadata.size_bytes),
                "total_rows": self.file_metadata.estimated_row_count.unwrap_or(0),
            },
            "table_names": vec![filename],
            "key_tables": [{
                "name": filename,
                "size_bytes": self.file_metadata.size_bytes,
                "connections": 0,
            }],
            "largest_tables": [{
                "name": filename,
                "size_bytes": self.file_metadata.size_bytes,
                "size_human": format_bytes(self.file_metadata.size_bytes),
            }],
            "analyzed_at": chrono::Utc::now().to_rfc3339(),
        }))
    }

    async fn get_tables_schema(&self, tables: Vec<&str>) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let filename = Path::new(&self.file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("data");

        let mut result = json!({});

        for table_name in tables {
            if table_name != filename {
                continue;
            }

            let schema = self.fetch_schema().await?;
            if let Some(tables_data) = schema.get("tables") {
                if let Some(table_data) = tables_data.get(table_name) {
                    let (headers, sample_rows) = self.get_sample_data(5).await?;

                    result[table_name] = json!({
                        "columns": table_data,
                        "primary_keys": [],
                        "foreign_keys": [],
                        "row_count": self.file_metadata.estimated_row_count.unwrap_or(0),
                        "sample_data": sample_rows.iter().enumerate().map(|(i, row)| {
                            let mut row_obj = json!({});
                            for (j, val) in row.iter().enumerate() {
                                if let Some(col_name) = headers.get(j) {
                                    row_obj[col_name] = json!(val);
                                }
                            }
                            row_obj
                        }).collect::<Vec<_>>(),
                    });
                }
            }
        }

        Ok(result)
    }

    async fn search_tables(&self, pattern: &str) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let filename = Path::new(&self.file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("data");

        let matches = if filename.to_lowercase().contains(&pattern.to_lowercase()) {
            vec![json!({
                "name": filename,
                "description": Some("CSV data file"),
                "column_count": self.file_metadata.column_count,
            })]
        } else {
            Vec::new()
        };

        Ok(json!({
            "matches": matches,
            "total_matches": matches.len(),
        }))
    }

    async fn get_related_tables(&self, table: &str) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let filename = Path::new(&self.file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("data");

        if table != filename {
            return Err("Table not found".into());
        }

        let table_schema = self.get_tables_schema(vec![table]).await?;

        Ok(json!({
            "main_table": table_schema.get(table),
            "related_tables": {},
            "relationship_count": 0,
        }))
    }

    async fn get_database_stats(&self) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let filename = Path::new(&self.file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("data");

        Ok(json!({
            "summary": {
                "total_tables": 1,
                "total_size_bytes": self.file_metadata.size_bytes,
                "total_size_human": format_bytes(self.file_metadata.size_bytes),
            },
            "largest_tables": [{
                "name": filename,
                "size_bytes": self.file_metadata.size_bytes,
                "size_human": format_bytes(self.file_metadata.size_bytes),
            }],
            "most_connected_tables": [],
        }))
    }
}