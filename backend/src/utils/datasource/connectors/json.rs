use super::super::core::base::{format_bytes, DataSourceConnector};
use super::duckdb_wrapper::{DuckDBWrapper, create_duckdb_wrapper};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error, info, warn};

pub struct JsonConnector {
    file_path: String,
    root_path: Option<String>,
    array_path: Option<String>,
    schema_cache: Option<Value>,
    file_metadata: JsonFileMetadata,
    // DuckDB integration for full SQL support
    duckdb_wrapper: Option<DuckDBWrapper>,
    use_duckdb: bool,
}

#[derive(Debug, Clone)]
struct JsonFileMetadata {
    size_bytes: u64,
    last_modified: chrono::DateTime<chrono::Utc>,
    total_objects: usize,
    root_structure: JsonStructure,
}

#[derive(Debug, Clone)]
enum JsonStructure {
    Array,
    Object { key_count: usize },
    Mixed,
    Invalid,
}

#[derive(Debug, Clone)]
struct JsonPath {
    segments: Vec<String>,
}

impl JsonPath {
    fn new(path: &str) -> Self {
        let segments = if path.is_empty() {
            Vec::new()
        } else {
            path.split('.').map(|s| s.to_string()).collect()
        };
        Self { segments }
    }

    fn navigate(&self, value: &Value) -> Option<Value> {
        let mut current = value;

        for segment in &self.segments {
            current = match current {
                Value::Object(map) => map.get(segment)?,
                Value::Array(arr) => {
                    if let Ok(index) = segment.parse::<usize>() {
                        arr.get(index)?
                    } else {
                        return None;
                    }
                },
                _ => return None,
            };
        }

        Some(current.clone())
    }
}

impl JsonConnector {
    pub fn new(config: &Value) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let file_path = config
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or("Missing file_path in configuration")?;

        // Validate file exists and is a JSON file
        if !Path::new(file_path).exists() {
            return Err(format!("File not found: {}", file_path).into());
        }

