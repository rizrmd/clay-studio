use super::base::McpHandlers;
use crate::core::mcp::types::*;
use crate::utils::datasource::create_connector;
use serde_json::{json, Value};

impl McpHandlers {
    pub async fn get_schema(
        &self,
        args: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        self.execute_db_operation("get_schema", async {
            let datasource_id = args
                .get("datasource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: datasource_id".to_string())?;

            // Get connector
            let source = self.get_datasource_connector(datasource_id).await?;
            let connector = create_connector(&source.source_type, &source.connection_config)
                .await
                .map_err(|e| format!("Failed to create connector: {}", e))?;

            // Get schema
            let schema = connector
                .fetch_schema()
                .await
                .map_err(|e| format!("Failed to get schema: {}", e))?;

            let response_data = json!({
                "datasource": {
                    "id": datasource_id,
                    "name": source.name
                },
                "schema": schema
            });
            Ok(serde_json::to_string(&response_data)?)
        })
        .await
    }

    pub async fn search_schema(
        &self,
        args: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        self.execute_db_operation("search_schema", async {
            let datasource_id = args
                .get("datasource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: datasource_id".to_string())?;

            let search_term = args
                .get("search_term")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: search_term".to_string())?;

            // Get connector
            let source = self.get_datasource_connector(datasource_id).await?;
            let connector = create_connector(&source.source_type, &source.connection_config)
                .await
                .map_err(|e| format!("Failed to create connector: {}", e))?;

            // Search schema
            let results = connector
                .search_tables(search_term)
                .await
                .map_err(|e| format!("Schema search failed: {}", e))?;

            let response_data = json!({
                "datasource": {
                    "id": datasource_id,
                    "name": source.name
                },
                "search_term": search_term,
                "matches": results
            });
            Ok(serde_json::to_string(&response_data)?)
        })
        .await
    }

    pub async fn get_related_schema(
        &self,
        args: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        self.execute_db_operation("get_related_schema", async {
            let datasource_id = args
                .get("datasource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: datasource_id".to_string())?;

            let table_name = args
                .get("table_name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: table_name".to_string())?;

            // Get connector
            let source = self.get_datasource_connector(datasource_id).await?;
            let connector = create_connector(&source.source_type, &source.connection_config)
                .await
                .map_err(|e| format!("Failed to create connector: {}", e))?;

            // Get related schema
            let related = connector
                .get_related_tables(table_name)
                .await
                .map_err(|e| format!("Failed to get related schema: {}", e))?;

            let response_data = json!({
                "datasource": {
                    "id": datasource_id,
                    "name": source.name
                },
                "table_name": table_name,
                "related_schema": related
            });
            Ok(serde_json::to_string(&response_data)?)
        })
        .await
    }

    pub async fn get_schema_stats(
        &self,
        args: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        self.execute_db_operation("get_schema_stats", async {
            let datasource_id = args
                .get("datasource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: datasource_id".to_string())?;

            // Get connector
            let source = self.get_datasource_connector(datasource_id).await?;
            let connector = create_connector(&source.source_type, &source.connection_config)
                .await
                .map_err(|e| format!("Failed to create connector: {}", e))?;

            // Get schema statistics
            let stats = connector
                .get_database_stats()
                .await
                .map_err(|e| format!("Failed to get schema stats: {}", e))?;

            let response_data = json!({
                "datasource": {
                    "id": datasource_id,
                    "name": source.name
                },
                "statistics": stats
            });
            Ok(serde_json::to_string(&response_data)?)
        })
        .await
    }

    #[allow(dead_code)]
    pub fn format_inspection_result(&self, name: &str, analysis: &Value) -> String {
        let table_count = analysis["statistics"]["table_count"].as_u64().unwrap_or(0);
        let view_count = analysis["statistics"]["view_count"].as_u64().unwrap_or(0);
        let total_records = analysis["statistics"]["total_records"]
            .as_u64()
            .unwrap_or(0);

        let top_tables = self.format_top_tables(analysis);

        format!(
            "🔍 **Datasource Inspection Complete**\n\n\
             🔗 **Name**: {}\n\
             📊 **Statistics**:\n\
             • Tables: {}\n\
             • Views: {}\n\
             • Total Records: {}\n\n\
             {}\n\n\
             ✅ Schema information has been cached for faster future queries.",
            name, table_count, view_count, total_records, top_tables
        )
    }

    #[allow(dead_code)]
    pub fn format_top_tables(&self, analysis: &Value) -> String {
        if let Some(tables) = analysis["top_tables"].as_array() {
            if !tables.is_empty() {
                let mut result = String::from("**Top Tables by Size**:\n");
                for table in tables.iter().take(5) {
                    if let (Some(name), Some(count)) =
                        (table["name"].as_str(), table["estimated_count"].as_u64())
                    {
                        result.push_str(&format!("• {}: {} records\n", name, count));
                    }
                }
                result
            } else {
                String::from("**Tables**: No table information available")
            }
        } else {
            String::from("**Tables**: No table information available")
        }
    }

