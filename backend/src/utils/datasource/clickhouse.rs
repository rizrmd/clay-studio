use super::base::DataSourceConnector;
use async_trait::async_trait;
use clickhouse::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::time::Duration;

pub struct ClickHouseConnector {
    client: Client,
}

impl ClickHouseConnector {
    pub fn new(config: &Value) -> Result<Self, Box<dyn Error>> {
        let url = if let Some(url) = config.get("url").and_then(|v| v.as_str()) {
            url.to_string()
        } else {
            // Construct URL from individual components
            let host = config
                .get("host")
                .and_then(|v| v.as_str())
                .unwrap_or("localhost");
            let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(8123);
            let database = config
                .get("database")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            let username = config
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            let password = config
                .get("password")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if password.is_empty() {
                format!("http://{}@{}:{}/{}", username, host, port, database)
            } else {
                format!(
                    "http://{}:{}@{}:{}/{}",
                    username, password, host, port, database
                )
            }
        };

        eprintln!(
            "[DEBUG] ClickHouse connection URL (masked): {}",
            Self::mask_url(&url)
        );

        let client = Client::default().with_url(url);

        Ok(Self { client })
    }

    fn mask_url(url: &str) -> String {
        if url.contains('@') {
            let parts: Vec<&str> = url.splitn(2, "://").collect();
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
                    url.to_string()
                }
            } else {
                url.to_string()
            }
        } else {
            url.to_string()
        }
    }
}

#[async_trait]
impl DataSourceConnector for ClickHouseConnector {
    async fn test_connection(&mut self) -> Result<bool, Box<dyn Error>> {
        // Wrap the connection test with a 3-second timeout
        match tokio::time::timeout(
            Duration::from_secs(3),
            self.client.query("SELECT 1").fetch_one::<u8>(),
        )
        .await
        {
            Ok(Ok(_)) => Ok(true),
            Ok(Err(e)) => Err(Box::new(e) as Box<dyn Error>),
            Err(_) => Err(Box::<dyn Error>::from("Connection timeout after 3 seconds")),
        }
    }

    async fn execute_query(&self, query: &str, limit: i32) -> Result<Value, Box<dyn Error>> {
        // Add LIMIT if not present
        let query_with_limit = if query.to_lowercase().contains("limit") {
            query.to_string()
        } else {
            format!("{} LIMIT {}", query, limit)
        };

        let start = std::time::Instant::now();

        // Use ClickHouse's native HTTP interface with JSON format for better control
        // We'll make a direct HTTP request to get proper column metadata
        let query_with_format = format!("{} FORMAT JSON", query_with_limit);

        // For now, use a simpler approach that works with the current clickhouse crate
        // The crate doesn't easily expose column metadata, so we'll use FORMAT JSON
        let raw_result = self
            .client
            .query(&query_with_format)
            .fetch_one::<String>()
            .await;

        let execution_time_ms = start.elapsed().as_millis() as i64;

        match raw_result {
            Ok(json_str) => {
                // Parse the JSON response from ClickHouse
                if let Ok(json_response) = serde_json::from_str::<Value>(&json_str) {
                    // Extract metadata and data from the JSON response
                    let meta = json_response.get("meta").and_then(|m| m.as_array());
                    let data = json_response.get("data").and_then(|d| d.as_array());

                    if let (Some(meta_array), Some(data_array)) = (meta, data) {
                        // Extract column names from metadata
                        let columns: Vec<String> = meta_array
                            .iter()
                            .filter_map(|col| col.get("name").and_then(|n| n.as_str()))
                            .map(|s| s.to_string())
                            .collect();

                        // Convert data rows to the expected format
                        let mut formatted_rows = Vec::new();
                        for row in data_array {
                            if let Some(row_obj) = row.as_object() {
                                let mut row_data = Vec::new();
                                for col_name in &columns {
                                    let value = row_obj.get(col_name).unwrap_or(&Value::Null);
                                    row_data.push(match value {
                                        Value::String(s) => s.clone(),
                                        Value::Number(n) => n.to_string(),
                                        Value::Bool(b) => b.to_string(),
                                        Value::Null => "NULL".to_string(),
                                        Value::Array(a) => serde_json::to_string(a)
                                            .unwrap_or_else(|_| "[]".to_string()),
                                        Value::Object(o) => serde_json::to_string(o)
                                            .unwrap_or_else(|_| "{}".to_string()),
                                    });
                                }
                                formatted_rows.push(row_data);
                            }
                        }

                        return Ok(json!({
                            "columns": columns,
                            "rows": formatted_rows,
                            "row_count": formatted_rows.len(),
                            "execution_time_ms": execution_time_ms
                        }));
                    }
                }

                // If JSON parsing failed, treat as a single string result
                Ok(json!({
                    "columns": ["result"],
                    "rows": [[json_str]],
                    "row_count": 1,
                    "execution_time_ms": execution_time_ms
                }))
            }
            Err(_) => {
                // Try without FORMAT JSON as a fallback
                let simple_result = self
                    .client
                    .query(&query_with_limit)
                    .fetch_all::<String>()
                    .await;

                match simple_result {
                    Ok(rows) => {
                        if rows.is_empty() {
                            Ok(json!({
                                "columns": [],
                                "rows": [],
                                "row_count": 0,
                                "execution_time_ms": execution_time_ms
                            }))
                        } else {
                            // Convert string results to rows
                            let formatted_rows: Vec<Vec<String>> =
                                rows.into_iter().map(|s| vec![s]).collect();

                            Ok(json!({
                                "columns": ["result"],
                                "rows": formatted_rows,
                                "row_count": formatted_rows.len(),
                                "execution_time_ms": execution_time_ms
                            }))
                        }
                    }
                    Err(e) => Err(Box::new(e)),
                }
            }
        }
    }