        let extension = Path::new(file_path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        if extension != "json" && extension != "jsonl" {
            return Err(format!("File must be a JSON file (.json, .jsonl): {}", file_path).into());
        }

        let root_path = config
            .get("root_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let array_path = config
            .get("array_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        info!("JSON connector configured for file: {}", file_path);
        debug!("Root path: {:?}", root_path);
        debug!("Array path: {:?}", array_path);

        let mut connector = Self {
            file_path: file_path.to_string(),
            root_path: root_path.clone(),
            array_path: array_path.clone(),
            schema_cache: None,
            file_metadata: JsonFileMetadata {
                size_bytes: 0,
                last_modified: chrono::Utc::now(),
                total_objects: 0,
                root_structure: JsonStructure::Invalid,
            },
            duckdb_wrapper: None,
            use_duckdb: false,
        };

        // Load file metadata
        connector.load_file_metadata()?;

        // Initialize DuckDB wrapper for SQL support
        let use_duckdb = config.get("use_duckdb").and_then(|v| v.as_bool()).unwrap_or(true);
        if use_duckdb {
            match create_duckdb_wrapper(&json!({
                "file_path": file_path,
                "file_type": "json",
                "table_name": "data"
            })) {
                Ok(duckdb) => {
                    info!("DuckDB wrapper initialized for JSON file");
                    connector.duckdb_wrapper = Some(duckdb);
                    connector.use_duckdb = true;
                },
                Err(e) => {
                    warn!("Failed to initialize DuckDB wrapper for JSON file: {}, falling back to basic operations", e);
                    connector.use_duckdb = false;
                }
            }
        }

        Ok(connector)
    }

    fn load_file_metadata(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let path = Path::new(&self.file_path);
        let metadata = std::fs::metadata(path)?;

        self.file_metadata.size_bytes = metadata.len();
        self.file_metadata.last_modified = metadata.modified()?.into();

        // Load and analyze JSON structure
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        let json_value: Value = serde_json::from_reader(reader)?;

        // Determine root structure
        self.file_metadata.root_structure = match &json_value {
            Value::Array(arr) => {
                self.file_metadata.total_objects = arr.len();
                JsonStructure::Array
            },
            Value::Object(obj) => {
                self.file_metadata.root_structure = JsonStructure::Object {
                    key_count: obj.len()
                };

                // Try to find arrays within the object
                self.file_metadata.total_objects = self.count_objects_in_value(&json_value);
                JsonStructure::Object { key_count: obj.len() }
            },
            _ => JsonStructure::Invalid,
        };

        debug!("JSON file metadata loaded: {} bytes, {} objects, structure: {:?}",
               self.file_metadata.size_bytes,
               self.file_metadata.total_objects,
               self.file_metadata.root_structure);

        Ok(())
    }

    fn count_objects_in_value(&self, value: &Value) -> usize {
        match value {
            Value::Array(arr) => arr.len(),
            Value::Object(obj) => {
                obj.values()
                    .map(|v| self.count_objects_in_value(v))
                    .sum()
            },
            _ => 0,
        }
    }

    fn get_data_array(&self) -> Result<Vec<Value>, Box<dyn Error + Send + Sync>> {
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        let json_value: Value = serde_json::from_reader(reader)?;

        // Navigate to the specified path if provided
        let target_value = if let Some(array_path) = &self.array_path {
            let path = JsonPath::new(array_path);
            path.navigate(&json_value)
                .ok_or_else(|| format!("Path '{}' not found in JSON", array_path))?
        } else if let Some(root_path) = &self.root_path {
            let path = JsonPath::new(root_path);
            path.navigate(&json_value)
                .ok_or_else(|| format!("Path '{}' not found in JSON", root_path))?
        } else {
            json_value
        };

        // Extract array from the target value
        match target_value {
            Value::Array(arr) => Ok(arr.clone()),
            Value::Object(obj) => {
                // If it's an object, wrap it in an array
                Ok(vec![obj.clone()])
            },
            _ => {
                // If it's a primitive value, create objects with a "value" field
                Ok(vec![json!({ "value": target_value })])
            }
        }
    }

    fn flatten_json_object(&self, obj: &Value, prefix: &str) -> Vec<(String, String)> {
        let mut result = Vec::new();

        match obj {
            Value::Object(map) => {
                for (key, value) in map {
                    let new_key = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };

                    match value {
                        Value::Object(_) | Value::Array(_) => {
                            result.extend(self.flatten_json_object(value, &new_key));
                        },
                        _ => {
                            let value_str = match value {
                                Value::Null => "NULL".to_string(),
                                Value::String(s) => s.clone(),
                                Value::Number(n) => n.to_string(),
                                Value::Bool(b) => b.to_string(),
                                _ => value.to_string(),
                            };
                            result.push((new_key, value_str));
                        }
                    }
                }
            },
            Value::Array(arr) => {
                for (i, item) in arr.iter().enumerate() {
                    let new_key = format!("{}[{}]", prefix, i);
                    match item {
                        Value::Object(_) | Value::Array(_) => {
                            result.extend(self.flatten_json_object(item, &new_key));
                        },
                        _ => {
                            let value_str = match item {
                                Value::Null => "NULL".to_string(),
                                Value::String(s) => s.clone(),
                                Value::Number(n) => n.to_string(),
                                Value::Bool(b) => b.to_string(),
                                _ => item.to_string(),
                            };
                            result.push((new_key, value_str));
                        }
                    }
                }
            },
            _ => {
                let value_str = match obj {
                    Value::Null => "NULL".to_string(),
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => obj.to_string(),
                };
                result.push((prefix.to_string(), value_str));
            }
        }

        result
    }

    fn infer_data_type(values: &[String]) -> String {
        if values.is_empty() {
            return "text".to_string();
        }

        let mut int_count = 0;
        let mut float_count = 0;
        let mut bool_count = 0;
        let mut date_count = 0;
        let mut total_non_null = values.iter().filter(|v| !v.is_empty() && *v != "NULL").count();

        for value in values {
            if value.is_empty() || value == "NULL" {
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

    fn get_table_name(&self) -> String {
        Path::new(&self.file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("json_data")
            .to_string()
    }
}

#[async_trait]
impl DataSourceConnector for JsonConnector {
    async fn test_connection(&mut self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        info!("Testing JSON file connection: {}", self.file_path);

        match self.get_data_array() {
            Ok(data) => {
                info!("JSON file test successful: {} ({} objects)", self.file_path, data.len());
                Ok(true)
            },
            Err(e) => {
                error!("JSON file test failed: {}", e);
                Err(e)
            }
        }
    }

    async fn execute_query(&self, query: &str, limit: i32) -> Result<Value, Box<dyn Error + Send + Sync>> {
        // Use DuckDB for full SQL support if available
        if self.use_duckdb {
            if let Some(duckdb) = &self.duckdb_wrapper {
                match duckdb.execute_query(query, limit).await {
                    Ok(result) => {
                        debug!("DuckDB query executed successfully on JSON file");
                        return Ok(result);
                    },
                    Err(e) => {
                        warn!("DuckDB query failed on JSON file: {}, falling back to basic operations", e);
                        // Fall through to basic JSON operations
                    }
                }
            }
        }

        // Fallback to basic JSON operations for simple SELECT * queries
        let table_name = self.get_table_name();
        let normalized_query = query.to_lowercase().replace('\n', " ").trim().to_string();

        if !normalized_query.starts_with("select") || !normalized_query.contains(&table_name.to_lowercase()) {
            return Err(format!("Only basic SELECT * queries are supported for JSON files without DuckDB. Available table: '{}'", table_name).into());
        }

        let start = Instant::now();
        let data_array = self.get_data_array()?;

        // Apply limit
        let limited_data: Vec<_> = data_array.into_iter().take(limit as usize).collect();

        // Flatten JSON objects and get all possible keys
        let mut all_keys = std::collections::HashSet::new();
        let mut flattened_rows = Vec::new();

        for obj in &limited_data {
            let flattened = self.flatten_json_object(obj, "");
            for (key, _) in &flattened {
                all_keys.insert(key.clone());
            }
            flattened_rows.push(flattened);
        }

        // Convert to table format
        let mut headers: Vec<String> = all_keys.into_iter().collect();
        headers.sort();

        let mut rows = Vec::new();
        for flattened_row in &flattened_rows {
            let mut row = Vec::new();
            for header in &headers {
                let value = flattened_row.iter()
                    .find(|(k, _)| k == header)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_else(|| "NULL".to_string());
                row.push(value);
            }
            rows.push(row);
        }

        let execution_time_ms = start.elapsed().as_millis() as i64;

        Ok(json!({
            "columns": headers,
            "rows": rows,
            "row_count": rows.len(),
            "execution_time_ms": execution_time_ms,
            "query": query
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
        let expected_table_name = self.get_table_name();

        if table_name != expected_table_name {
            return Err(format!("Table '{}' not found. Available table: '{}'", table_name, expected_table_name).into());
        }

        let start = Instant::now();
        let offset = (page - 1) * limit;

        let data_array = self.get_data_array()?;

        // Flatten all objects
        let mut all_keys = std::collections::HashSet::new();
        let mut flattened_rows = Vec::new();

        for obj in &data_array {
            let flattened = self.flatten_json_object(obj, "");
            for (key, _) in &flattened {
                all_keys.insert(key.clone());
            }
            flattened_rows.push(flattened);
        }

        // Convert to table format
        let mut headers: Vec<String> = all_keys.into_iter().collect();
        headers.sort();

        let mut rows = Vec::new();
        for flattened_row in &flattened_rows {
            let mut row = Vec::new();
            for header in &headers {
                let value = flattened_row.iter()
                    .find(|(k, _)| k == header)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_else(|| "NULL".to_string());
                row.push(value);
            }
            rows.push(row);
        }

        // Apply sorting if specified
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

        let start = Instant::now();

        // Use DuckDB for schema if available
        if self.use_duckdb {
            if let Some(duckdb) = &self.duckdb_wrapper {
                match duckdb.fetch_schema().await {
                    Ok(mut duckdb_schema) => {
                        // Enhance DuckDB schema with JSON-specific metadata
                        duckdb_schema["file_metadata"] = json!({
                            "size_bytes": self.file_metadata.size_bytes,
                            "size_human": format_bytes(self.file_metadata.size_bytes),
                            "last_modified": self.file_metadata.last_modified.to_rfc3339(),
                            "total_objects": self.file_metadata.total_objects,
                            "root_structure": format!("{:?}", self.file_metadata.root_structure),
                            "root_path": self.root_path,
                            "array_path": self.array_path
                        });

                        debug!("DuckDB schema fetched successfully for JSON file");
                        return Ok(duckdb_schema);
                    },
                    Err(e) => {
                        warn!("Failed to fetch schema using DuckDB for JSON file: {}, falling back to basic operations", e);
                        // Fall through to basic JSON operations
                    }
                }
            }
        }

        // Fallback to basic JSON operations
        let data_array = self.get_data_array()?;
        let sample_data: Vec<_> = data_array.iter().take(1000).collect();

        // Flatten sample objects and analyze columns
        let mut all_keys = std::collections::HashSet::new();
        let mut column_values: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

        for obj in &sample_data {
            let flattened = self.flatten_json_object(obj, "");

            for (key, value) in &flattened {
                all_keys.insert(key.clone());
                column_values.entry(key.clone()).or_insert_with(Vec::new).push(value.clone());
            }
        }

        let mut headers: Vec<String> = all_keys.into_iter().collect();
        headers.sort();

        let mut columns = Vec::new();
        for header in &headers {
            let values = column_values.get(header).unwrap_or(&Vec::new());
            let data_type = Self::infer_data_type(values);
            let nullable = values.iter().filter(|v| v.is_empty() || *v == "NULL").count() > 0;

            columns.push(json!({
                "column_name": header,
                "data_type": data_type,
                "is_nullable": if nullable { "YES" } else { "NO" },
                "sample_values": values.iter().take(5).cloned().collect::<Vec<_>>()
            }));
        }

        let table_name = self.get_table_name();

        let schema = json!({
            "database_schema": "json",
            "tables": {
                table_name: columns
            },
            "file_metadata": {
                "size_bytes": self.file_metadata.size_bytes,
                "size_human": format_bytes(self.file_metadata.size_bytes),
                "last_modified": self.file_metadata.last_modified.to_rfc3339(),
                "total_objects": self.file_metadata.total_objects,
                "root_structure": format!("{:?}", self.file_metadata.root_structure),
                "root_path": self.root_path,
                "array_path": self.array_path
            },
            "refreshed_at": chrono::Utc::now().to_rfc3339(),
            "generation_time_ms": start.elapsed().as_millis()
        });

        Ok(schema)
    }

    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        Ok(vec![self.get_table_name()])
    }

    async fn analyze_database(&self) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let table_name = self.get_table_name();
        let _schema = self.fetch_schema().await?;

        Ok(json!({
            "database_schema": "json",
            "statistics": {
                "table_count": 1,
                "total_size": self.file_metadata.size_bytes,
                "total_size_human": format_bytes(self.file_metadata.size_bytes),
                "total_rows": self.file_metadata.total_objects,
            },
            "table_names": vec![table_name.clone()],
            "key_tables": [{
                "name": table_name,
                "size_bytes": self.file_metadata.size_bytes,
                "connections": 0,
            }],
            "largest_tables": [{
                "name": table_name,
                "size_bytes": self.file_metadata.size_bytes,
                "size_human": format_bytes(self.file_metadata.size_bytes),
            }],
            "analyzed_at": chrono::Utc::now().to_rfc3339(),
        }))
    }

    async fn get_tables_schema(&self, tables: Vec<&str>) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let table_name = self.get_table_name();
        let mut result = json!({});

        for requested_table in tables {
            if requested_table != table_name {
                continue;
            }

            let data_array = self.get_data_array()?;
            let sample_data: Vec<_> = data_array.iter().take(5).collect();

            // Flatten sample objects
            let mut all_keys = std::collections::HashSet::new();
            let mut flattened_rows = Vec::new();

            for obj in &sample_data {
                let flattened = self.flatten_json_object(obj, "");
                for (key, _) in &flattened {
                    all_keys.insert(key.clone());
                }
                flattened_rows.push(flattened);
            }

            let mut headers: Vec<String> = all_keys.into_iter().collect();
            headers.sort();

            // Build column info
            let mut columns = Vec::new();
            for header in &headers {
                columns.push(json!({
                    "name": header,
                    "type": "text", // JSON doesn't have strict types
                    "nullable": true,
                }));
            }

            // Build sample data rows
            let sample_rows: Vec<Value> = flattened_rows.iter().map(|flattened_row| {
                let mut row_obj = json!({});
                for header in &headers {
                    let value = flattened_row.iter()
                        .find(|(k, _)| k == header)
                        .map(|(_, v)| v.clone())
                        .unwrap_or_else(|| "NULL".to_string());
                    row_obj[header] = json!(value);
                }
                row_obj
            }).collect();

            result[requested_table] = json!({
                "columns": columns,
                "primary_keys": [],
                "foreign_keys": [],
                "row_count": self.file_metadata.total_objects,
                "sample_data": sample_rows,
            });
        }

        Ok(result)
    }

    async fn search_tables(&self, pattern: &str) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let table_name = self.get_table_name();
        let pattern_lower = pattern.to_lowercase();

        let matches = if table_name.to_lowercase().contains(&pattern_lower) {
            vec![json!({
                "name": table_name,
                "description": Some("JSON data file"),
                "column_count": 0, // Would require schema analysis
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
        let table_name = self.get_table_name();

        if table != table_name {
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
        let table_name = self.get_table_name();

        Ok(json!({
            "summary": {
                "total_tables": 1,
                "total_size_bytes": self.file_metadata.size_bytes,
                "total_size_human": format_bytes(self.file_metadata.size_bytes),
            },
            "largest_tables": [{
                "name": table_name,
                "size_bytes": self.file_metadata.size_bytes,
                "size_human": format_bytes(self.file_metadata.size_bytes),
            }],
            "most_connected_tables": [],
        }))
    }
}