    pub fn parse_connection_config(
        &self,
        config: &Value,
        source_type: &str,
    ) -> Result<Value, JsonRpcError> {
        match config {
            Value::String(connection_url) => {
                // Parse connection URL into config object
                self.parse_connection_url(connection_url, source_type)
                    .map(Value::Object)
            }
            Value::Object(_) => {
                // Already a config object, validate it
                Ok(config.clone())
            }
            _ => Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: "Config must be either a connection URL string or a configuration object"
                    .to_string(),
                data: None,
            }),
        }
    }

    pub fn parse_connection_url(
        &self,
        url: &str,
        source_type: &str,
    ) -> Result<serde_json::Map<String, Value>, JsonRpcError> {
        let config = match source_type {
            "postgresql" | "postgres" => self.parse_postgres_url(url),
            "mysql" => self.parse_mysql_url(url),
            "clickhouse" => self.parse_clickhouse_url(url),
            _ => self.parse_generic_url(url),
        };

        config.ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: format!("Invalid connection URL format for {}", source_type),
            data: None,
        })
    }

    pub fn parse_postgres_url(&self, url: &str) -> Option<serde_json::Map<String, Value>> {
        let re = regex::Regex::new(
            r"postgresql://(?:([^:]+)(?::([^@]+))?@)?([^:/]+)(?::(\d+))?/([^?]+)(?:\?(.+))?",
        )
        .ok()?;
        let caps = re.captures(url)?;

        let mut config = serde_json::Map::new();
        config.insert("host".to_string(), json!(caps.get(3)?.as_str()));
        config.insert("database".to_string(), json!(caps.get(5)?.as_str()));

        if let Some(user) = caps.get(1) {
            config.insert("user".to_string(), json!(user.as_str()));
        }
        if let Some(password) = caps.get(2) {
            config.insert("password".to_string(), json!(password.as_str()));
        }
        if let Some(port) = caps.get(4) {
            if let Ok(port_num) = port.as_str().parse::<u16>() {
                config.insert("port".to_string(), json!(port_num));
            }
        } else {
            config.insert("port".to_string(), json!(5432));
        }

        Some(config)
    }

    pub fn parse_mysql_url(&self, url: &str) -> Option<serde_json::Map<String, Value>> {
        let re = regex::Regex::new(
            r"mysql://(?:([^:]+)(?::([^@]+))?@)?([^:/]+)(?::(\d+))?/([^?]+)(?:\?(.+))?",
        )
        .ok()?;
        let caps = re.captures(url)?;

        let mut config = serde_json::Map::new();
        config.insert("host".to_string(), json!(caps.get(3)?.as_str()));
        config.insert("database".to_string(), json!(caps.get(5)?.as_str()));

        if let Some(user) = caps.get(1) {
            config.insert("user".to_string(), json!(user.as_str()));
        }
        if let Some(password) = caps.get(2) {
            config.insert("password".to_string(), json!(password.as_str()));
        }
        if let Some(port) = caps.get(4) {
            if let Ok(port_num) = port.as_str().parse::<u16>() {
                config.insert("port".to_string(), json!(port_num));
            }
        } else {
            config.insert("port".to_string(), json!(3306));
        }

        Some(config)
    }

    pub fn parse_clickhouse_url(&self, url: &str) -> Option<serde_json::Map<String, Value>> {
        let re = regex::Regex::new(
            r"clickhouse://(?:([^:]+)(?::([^@]+))?@)?([^:/]+)(?::(\d+))?/([^?]+)(?:\?(.+))?",
        )
        .ok()?;
        let caps = re.captures(url)?;

        let mut config = serde_json::Map::new();
        config.insert("host".to_string(), json!(caps.get(3)?.as_str()));
        config.insert("database".to_string(), json!(caps.get(5)?.as_str()));

        if let Some(user) = caps.get(1) {
            config.insert("user".to_string(), json!(user.as_str()));
        }
        if let Some(password) = caps.get(2) {
            config.insert("password".to_string(), json!(password.as_str()));
        }
        if let Some(port) = caps.get(4) {
            if let Ok(port_num) = port.as_str().parse::<u16>() {
                config.insert("port".to_string(), json!(port_num));
            }
        } else {
            config.insert("port".to_string(), json!(8123));
        }

        Some(config)
    }

    pub fn parse_generic_url(&self, url: &str) -> Option<serde_json::Map<String, Value>> {
        let re = regex::Regex::new(
            r"(\w+)://(?:([^:]+)(?::([^@]+))?@)?([^:/]+)(?::(\d+))?/([^?]+)(?:\?(.+))?",
        )
        .ok()?;
        let caps = re.captures(url)?;

        let mut config = serde_json::Map::new();
        config.insert("host".to_string(), json!(caps.get(4)?.as_str()));
        config.insert("database".to_string(), json!(caps.get(6)?.as_str()));

        if let Some(user) = caps.get(2) {
            config.insert("user".to_string(), json!(user.as_str()));
        }
        if let Some(password) = caps.get(3) {
            config.insert("password".to_string(), json!(password.as_str()));
        }
        if let Some(port) = caps.get(5) {
            if let Ok(port_num) = port.as_str().parse::<u16>() {
                config.insert("port".to_string(), json!(port_num));
            }
        }

        Some(config)
    }

    #[allow(dead_code)]
    pub fn construct_connection_url(
        &self,
        config: &serde_json::Map<String, Value>,
        source_type: &str,
    ) -> Option<String> {
        let host = config.get("host")?.as_str()?;
        let database = config.get("database")?.as_str()?;

        let user = config.get("user").and_then(|v| v.as_str()).unwrap_or("");
        let password = config
            .get("password")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let port =
            config
                .get("port")
                .and_then(|v| v.as_u64())
                .unwrap_or(match source_type {
                    "postgresql" | "postgres" => 5432,
                    "mysql" => 3306,
                    "clickhouse" => 8123,
                    _ => 5432,
                });

        let scheme = match source_type {
            "postgresql" | "postgres" => "postgresql",
            "mysql" => "mysql",
            "clickhouse" => "clickhouse",
            _ => source_type,
        };

        if user.is_empty() {
            Some(format!("{}://{}:{}/{}", scheme, host, port, database))
        } else if password.is_empty() {
            Some(format!(
                "{}://{}@{}:{}/{}",
                scheme, user, host, port, database
            ))
        } else {
            Some(format!(
                "{}://{}:{}@{}:{}/{}",
                scheme, user, password, host, port, database
            ))
        }
    }
}
