use super::super::core::base::{format_bytes, DataSourceConnector};
use super::duckdb_wrapper::{DuckDBWrapper, create_duckdb_wrapper};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::error::Error;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error, info, warn};
use calamine::{open_workbook, Reader, DataType, Xlsx};

pub struct ExcelConnector {
    file_path: String,
    sheet_name: Option<String>,
    header_row: Option<usize>,
    data_start_row: usize,
    schema_cache: Option<Value>,
    file_metadata: ExcelFileMetadata,
    // DuckDB integration for full SQL support
    duckdb_wrapper: Option<DuckDBWrapper>,
    use_duckdb: bool,
}

#[derive(Debug, Clone)]
struct ExcelFileMetadata {
    size_bytes: u64,
    last_modified: chrono::DateTime<chrono::Utc>,
    sheet_names: Vec<String>,
    total_sheets: usize,
}

#[derive(Debug, Clone)]
struct SheetInfo {
    name: String,
    row_count: usize,
    column_count: usize,
}

impl ExcelConnector {
    pub fn new(config: &Value) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let file_path = config
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or("Missing file_path in configuration")?;

        // Validate file exists and is an Excel file
        if !Path::new(file_path).exists() {
            return Err(format!("File not found: {}", file_path).into());
        }

        let extension = Path::new(file_path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        if !matches!(extension.as_str(), "xlsx" | "xls" | "xlsm") {
            return Err(format!("File must be an Excel file (.xlsx, .xls, .xlsm): {}", file_path).into());
        }

        let sheet_name = config
            .get("sheet_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let header_row = config
            .get("header_row")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let data_start_row = config
            .get("data_start_row")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or_else(|| header_row.map(|h| h + 1).unwrap_or(0));

        info!("Excel connector configured for file: {}", file_path);
        debug!("Sheet name: {:?}", sheet_name);
        debug!("Header row: {:?}", header_row);
        debug!("Data start row: {}", data_start_row);

        let mut connector = Self {
            file_path: file_path.to_string(),
            sheet_name: sheet_name.clone(),
            header_row,
            data_start_row,
            schema_cache: None,
            file_metadata: ExcelFileMetadata {
                size_bytes: 0,
                last_modified: chrono::Utc::now(),
                sheet_names: Vec::new(),
                total_sheets: 0,
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
                "file_type": "excel",
                "table_name": sheet_name.as_deref().unwrap_or("data"),
                "sheet_name": sheet_name
            })) {
                Ok(duckdb) => {
                    info!("DuckDB wrapper initialized for Excel file");
                    connector.duckdb_wrapper = Some(duckdb);
                    connector.use_duckdb = true;
                },
                Err(e) => {
                    warn!("Failed to initialize DuckDB wrapper for Excel file: {}, falling back to basic operations", e);
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

        // Get sheet names
        let mut excel: Xlsx<_> = open_workbook(&self.file_path)?;
        self.file_metadata.sheet_names = excel.sheet_names().to_vec();
        self.file_metadata.total_sheets = self.file_metadata.sheet_names.len();

        debug!("Excel file metadata loaded: {} bytes, {} sheets",
               self.file_metadata.size_bytes,
               self.file_metadata.total_sheets);

        Ok(())
    }

    fn get_sheet_info(&self, sheet_name: &str) -> Result<SheetInfo, Box<dyn Error + Send + Sync>> {
        let mut excel: Xlsx<_> = open_workbook(&self.file_path)?;

        if let Some(range) = excel.worksheet_range(sheet_name) {
            let range = range?;
            Ok(SheetInfo {
                name: sheet_name.to_string(),
                row_count: range.height(),
                column_count: range.width(),
            })
        } else {
            Err(format!("Sheet '{}' not found", sheet_name).into())
        }
    }

    fn get_all_sheet_names(&self) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        Ok(self.file_metadata.sheet_names.clone())
    }

    fn read_sheet_data(&self, sheet_name: &str, limit: Option<usize>) -> Result<(Vec<String>, Vec<Vec<String>>), Box<dyn Error + Send + Sync>> {
        let mut excel: Xlsx<_> = open_workbook(&self.file_path)?;

        if let Some(range) = excel.worksheet_range(sheet_name) {
            let range = range?;

            let mut headers = Vec::new();
            let mut rows = Vec::new();
            let mut row_count = 0;

            // Determine the actual data start row
            let start_row = if let Some(header_row) = self.header_row {
                header_row
            } else {
                self.data_start_row
            };

            for (i, row) in range.rows().enumerate() {
                if i < start_row {
                    continue;
                }

                // Apply limit if specified
                if let Some(limit_val) = limit {
                    if row_count >= limit_val {
                        break;
                    }
                }

                let row_data: Vec<String> = row.iter()
                    .map(|cell| match cell {
                        DataType::Empty => String::new(),
                        DataType::String(s) => s.clone(),
                        DataType::Float(f) => f.to_string(),
                        DataType::Int(i) => i.to_string(),
                        DataType::Bool(b) => b.to_string(),
                        DataType::Error(e) => format!("ERROR: {:?}", e),
                    })
                    .collect();

                if i == start_row && self.header_row.is_some() {
                    headers = row_data;
                } else {
                    rows.push(row_data);
                    row_count += 1;
                }
            }

            // If no headers were specified, generate default column names
            if headers.is_empty() && !rows.is_empty() {
                let col_count = rows[0].len();
                for i in 0..col_count {
                    headers.push(format!("column_{}", i + 1));
                }
            }

            Ok((headers, rows))
        } else {
            Err(format!("Sheet '{}' not found", sheet_name).into())
        }
    }

    fn infer_data_type(values: &[String]) -> String {
        if values.is_empty() {
            return "text".to_string();
        }

        let mut int_count = 0;
        let mut float_count = 0;
        let mut bool_count = 0;
        let mut date_count = 0;
        let mut null_count = 0;
        let total_non_null = values.iter().filter(|v| !v.is_empty() && v != "NULL" && v != "null").count();

        for value in values {
            if value.is_empty() || value == "NULL" || value == "null" {
                null_count += 1;
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

            // Excel serial date detection (basic)
            if let Ok(serial_num) = value.parse::<f64>() {
                if serial_num > 1.0 && serial_num < 100000.0 {
                    date_count += 1;
                    continue;
                }
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

    fn get_active_sheet_name(&self) -> Result<String, Box<dyn Error + Send + Sync>> {
        // Return first sheet as default when no specific sheet is configured
        let sheet_names = self.get_all_sheet_names()?;
        if let Some(first_sheet) = sheet_names.first() {
            Ok(first_sheet.clone())
        } else {
            Err("No sheets found in Excel file".into())
        }
    }
}

#[async_trait]
impl DataSourceConnector for ExcelConnector {
    async fn test_connection(&mut self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        info!("Testing Excel file connection: {}", self.file_path);

        match self.get_active_sheet_name() {
            Ok(sheet_name) => {
                match self.get_sheet_info(&sheet_name) {
                    Ok(sheet_info) => {
                        info!("Excel file test successful: {} (sheet: {})", self.file_path, sheet_name);
                        debug!("Sheet info: {} rows, {} columns", sheet_info.row_count, sheet_info.column_count);
                        Ok(true)
                    },
                    Err(e) => {
                        error!("Excel file test failed - sheet error: {}", e);
                        Err(e)
                    }
                }
            },
            Err(e) => {
                error!("Excel file test failed - cannot determine sheet: {}", e);
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
                        debug!("DuckDB query executed successfully on Excel file");
                        return Ok(result);
                    },
                    Err(e) => {
                        warn!("DuckDB query failed on Excel file: {}, falling back to basic operations", e);
                        // Fall through to basic Excel operations
                    }
                }
            }
        }

        // Fallback to basic Excel operations for simple SELECT * queries
        let default_sheet_name = self.get_active_sheet_name()?;
        let sheet_names = self.get_all_sheet_names()?;

        // For Excel, we only support basic SELECT * FROM [table] queries
        // where table is one of the sheet names
        let normalized_query = query.to_lowercase().replace('\n', " ").trim().to_string();

        // Find which sheet name is referenced in the query
        let queried_sheet = sheet_names.iter().find(|sheet| {
            normalized_query.contains(&sheet.to_lowercase())
        });

        let sheet_name = if let Some(found_sheet) = queried_sheet {
            found_sheet.clone()
        } else {
            return Err(format!("Only basic SELECT * queries are supported for Excel files without DuckDB. Available tables: {}", sheet_names.join(", ")).into());
        };

        let start = Instant::now();
        let (headers, rows) = self.read_sheet_data(&sheet_name, Some(limit as usize))?;
        let execution_time_ms = start.elapsed().as_millis() as i64;

        if rows.is_empty() {
            return Ok(json!({
                "columns": headers,
                "rows": [],
                "row_count": 0,
                "execution_time_ms": execution_time_ms
            }));
        }

        Ok(json!({
            "columns": headers,
            "rows": rows,
            "row_count": rows.len(),
            "execution_time_ms": execution_time_ms,
            "query": query,
            "sheet_name": sheet_name
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
        let start = Instant::now();
        let offset = (page - 1) * limit;

        // For Excel, we need to read all data and then paginate
        // Use the requested sheet name regardless of configured sheet
        let (headers, all_rows) = self.read_sheet_data(table_name, None)?;

        // Apply sorting if specified
        let mut rows = all_rows;
        if let Some(sort_col) = sort_column {
            if let Some(col_index) = headers.iter().position(|h| h == sort_col) {
                let direction = sort_direction.unwrap_or("ASC");
                rows.sort_by(|a, b| {
                    let a_val = a.get(col_index).unwrap_or(&"".to_string());
                    let b_val = b.get(col_index).unwrap_or(&"".to_string());

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
            "page_size": limit,
            "sheet_name": table_name
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
                        // Enhance DuckDB schema with Excel-specific metadata
                        if let Some(tables) = duckdb_schema["tables"].as_object_mut() {
                            for (sheet_name, table_info) in tables {
                                if let Ok(sheet_info) = self.get_sheet_info(sheet_name) {
                                    table_info["sheet_info"] = json!({
                                        "total_rows": sheet_info.row_count,
                                        "total_columns": sheet_info.column_count,
                                        "is_active_sheet": self.sheet_name.as_ref().map_or(true, |s| s == sheet_name)
                                    });
                                }
                            }
                        }

                        // Add Excel-specific file metadata
                        duckdb_schema["file_metadata"] = json!({
                            "size_bytes": self.file_metadata.size_bytes,
                            "size_human": format_bytes(self.file_metadata.size_bytes),
                            "last_modified": self.file_metadata.last_modified.to_rfc3339(),
                            "total_sheets": self.file_metadata.total_sheets,
                            "sheet_names": self.file_metadata.sheet_names,
                            "header_row": self.header_row,
                            "data_start_row": self.data_start_row
                        });

                        debug!("DuckDB schema fetched successfully for Excel file");
                        return Ok(duckdb_schema);
                    },
                    Err(e) => {
                        warn!("Failed to fetch schema using DuckDB for Excel file: {}, falling back to basic operations", e);
                        // Fall through to basic Excel operations
                    }
                }
            }
        }

        // Fallback to basic Excel operations
        let sheet_names = self.get_all_sheet_names()?;
        let mut tables = json!({});

        // Generate schema for all sheets in the Excel file
        for sheet_name in &sheet_names {
            match self.read_sheet_data(sheet_name, Some(1000)) {
                Ok((headers, sample_rows)) => {
                    let mut columns = Vec::new();

                    for (col_index, col_name) in headers.iter().enumerate() {
                        let mut values = Vec::new();

                        for row in &sample_rows {
                            if let Some(value) = row.get(col_index) {
                                values.push(value.clone());
                            }
                        }

                        let data_type = Self::infer_data_type(&values);
                        let nullable = values.iter().filter(|v| v.is_empty() || v == "NULL" || v == "null").count() > 0;

                        columns.push(json!({
                            "column_name": col_name,
                            "data_type": data_type,
                            "is_nullable": if nullable { "YES" } else { "NO" },
                            "sample_values": values.iter().take(5).cloned().collect::<Vec<_>>()
                        }));
                    }

                    let sheet_info = self.get_sheet_info(sheet_name)?;
                    tables[sheet_name] = json!({
                        "columns": columns,
                        "row_count": sheet_info.row_count.saturating_sub(self.data_start_row),
                        "sheet_info": {
                            "total_rows": sheet_info.row_count,
                            "total_columns": sheet_info.column_count,
                            "is_active_sheet": self.sheet_name.as_ref().map_or(true, |s| s == sheet_name)
                        }
                    });
                },
                Err(e) => {
                    warn!("Failed to read sheet '{}': {}", sheet_name, e);
                    // Still include the sheet in the schema even if we can't read it
                    tables[sheet_name] = json!({
                        "columns": [],
                        "row_count": 0,
                        "sheet_info": {
                            "total_rows": 0,
                            "total_columns": 0,
                            "read_error": e.to_string()
                        }
                    });
                }
            }
        }

        let schema = json!({
            "database_schema": "excel",
            "tables": tables,
            "file_metadata": {
                "size_bytes": self.file_metadata.size_bytes,
                "size_human": format_bytes(self.file_metadata.size_bytes),
                "last_modified": self.file_metadata.last_modified.to_rfc3339(),
                "total_sheets": self.file_metadata.total_sheets,
                "sheet_names": self.file_metadata.sheet_names,
                "header_row": self.header_row,
                "data_start_row": self.data_start_row
            },
            "refreshed_at": chrono::Utc::now().to_rfc3339(),
            "generation_time_ms": start.elapsed().as_millis()
        });

        Ok(schema)
    }

    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        let mut sheets = self.get_all_sheet_names()?;

        // If a specific sheet is configured, prioritize it but still list all sheets
        if let Some(configured_sheet) = &self.sheet_name {
            if let Some(pos) = sheets.iter().position(|s| s == configured_sheet) {
                // Move the configured sheet to the front
                let sheet = sheets.remove(pos);
                sheets.insert(0, sheet);
            }
        }

        Ok(sheets)
    }

    async fn analyze_database(&self) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let sheet_names = self.get_all_sheet_names()?;
        let mut table_names = Vec::new();
        let mut key_tables = Vec::new();
        let mut largest_tables = Vec::new();

        for sheet_name in &sheet_names {
            if let Ok(sheet_info) = self.get_sheet_info(sheet_name) {
                table_names.push(sheet_name.clone());

                // All sheets are considered key tables in Excel files
                key_tables.push(json!({
                    "name": sheet_name,
                    "size_bytes": 0, // Excel sheets don't have individual sizes
                    "connections": 0,
                }));

                // Track largest sheets by row count
                largest_tables.push(json!({
                    "name": sheet_name,
                    "size_bytes": sheet_info.row_count as u64,
                    "size_human": format!("{} rows", sheet_info.row_count),
                }));
            }
        }

        // Sort by row count
        largest_tables.sort_by(|a, b| {
            b["size_bytes"].as_u64().cmp(&a["size_bytes"].as_u64())
        });

        Ok(json!({
            "database_schema": "excel",
            "statistics": {
                "table_count": sheet_names.len(),
                "total_size": self.file_metadata.size_bytes,
                "total_size_human": format_bytes(self.file_metadata.size_bytes),
                "total_rows": largest_tables.iter().map(|t| t["size_bytes"].as_u64().unwrap_or(0)).sum::<u64>(),
            },
            "table_names": table_names,
            "key_tables": key_tables,
            "largest_tables": largest_tables.into_iter().take(10).collect::<Vec<_>>(),
            "analyzed_at": chrono::Utc::now().to_rfc3339(),
        }))
    }

    async fn get_tables_schema(&self, tables: Vec<&str>) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let mut result = json!({});

        for table_name in tables {
            match self.read_sheet_data(table_name, Some(5)) {
                Ok((headers, sample_rows)) => {
                    let sheet_info = self.get_sheet_info(table_name)?;

                    // Build column info
                    let mut columns = Vec::new();
                    for col_name in &headers {
                        columns.push(json!({
                            "name": col_name,
                            "type": "text", // Excel doesn't have strict types
                            "nullable": true,
                        }));
                    }

                    result[table_name] = json!({
                        "columns": columns,
                        "primary_keys": [],
                        "foreign_keys": [],
                        "row_count": sheet_info.row_count.saturating_sub(self.data_start_row),
                        "sheet_info": {
                            "total_rows": sheet_info.row_count,
                            "total_columns": sheet_info.column_count,
                            "is_configured_sheet": self.sheet_name.as_ref().map_or(false, |s| s == table_name)
                        },
                        "sample_data": sample_rows.iter().enumerate().map(|(_i, row)| {
                            let mut row_obj = json!({});
                            for (j, val) in row.iter().enumerate() {
                                if let Some(col_name) = headers.get(j) {
                                    row_obj[col_name] = json!(val);
                                }
                            }
                            row_obj
                        }).collect::<Vec<_>>(),
                    });
                },
                Err(e) => {
                    warn!("Failed to get schema for sheet '{}': {}", table_name, e);
                    // Still include the sheet in the result even if we can't read it
                    result[table_name] = json!({
                        "columns": [],
                        "primary_keys": [],
                        "foreign_keys": [],
                        "row_count": 0,
                        "sheet_info": {
                            "read_error": e.to_string()
                        },
                        "sample_data": []
                    });
                }
            }
        }

        Ok(result)
    }

    async fn search_tables(&self, pattern: &str) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let sheet_names = self.get_all_sheet_names()?;
        let pattern_lower = pattern.to_lowercase();

        let matches: Vec<Value> = sheet_names.iter()
            .filter(|name| name.to_lowercase().contains(&pattern_lower))
            .map(|name| {
                let sheet_info = self.get_sheet_info(name).unwrap_or_else(|_| SheetInfo {
                    name: name.clone(),
                    row_count: 0,
                    column_count: 0,
                });

                let is_configured = self.sheet_name.as_ref().map_or(false, |s| s == name);

                json!({
                    "name": name,
                    "description": Some(if is_configured {
                        "Excel worksheet (configured)"
                    } else {
                        "Excel worksheet"
                    }),
                    "column_count": sheet_info.column_count,
                    "row_count": sheet_info.row_count.saturating_sub(self.data_start_row),
                    "sheet_info": {
                        "total_rows": sheet_info.row_count,
                        "is_configured": is_configured
                    }
                })
            })
            .collect();

        Ok(json!({
            "matches": matches,
            "total_matches": matches.len(),
        }))
    }

    async fn get_related_tables(&self, table: &str) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let table_schema = self.get_tables_schema(vec![table]).await?;
        let all_sheet_names = self.get_all_sheet_names()?;

        // List all other sheets as "related" for navigation
        let other_sheets: Vec<String> = all_sheet_names
            .iter()
            .filter(|name| *name != table)
            .cloned()
            .collect();

        Ok(json!({
            "main_table": table_schema.get(table),
            "related_tables": {
                "other_sheets": other_sheets.iter().map(|name| {
                    let is_configured = self.sheet_name.as_ref().map_or(false, |s| s == name);
                    json!({
                        "name": name,
                        "type": "excel_sheet",
                        "description": if is_configured {
                            "Excel worksheet (configured default)"
                        } else {
                            "Excel worksheet"
                        }
                    })
                }).collect::<Vec<_>>()
            },
            "relationship_count": other_sheets.len(),
        }))
    }

    async fn get_database_stats(&self) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let sheet_names = self.get_all_sheet_names()?;

        Ok(json!({
            "summary": {
                "total_tables": sheet_names.len(),
                "total_size_bytes": self.file_metadata.size_bytes,
                "total_size_human": format_bytes(self.file_metadata.size_bytes),
            },
            "largest_tables": sheet_names.iter().map(|name| {
                let sheet_info = self.get_sheet_info(name).unwrap_or_else(|_| SheetInfo {
                    name: name.clone(),
                    row_count: 0,
                    column_count: 0,
                });

                json!({
                    "name": name,
                    "size_bytes": sheet_info.row_count as u64,
                    "size_human": format!("{} rows", sheet_info.row_count),
                })
            }).collect::<Vec<_>>(),
            "most_connected_tables": [],
        }))
    }
}