    async fn fetch_schema(&self) -> Result<Value, Box<dyn Error>> {
        let columns: Vec<(String, String, String, String)> = self.client
            .query("SELECT database, table, name, type FROM system.columns WHERE database NOT IN ('system', 'information_schema', 'INFORMATION_SCHEMA') ORDER BY database, table, name")
            .fetch_all()
            .await?;

        let mut tables: std::collections::HashMap<String, Value> = std::collections::HashMap::new();

        for (database, table, name, col_type) in columns {
            let full_table_name = format!("{}.{}", database, table);

            let table_entry = tables.entry(full_table_name.clone()).or_insert_with(|| {
                json!({
                    "name": full_table_name,
                    "database": database,
                    "table": table,
                    "columns": []
                })
            });

            if let Some(columns_array) = table_entry
                .get_mut("columns")
                .and_then(|v| v.as_array_mut())
            {
                columns_array.push(json!({
                    "name": name,
                    "type": col_type,
                    "nullable": col_type.contains("Nullable")
                }));
            }
        }

        Ok(json!({
            "tables": tables
        }))
    }

    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let tables_info: Vec<(String, String)> = self.client
            .query("SELECT database, name FROM system.tables WHERE database NOT IN ('system', 'information_schema', 'INFORMATION_SCHEMA') ORDER BY database, name")
            .fetch_all()
            .await?;

        let tables = tables_info
            .into_iter()
            .map(|(database, name)| format!("{}.{}", database, name))
            .collect();

        Ok(tables)
    }

    async fn analyze_database(&self) -> Result<Value, Box<dyn Error>> {
        // Get table statistics - simplified version
        let table_count: u64 = self.client
            .query("SELECT count() FROM system.tables WHERE database NOT IN ('system', 'information_schema', 'INFORMATION_SCHEMA')")
            .fetch_one()
            .await?;

        // Get database names
        let db_names: Vec<String> = self.client
            .query("SELECT name FROM system.databases WHERE name NOT IN ('system', 'information_schema', 'INFORMATION_SCHEMA')")
            .fetch_all()
            .await?;

        Ok(json!({
            "statistics": {
                "database_count": db_names.len(),
                "table_count": table_count,
                "total_rows": 0,
                "total_size_bytes": 0,
                "total_size_human": "Unknown"
            },
            "largest_tables": [],
            "key_tables": [],
            "table_names": []
        }))
    }

    async fn get_tables_schema(&self, tables: Vec<&str>) -> Result<Value, Box<dyn Error>> {
        let table_list = tables
            .iter()
            .map(|t| format!("'{}'", t.replace("'", "''")))
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!(
            "SELECT database, table, name, type FROM system.columns WHERE CONCAT(database, '.', table) IN ({}) ORDER BY database, table, name",
            table_list
        );

        let columns: Vec<(String, String, String, String)> =
            self.client.query(&query).fetch_all().await?;

        let mut result: std::collections::HashMap<String, Value> = std::collections::HashMap::new();

        for (database, table, name, col_type) in columns {
            let full_table_name = format!("{}.{}", database, table);

            let table_entry = result.entry(full_table_name.clone()).or_insert_with(|| {
                json!({
                    "name": full_table_name,
                    "database": database,
                    "table": table,
                    "columns": []
                })
            });

            if let Some(columns_array) = table_entry
                .get_mut("columns")
                .and_then(|v| v.as_array_mut())
            {
                columns_array.push(json!({
                    "name": name,
                    "type": col_type,
                    "nullable": col_type.contains("Nullable")
                }));
            }
        }

        Ok(json!({
            "tables": result
        }))
    }

    async fn search_tables(&self, pattern: &str) -> Result<Value, Box<dyn Error>> {
        let query = format!(
            "SELECT database, name FROM system.tables WHERE database NOT IN ('system', 'information_schema', 'INFORMATION_SCHEMA') AND name LIKE '{}' ORDER BY database, name",
            pattern.replace('%', "%").replace('*', "%")
        );

        let tables_info: Vec<(String, String)> = self.client.query(&query).fetch_all().await?;

        let tables: Vec<Value> = tables_info
            .into_iter()
            .map(|(database, name)| {
                json!({
                    "database": database,
                    "name": format!("{}.{}", database, name),
                    "table": name
                })
            })
            .collect();

        Ok(json!({
            "tables": tables
        }))
    }

    async fn get_related_tables(&self, table: &str) -> Result<Value, Box<dyn Error>> {
        // ClickHouse doesn't have traditional foreign keys, so we'll return the table schema
        // and suggest related tables based on naming patterns
        let parts: Vec<&str> = table.split('.').collect();
        let (database, table_name) = if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            ("default", table)
        };

        let main_table_query = format!(
            "SELECT name, type FROM system.columns WHERE database = '{}' AND table = '{}' ORDER BY name",
            database, table_name
        );

        let columns: Vec<(String, String)> =
            self.client.query(&main_table_query).fetch_all().await?;

        let columns_json: Vec<Value> = columns
            .into_iter()
            .map(|(name, col_type)| {
                json!({
                    "name": name,
                    "type": col_type,
                    "nullable": col_type.contains("Nullable")
                })
            })
            .collect();

        // Find potentially related tables based on naming patterns
        let related_query = format!(
            "SELECT name FROM system.tables WHERE database = '{}' AND name != '{}' AND (name LIKE '%{}%' OR '{}' LIKE CONCAT('%', name, '%')) LIMIT 10",
            database, table_name, table_name, table_name
        );

        let related_tables: Vec<String> = self.client.query(&related_query).fetch_all().await?;

        let related_full_names: Vec<String> = related_tables
            .into_iter()
            .map(|name| format!("{}.{}", database, name))
            .collect();

        Ok(json!({
            "main_table": {
                "name": table,
                "database": database,
                "table": table_name,
                "columns": columns_json
            },
            "related_tables": related_full_names,
            "note": "ClickHouse doesn't have traditional foreign key relationships. Related tables are suggested based on naming patterns."
        }))
    }

    async fn get_database_stats(&self) -> Result<Value, Box<dyn Error>> {
        // Get database names
        let databases: Vec<String> = self.client
            .query("SELECT name FROM system.databases WHERE name NOT IN ('system', 'information_schema', 'INFORMATION_SCHEMA')")
            .fetch_all()
            .await?;

        // Get total table count
        let table_count: u64 = self.client
            .query("SELECT count() FROM system.tables WHERE database NOT IN ('system', 'information_schema', 'INFORMATION_SCHEMA')")
            .fetch_one()
            .await?;

        Ok(json!({
            "database_count": databases.len(),
            "table_count": table_count,
            "total_rows": 0,
            "total_size_bytes": 0,
            "total_size_human": "Unknown",
            "databases": databases
        }))
    }
}
