use super::types::*;
use crate::utils::datasource::create_connector;
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use chrono::{DateTime, Utc};
use uuid;

pub struct McpHandlers {
    pub project_id: String,
    #[allow(dead_code)]
    pub client_id: String,
    pub db_pool: PgPool,
}

impl McpHandlers {
    /// Centralized error logging and formatting for MCP operations
    fn handle_mcp_error(&self, operation: &str, error: Box<dyn std::error::Error + Send + Sync>) -> JsonRpcError {
        let error_msg = error.to_string();
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        
        // Log the error with context
        eprintln!(
            "[{}] [ERROR] MCP operation '{}' failed for project {}: {}", 
            timestamp, 
            operation, 
            self.project_id, 
            error_msg
        );
        
        // Determine error type and provide user-friendly message
        let (code, user_message) = if error_msg.contains("type compatibility") || error_msg.contains("ColumnDecode") {
            (INTERNAL_ERROR, format!("Database compatibility issue during {}: {}", operation, error_msg))
        } else if error_msg.contains("Connection") || error_msg.contains("connect") {
            (INTERNAL_ERROR, format!("Database connection failed during {}: Please check your database configuration", operation))
        } else if error_msg.contains("permission") || error_msg.contains("Permission") {
            (INTERNAL_ERROR, format!("Database permission error during {}: {}", operation, error_msg))
        } else if error_msg.contains("timeout") || error_msg.contains("Timeout") {
            (INTERNAL_ERROR, format!("Database operation timed out during {}: {}", operation, error_msg))
        } else {
            (INTERNAL_ERROR, format!("Unexpected error during {}: {}", operation, error_msg))
        };
        
        JsonRpcError {
            code,
            message: user_message,
            data: Some(json!({
                "operation": operation,
                "timestamp": timestamp.to_string(),
                "raw_error": error_msg
            })),
        }
    }

    /// Wrapper for database operations with consistent error handling
    async fn execute_db_operation<F, T>(&self, operation: &str, f: F) -> Result<T, JsonRpcError>
    where
        F: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
    {
        let start_time = std::time::Instant::now();
        
        match f.await {
            Ok(result) => {
                let duration = start_time.elapsed();
                eprintln!(
                    "[{}] [DEBUG] MCP operation '{}' completed successfully in {}ms", 
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                    operation,
                    duration.as_millis()
                );
                Ok(result)
            }
            Err(error) => {
                let duration = start_time.elapsed();
                eprintln!(
                    "[{}] [ERROR] MCP operation '{}' failed after {}ms", 
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                    operation,
                    duration.as_millis()
                );
                Err(self.handle_mcp_error(operation, error))
            }
        }
    }

    pub async fn handle_initialize(&self, _params: Option<Value>) -> Result<Value, JsonRpcError> {
        eprintln!(
            "[{}] [INFO] Handling initialize request for project: {}", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            self.project_id
        );
        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            server_info: ServerInfo {
                name: "clay-studio-mcp".to_string(),
                version: "0.1.0".to_string(),
            },
            capabilities: Capabilities {
                resources: Some(ResourcesCapability {}),
                tools: Some(ToolsCapability {}),
            },
        };
        
        let response = serde_json::to_value(result).unwrap();
        eprintln!(
            "[{}] [INFO] Initialize completed successfully", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
        Ok(response)
    }
    
    pub async fn handle_resources_list(&self, _params: Option<Value>) -> Result<Value, JsonRpcError> {
        // Query data sources from PostgreSQL
        let sources = sqlx::query(
            "SELECT id, name, source_type 
             FROM data_sources 
             WHERE project_id = $1 AND is_active = true"
        )
        .bind(&self.project_id)
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Database error: {}", e),
            data: None,
        })?;
        
        let resources: Vec<Resource> = sources
            .into_iter()
            .map(|row| {
                let id: String = row.get("id");
                let name: String = row.get("name");
                let source_type: String = row.get("source_type");
                
                Resource {
                    uri: format!("clay://datasource/{}", id),
                    name,
                    mime_type: "application/json".to_string(),
                    description: Some(format!("{} data source", source_type)),
                }
            })
            .collect();
        
        Ok(json!({
            "resources": resources
        }))
    }
    
    pub async fn handle_resources_read(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Missing params".to_string(),
            data: None,
        })?;
        
        let uri = params["uri"].as_str().ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Missing uri parameter".to_string(),
            data: None,
        })?;
        
        // Extract datasource ID from URI (clay://datasource/ID)
        let ds_id = uri.strip_prefix("clay://datasource/")
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Invalid URI format. Expected clay://datasource/ID".to_string(),
                data: None,
            })?;
        
        // Query specific data source
        let source = sqlx::query(
            "SELECT id, name, source_type, connection_config, 
                    schema_info, table_list, is_active, 
                    last_tested_at, created_at
             FROM data_sources 
             WHERE id = $1 AND project_id = $2"
        )
        .bind(ds_id)
        .bind(&self.project_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Database error: {}", e),
            data: None,
        })?;
        
        match source {
            Some(row) => {
                // Build data source info with full connection config (no filtering)
                let connection_config: Value = row.get("connection_config");
                
                let data_source = json!({
                    "id": row.get::<String, _>("id"),
                    "name": row.get::<String, _>("name"),
                    "source_type": row.get::<String, _>("source_type"),
                    "connection_config": connection_config,
                    "schema_info": row.get::<Option<Value>, _>("schema_info"),
                    "table_list": row.get::<Option<Value>, _>("table_list"),
                    "is_active": row.get::<bool, _>("is_active"),
                    "last_tested_at": row.get::<Option<DateTime<Utc>>, _>("last_tested_at")
                        .map(|dt| dt.to_rfc3339()),
                    "created_at": row.get::<DateTime<Utc>, _>("created_at").to_rfc3339(),
                });
                
                let content = ResourceContent {
                    uri: uri.to_string(),
                    mime_type: "application/json".to_string(),
                    text: serde_json::to_string_pretty(&data_source).unwrap(),
                };
                
                Ok(json!({
                    "contents": [content]
                }))
            }
            None => {
                Err(JsonRpcError {
                    code: INVALID_PARAMS,
                    message: "Data source not found. The datasource_id does not exist or has been deleted. Use datasource_list to see available data sources.".to_string(),
                    data: None,
                })
            }
        }
    }
    
    pub async fn handle_tools_list(&self, _params: Option<Value>) -> Result<Value, JsonRpcError> {
        let tools = vec![
            // Datasource Management Tools
            Tool {
                name: "datasource_add".to_string(),
                description: "Add a new data source to the project".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name of the data source"
                        },
                        "source_type": {
                            "type": "string",
                            "description": "Type of data source (postgres, mysql, sqlite, mongodb, etc.)"
                        },
                        "connection_config": {
                            "type": "object",
                            "description": "Connection configuration for the data source. Can provide either 'url' or individual components",
                            "properties": {
                                "url": {
                                    "type": "string",
                                    "description": "Full connection URL (e.g., postgres://user:pass@host:port/db)"
                                },
                                "host": {"type": "string"},
                                "port": {"type": "integer"},
                                "database": {"type": "string"},
                                "username": {"type": "string"},
                                "password": {"type": "string"},
                                "schema": {
                                    "type": "string",
                                    "description": "Database schema to use (PostgreSQL only, defaults to 'public')"
                                }
                            }
                        }
                    },
                    "required": ["name", "source_type", "connection_config"]
                }),
            },
            Tool {
                name: "datasource_list".to_string(),
                description: "List all data sources in the project".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "datasource_remove".to_string(),
                description: "Remove a data source from the project".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the data source to remove"
                        }
                    },
                    "required": ["datasource_id"]
                }),
            },
            Tool {
                name: "datasource_test".to_string(),
                description: "Test connection to a data source".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the data source to test"
                        }
                    },
                    "required": ["datasource_id"]
                }),
            },
            Tool {
                name: "datasource_inspect".to_string(),
                description: "Inspect database structure and return intelligent summary based on size. Call again to refresh if schema changed.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the data source to inspect"
                        },
                        "force_refresh": {
                            "type": "boolean",
                            "description": "Force refresh of cached schema (default: false)",
                            "default": false
                        }
                    },
                    "required": ["datasource_id"]
                }),
            },
            // Schema Query Tools
            Tool {
                name: "schema_get".to_string(),
                description: "Get full schema information for specific tables".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the data source"
                        },
                        "tables": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of table names to get schemas for"
                        }
                    },
                    "required": ["datasource_id", "tables"]
                }),
            },
            Tool {
                name: "schema_search".to_string(),
                description: "Search for tables matching a pattern".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the data source"
                        },
                        "pattern": {
                            "type": "string",
                            "description": "SQL LIKE pattern (use % for wildcards, e.g., '%customer%')"
                        }
                    },
                    "required": ["datasource_id", "pattern"]
                }),
            },
            Tool {
                name: "schema_get_related".to_string(),
                description: "Get schema for a table and all related tables (via foreign keys)".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the data source"
                        },
                        "table": {
                            "type": "string",
                            "description": "Table name to get relationships for"
                        }
                    },
                    "required": ["datasource_id", "table"]
                }),
            },
            Tool {
                name: "schema_stats".to_string(),
                description: "Get database statistics and metadata".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the data source"
                        }
                    },
                    "required": ["datasource_id"]
                }),
            },
            // Data Query Tool
            Tool {
                name: "data_query".to_string(),
                description: "Execute a read-only SQL query on a data source".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the data source to query"
                        },
                        "query": {
                            "type": "string",
                            "description": "SQL query to execute (SELECT only)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of rows to return",
                            "default": 100
                        }
                    },
                    "required": ["datasource_id", "query"]
                }),
            },
        ];
        
        Ok(json!({
            "tools": tools
        }))
    }
    
    pub async fn handle_tools_call(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Missing params".to_string(),
            data: None,
        })?;
        
        let tool_name = params["name"].as_str().ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Missing tool name".to_string(),
            data: None,
        })?;
        
        let arguments = params["arguments"].as_object().ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Missing or invalid arguments".to_string(),
            data: None,
        })?;
        
        let result = match tool_name {
            // Datasource Management
            "datasource_add" => {
                self.add_datasource(arguments).await
            }
            "datasource_list" => {
                self.list_datasources(arguments).await
            }
            "datasource_remove" => {
                self.remove_datasource(arguments).await
            }
            "datasource_test" => {
                self.test_connection(arguments).await
            }
            "datasource_inspect" => {
                self.inspect_datasource(arguments).await
            }
            // Schema Query Tools
            "schema_get" => {
                self.get_schema(arguments).await
            }
            "schema_search" => {
                self.search_schema(arguments).await
            }
            "schema_get_related" => {
                self.get_related_schema(arguments).await
            }
            "schema_stats" => {
                self.get_schema_stats(arguments).await
            }
            // Data Query
            "data_query" => {
                self.query_datasource(arguments).await
            }
            _ => {
                Err(JsonRpcError {
                    code: METHOD_NOT_FOUND,
                    message: format!("Unknown tool: {}", tool_name),
                    data: None,
                })
            }
        };
        
        match result {
            Ok(text) => {
                let tool_result = ToolCallResult {
                    content: vec![ToolContent {
                        content_type: "text".to_string(),
                        text,
                    }],
                    is_error: None,
                };
                Ok(serde_json::to_value(tool_result).unwrap())
            }
            Err(e) => {
                // Extract detailed error information to inform the LLM
                let error_details = if let Some(data) = &e.data {
                    format!(
                        "\n\n🔍 **Error Details:**\n- Operation: {}\n- Timestamp: {}\n- Technical Details: {}", 
                        data.get("operation").and_then(|v| v.as_str()).unwrap_or("unknown"),
                        data.get("timestamp").and_then(|v| v.as_str()).unwrap_or("unknown"),
                        data.get("raw_error").and_then(|v| v.as_str()).unwrap_or("unavailable")
                    )
                } else {
                    String::new()
                };
                
                let detailed_message = format!(
                    "❌ **MCP Server Error**\n\n{}{}\n\n💡 **What this means:** The MCP server encountered an issue while processing your request. This error has been logged with detailed information for debugging.\n\n🔧 **Suggested actions:**\n- Try the operation again\n- Check if the database connection is stable\n- If the error persists, this may indicate a database compatibility or configuration issue",
                    e.message,
                    error_details
                );
                
                let tool_result = ToolCallResult {
                    content: vec![ToolContent {
                        content_type: "text".to_string(),
                        text: detailed_message,
                    }],
                    is_error: Some(true),
                };
                Ok(serde_json::to_value(tool_result).unwrap())
            }
        }
    }
    
    // Tool implementations
    async fn add_datasource(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        eprintln!(
            "[{}] [INFO] Adding datasource for project: {}", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            self.project_id
        );
        let name = args.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing name".to_string(),
                data: None,
            })?;
        
        let source_type = args.get("source_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing source_type".to_string(),
                data: None,
            })?;
        
        let connection_config_input = args.get("connection_config")
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing connection_config".to_string(),
                data: None,
            })?;
        
        // Parse and enhance connection config
        let connection_config = self.parse_connection_config(connection_config_input, source_type)?;
        
        // Debug: Log the parsed connection config
        eprintln!(
            "[{}] [DEBUG] Parsed connection config: {}", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            serde_json::to_string_pretty(&connection_config).unwrap_or_else(|_| "failed to serialize".to_string())
        );
        
        // Generate a new UUID for the datasource
        let datasource_id = uuid::Uuid::new_v4().to_string();
        eprintln!(
            "[{}] [DEBUG] Generated datasource ID: {} for name: '{}' type: '{}'", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            datasource_id, 
            name, 
            source_type
        );
        
        // Insert new data source into database
        sqlx::query(
            "INSERT INTO data_sources (id, project_id, name, source_type, connection_config, is_active, created_at) 
             VALUES ($1, $2, $3, $4, $5, true, NOW())"
        )
        .bind(&datasource_id)
        .bind(&self.project_id)
        .bind(name)
        .bind(source_type)
        .bind(connection_config)
        .execute(&self.db_pool)
        .await
        .map_err(|e| {
            eprintln!(
                "[{}] [ERROR] Failed to insert datasource '{}': {}", 
                Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                name, 
                e
            );
            JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to add data source: {}", e),
                data: None,
            }
        })?;
        
        eprintln!(
            "[{}] [INFO] Successfully added datasource '{}' ({}) with ID: {}", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            name, 
            source_type, 
            datasource_id
        );
        
        Ok(format!(
            "✅ Data source '{}' ({}) added successfully with ID: {}",
            name, source_type, datasource_id
        ))
    }
    
    async fn list_datasources(&self, _args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        // Query all data sources for this project
        let sources = sqlx::query(
            "SELECT id, name, source_type, is_active, last_tested_at, created_at 
             FROM data_sources 
             WHERE project_id = $1 
             ORDER BY created_at DESC"
        )
        .bind(&self.project_id)
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Database error: {}", e),
            data: None,
        })?;
        
        if sources.is_empty() {
            return Ok("📊 No data sources configured\n\nThis project has no data sources. Use datasource_add to connect a database.".to_string());
        }
        
        let mut result = format!("Data sources ({} total):\n", sources.len());
        for row in sources {
            let id: String = row.get("id");
            let name: String = row.get("name");
            let source_type: String = row.get("source_type");
            let is_active: bool = row.get("is_active");
            let last_tested: Option<DateTime<Utc>> = row.get("last_tested_at");
            
            result.push_str(&format!(
                "\n• {} ({})\n  ID: {}\n  Status: {}\n  Last tested: {}\n",
                name,
                source_type,
                id,
                if is_active { "Active" } else { "Inactive" },
                last_tested.map(|dt| dt.to_rfc3339()).unwrap_or_else(|| "Never".to_string())
            ));
        }
        
        Ok(result)
    }
    
    async fn remove_datasource(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        let datasource_id = args.get("datasource_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing datasource_id".to_string(),
                data: None,
            })?;
        
        // First check if the data source exists and get its name
        let source = sqlx::query(
            "SELECT name FROM data_sources WHERE id = $1 AND project_id = $2"
        )
        .bind(datasource_id)
        .bind(&self.project_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Database error: {}", e),
            data: None,
        })?;
        
        match source {
            Some(row) => {
                let name: String = row.get("name");
                
                // Delete the data source
                sqlx::query(
                    "DELETE FROM data_sources WHERE id = $1 AND project_id = $2"
                )
                .bind(datasource_id)
                .bind(&self.project_id)
                .execute(&self.db_pool)
                .await
                .map_err(|e| JsonRpcError {
                    code: INTERNAL_ERROR,
                    message: format!("Failed to remove data source: {}", e),
                    data: None,
                })?;
                
                Ok(format!("✅ Data source '{}' removed successfully", name))
            }
            None => {
                Err(JsonRpcError {
                    code: INVALID_PARAMS,
                    message: "Data source not found. The datasource_id does not exist or has been deleted. Use datasource_list to see available data sources.".to_string(),
                    data: None,
                })
            }
        }
    }
    
    async fn test_connection(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        let datasource_id = args.get("datasource_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing datasource_id".to_string(),
                data: None,
            })?;
        
        // Get data source from database
        let source = sqlx::query(
            "SELECT name, source_type, connection_config 
             FROM data_sources 
             WHERE id = $1 AND project_id = $2"
        )
        .bind(datasource_id)
        .bind(&self.project_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Database error: {}", e),
            data: None,
        })?
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Data source not found. The datasource_id does not exist or has been deleted. Use datasource_list to see available data sources.".to_string(),
            data: None,
        })?;
        
        let name: String = source.get("name");
        let source_type: String = source.get("source_type");
        let connection_config: Value = source.get("connection_config");
        
        eprintln!(
            "[{}] [INFO] Testing connection to datasource '{}' ({})", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            name, 
            source_type
        );
        
        // Actually test the connection using the appropriate connector
        let connector_start = std::time::Instant::now();
        match create_connector(&source_type, &connection_config).await {
            Ok(connector) => {
                let connector_duration = connector_start.elapsed();
                eprintln!(
                    "[{}] [DEBUG] Connector created in {}ms, testing connection...", 
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                    connector_duration.as_millis()
                );
                
                // Try to test the actual connection
                let test_start = std::time::Instant::now();
                match connector.test_connection().await {
                    Ok(true) => {
                        let test_duration = test_start.elapsed();
                        eprintln!(
                            "[{}] [INFO] Connection test successful for '{}' in {}ms", 
                            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                            name, 
                            test_duration.as_millis()
                        );
                        
                        // Connection successful - update last_tested_at and fetch schema
                        sqlx::query(
                            "UPDATE data_sources 
                             SET last_tested_at = NOW(), is_active = true 
                             WHERE id = $1"
                        )
                        .bind(datasource_id)
                        .execute(&self.db_pool)
                        .await
                        .map_err(|e| JsonRpcError {
                            code: INTERNAL_ERROR,
                            message: format!("Failed to update data source status: {}", e),
                            data: None,
                        })?;
                        
                        // Try to fetch schema and analysis information
                        let mut schema_info = String::new();
                        let mut combined_result = json!({});
                        
                        // Fetch the actual database schema
                        if let Ok(schema) = connector.fetch_schema().await {
                            // Count tables for the success message
                            if let Some(tables) = schema.get("tables").and_then(|t| t.as_object()) {
                                schema_info = format!(" Found {} tables.", tables.len());
                            }
                            combined_result["schema"] = schema;
                        }
                        
                        // Fetch database analysis
                        if let Ok(analysis) = connector.analyze_database().await {
                            // Merge analysis into combined result
                            if let Some(analysis_obj) = analysis.as_object() {
                                for (key, value) in analysis_obj {
                                    combined_result[key] = value.clone();
                                }
                            }
                        }
                        
                        // Update schema information in the database with combined result
                        if !combined_result.as_object().unwrap().is_empty() {
                            sqlx::query(
                                "UPDATE data_sources 
                                 SET schema_info = $1 
                                 WHERE id = $2"
                            )
                            .bind(&combined_result)
                            .bind(datasource_id)
                            .execute(&self.db_pool)
                            .await.ok();
                        }
                        
                        // Try to fetch and update table list
                        if let Ok(tables) = connector.list_tables().await {
                            let table_list = json!(tables);
                            sqlx::query(
                                "UPDATE data_sources 
                                 SET table_list = $1 
                                 WHERE id = $2"
                            )
                            .bind(&table_list)
                            .bind(datasource_id)
                            .execute(&self.db_pool)
                            .await.ok();
                        }
                        
                        Ok(format!(
                            "✅ Connection successful for '{}' ({} data source).{}",
                            name, source_type, schema_info
                        ))
                    }
                    Ok(false) => {
                        let test_duration = test_start.elapsed();
                        eprintln!(
                            "[{}] [WARNING] Connection test returned false for '{}' after {}ms", 
                            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                            name, 
                            test_duration.as_millis()
                        );
                        
                        // Connection failed - mark as inactive
                        sqlx::query(
                            "UPDATE data_sources 
                             SET is_active = false 
                             WHERE id = $1"
                        )
                        .bind(datasource_id)
                        .execute(&self.db_pool)
                        .await.ok();
                        
                        Ok(format!(
                            "❌ Connection test returned false for '{}' ({} data source)",
                            name, source_type
                        ))
                    }
                    Err(e) => {
                        let test_duration = test_start.elapsed();
                        eprintln!(
                            "[{}] [ERROR] Connection test failed for '{}' after {}ms: {}", 
                            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                            name, 
                            test_duration.as_millis(), 
                            e
                        );
                        
                        // Connection error - mark as inactive
                        sqlx::query(
                            "UPDATE data_sources 
                             SET is_active = false 
                             WHERE id = $1"
                        )
                        .bind(datasource_id)
                        .execute(&self.db_pool)
                        .await.ok();
                        
                        Ok(format!(
                            "❌ Connection failed for '{}' ({} data source): {}",
                            name, source_type, e
                        ))
                    }
                }
            }
            Err(e) => {
                let connector_duration = connector_start.elapsed();
                eprintln!(
                    "[{}] [ERROR] Failed to create connector for '{}' ({}) after {}ms: {}", 
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                    name, 
                    source_type, 
                    connector_duration.as_millis(), 
                    e
                );
                
                // Failed to create connector
                Ok(format!(
                    "❌ Failed to create connector for '{}' ({} data source): {}",
                    name, source_type, e
                ))
            }
        }
    }
    async fn query_datasource(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        eprintln!(
            "[{}] [INFO] Executing query for project: {}", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            self.project_id
        );
        
        // Log all received parameters for debugging
        eprintln!(
            "[{}] [DEBUG] Received parameters: {}", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            serde_json::to_string_pretty(args).unwrap_or_else(|_| "<error>".to_string())
        );
        
        // Extract context parameters if provided
        let conversation_id = args.get("_conversation_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let message_id = args.get("_message_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        eprintln!(
            "[{}] [DEBUG] Tool context - conversation_id: {:?}, message_id: {:?}", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            conversation_id,
            message_id
        );
        
        let datasource_id = args.get("datasource_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing datasource_id".to_string(),
                data: None,
            })?;
        
        let query = args.get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing query".to_string(),
                data: None,
            })?;
        
        let limit = args.get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(100) as i32;
        
        eprintln!(
            "[{}] [DEBUG] Query request - datasource: {}, query: {}, limit: {}", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            datasource_id, 
            query, 
            limit
        );
        
        // Security: Only allow SELECT queries
        let query_lower = query.to_lowercase();
        if !query_lower.trim().starts_with("select") {
            eprintln!(
                "[{}] [WARNING] Non-SELECT query attempted: {}", 
                Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                query
            );
            return Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: "Only SELECT queries are allowed".to_string(),
                data: None,
            });
        }
        
        // Get data source with connection config
        let source = sqlx::query(
            "SELECT name, source_type, connection_config FROM data_sources WHERE id = $1 AND project_id = $2"
        )
        .bind(datasource_id)
        .bind(&self.project_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Database error: {}", e),
            data: None,
        })?
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Data source not found. The datasource_id does not exist or has been deleted. Use datasource_list to see available data sources.".to_string(),
            data: None,
        })?;
        
        let name: String = source.get("name");
        let source_type: String = source.get("source_type");
        let connection_config: Value = source.get("connection_config");
        
        // Create connector and execute real query
        let connector = create_connector(&source_type, &connection_config)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to create connector: {}", e),
                data: None,
            })?;
        
        // Execute query
        eprintln!(
            "[{}] [DEBUG] Executing query on datasource '{}'", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            name
        );
        let query_start = std::time::Instant::now();
        let result = connector.execute_query(query, limit)
            .await
            .map_err(|e| {
                eprintln!(
                    "[{}] [ERROR] Query execution failed on '{}': {}", 
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                    name, 
                    e
                );
                JsonRpcError {
                    code: INTERNAL_ERROR,
                    message: format!("Query execution failed: {}", e),
                    data: None,
                }
            })?;
        
        let query_duration = query_start.elapsed();
        eprintln!(
            "[{}] [INFO] Query executed successfully on '{}' in {}ms", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            name, 
            query_duration.as_millis()
        );
        
        // Update tool_usages table based on context or parameter matching
        eprintln!(
            "[{}] [TIMING] MCP attempting to update tool_usage at {:?}", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            chrono::Utc::now()
        );
        
        // Try to update tool_usage with retries (in case it's still being saved)
        let mut update_result = None;
        let max_retries = 3;
        let retry_delay = std::time::Duration::from_millis(500);
        
        for attempt in 0..max_retries {
            if attempt > 0 {
                eprintln!(
                    "[{}] [DEBUG] Retry attempt {} after {}ms delay", 
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                    attempt,
                    retry_delay.as_millis()
                );
                tokio::time::sleep(retry_delay).await;
            }
            
            if let Some(ref msg_id) = message_id {
                // We have exact message_id - use it for precise matching
                eprintln!(
                    "[{}] [TIMING] Using message_id {} for precise tool_usage matching (attempt {})", 
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                    msg_id,
                    attempt + 1
                );
                
                update_result = sqlx::query(
                    "UPDATE tool_usages 
                     SET output = $1,
                         execution_time_ms = $2
                     WHERE message_id = $3
                       AND (tool_name LIKE '%data_query%')
                       AND (output->>'status' = 'executing' OR output IS NULL)
                     ORDER BY created_at DESC
                     LIMIT 1
                     RETURNING id"
                )
                .bind(json!({
                    "status": "success",
                    "result": result,
                    "datasource": name,
                    "query": query,
                    "row_count": result.get("rows").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
                .bind(query_duration.as_millis() as i32)
                .bind(msg_id)
                .fetch_optional(&self.db_pool)
                .await.ok();
                
                if update_result.is_some() && update_result.as_ref().unwrap().is_some() {
                    break; // Successfully found and updated
                }
            } else {
                // No message_id, try parameter matching
                break; // Don't retry for parameter matching
            }
        }
        
        if let Some(msg_id) = message_id {
            let update_result = update_result.unwrap_or(None);
            
            match update_result {
                Some(row) => {
                    let id: uuid::Uuid = row.get("id");
                    eprintln!(
                        "[{}] [INFO] Successfully updated tool_usage {} with query results (rows: {})", 
                        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                        id,
                        result.get("rows").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0)
                    );
                },
                None => {
                    eprintln!(
                        "[{}] [WARNING] No matching tool_usage found for message_id: {} after {} retries", 
                        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                        msg_id,
                        max_retries
                    );
                }
            }
        } else {
            // Fallback to parameter matching if no message_id provided
            eprintln!(
                "[{}] [WARNING] No message_id provided, falling back to parameter matching with retries", 
                Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
            );
            
            // Build the parameters JSON to match against (excluding context params)
            let expected_params = json!({
                "datasource_id": datasource_id,
                "query": query,
                "limit": limit
            });
            
            // Try with retries for parameter matching too
            let mut update_result = None;
            for attempt in 0..max_retries {
                if attempt > 0 {
                    eprintln!(
                        "[{}] [DEBUG] Parameter matching retry attempt {} after {}ms delay", 
                        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                        attempt,
                        retry_delay.as_millis()
                    );
                    tokio::time::sleep(retry_delay).await;
                }
                
                // Find and update based on parameter matching (less precise)
                update_result = sqlx::query(
                    "UPDATE tool_usages 
                     SET output = $1,
                         execution_time_ms = $2
                     WHERE id = (
                         SELECT id FROM tool_usages 
                         WHERE (tool_name LIKE '%data_query%')
                           AND (parameters @> $3)
                           AND (output->>'status' = 'executing' OR output IS NULL)
                           AND created_at > NOW() - INTERVAL '10 seconds'
                         ORDER BY created_at DESC
                         LIMIT 1
                     )
                     RETURNING id, message_id"
                )
                .bind(json!({
                    "status": "success",
                    "result": result,
                    "datasource": name,
                    "query": query,
                    "row_count": result.get("rows").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
                .bind(query_duration.as_millis() as i32)
                .bind(&expected_params)
                .fetch_optional(&self.db_pool)
                .await.ok();
                
                if update_result.is_some() && update_result.as_ref().unwrap().is_some() {
                    break; // Successfully found and updated
                }
            }
            
            let update_result = update_result.unwrap_or(None);
            
            match update_result {
                Some(row) => {
                    let id: uuid::Uuid = row.get("id");
                    let msg_id: String = row.get("message_id");
                    eprintln!(
                        "[{}] [INFO] Successfully updated tool_usage {} via parameter matching for message {}", 
                        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                        id,
                        msg_id
                    );
                },
                None => {
                    eprintln!(
                        "[{}] [WARNING] No matching tool_usage found via parameter matching after {} retries", 
                        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                        max_retries
                    );
                }
            }
        }
        
        Ok(format!(
            "Query executed on '{}' (limited to {} rows):\n{}",
            name, limit, serde_json::to_string_pretty(&result).unwrap()
        ))
    }
    
    // New tool implementations
    async fn inspect_datasource(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        let datasource_id = args.get("datasource_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing datasource_id".to_string(),
                data: None,
            })?;
        
        // Use centralized error handling for the inspection operation
        self.execute_db_operation("datasource_inspect", async {
            self.inspect_datasource_internal(datasource_id).await
        }).await
    }

    async fn inspect_datasource_internal(&self, datasource_id: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        
        // Get data source info and check if schema_info already exists
        let source = sqlx::query(
            "SELECT name, source_type, connection_config, schema_info, table_list, last_tested_at 
             FROM data_sources 
             WHERE id = $1 AND project_id = $2"
        )
        .bind(datasource_id)
        .bind(&self.project_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { 
            format!("Database error: {}", e).into() 
        })?
        .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
            "Data source not found. The specified datasource_id does not exist. Use datasource_list to see available data sources.".to_string().into()
        })?;
        
        let name: String = source.get("name");
        let source_type: String = source.get("source_type");
        let connection_config: Value = source.get("connection_config");
        let existing_schema: Option<Value> = source.get("schema_info");
        
        // Check if we have valid schema info with actual schema data
        if let Some(schema_info) = existing_schema {
            // Check if the schema_info contains the actual "schema" field with table definitions
            if schema_info.get("schema").is_some() && schema_info.get("schema").unwrap().get("tables").is_some() {
                return Ok(self.format_inspection_result(&name, &schema_info));
            }
            // If we have old format data without schema, continue to fetch fresh data
            eprintln!(
                "[{}] [INFO] Existing schema_info lacks 'schema' field, fetching fresh data for datasource '{}'", 
                Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                name
            );
        }
        
        // No cached analysis - perform fresh inspection
        let connector = create_connector(&source_type, &connection_config)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { 
                format!("Failed to create connector: {}", e).into() 
            })?;
        
        // Fetch the actual database schema
        eprintln!(
            "[{}] [INFO] Fetching schema for datasource '{}'", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            name
        );
        let schema = connector.fetch_schema()
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { 
                eprintln!(
                    "[{}] [ERROR] Failed to fetch schema for '{}': {}", 
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                    name,
                    e
                );
                format!("Failed to fetch schema: {}", e).into() 
            })?;
        
        eprintln!(
            "[{}] [DEBUG] Schema fetched, tables count: {}", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            schema.get("tables").and_then(|t| t.as_object()).map(|t| t.len()).unwrap_or(0)
        );
        
        // Analyze the database for statistics
        eprintln!(
            "[{}] [INFO] Analyzing database for datasource '{}'", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            name
        );
        let analysis = connector.analyze_database()
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { 
                eprintln!(
                    "[{}] [ERROR] Failed to analyze database for '{}': {}", 
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                    name,
                    e
                );
                format!("Failed to analyze database: {}", e).into() 
            })?;
        
        // Combine schema and analysis into a comprehensive result
        let mut combined_result = analysis.clone();
        if let Some(obj) = combined_result.as_object_mut() {
            obj.insert("schema".to_string(), schema);
        }
        
        eprintln!(
            "[{}] [INFO] Storing combined schema and analysis for datasource '{}'", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            name
        );
        
        // Store combined result in schema_info column
        let update_result = sqlx::query(
            "UPDATE data_sources 
             SET schema_info = $1, 
                 table_list = $2,
                 last_tested_at = NOW()
             WHERE id = $3"
        )
        .bind(&combined_result)
        .bind(combined_result.get("table_names").cloned())
        .bind(datasource_id)
        .execute(&self.db_pool)
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { 
            eprintln!(
                "[{}] [ERROR] Failed to store schema for '{}': {}", 
                Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                name,
                e
            );
            format!("Failed to store analysis: {}", e).into() 
        })?;
        
        eprintln!(
            "[{}] [SUCCESS] Schema and analysis stored for datasource '{}', {} rows affected", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            name,
            update_result.rows_affected()
        );
        
        Ok(self.format_inspection_result(&name, &combined_result))
    }
    
    async fn get_schema(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        let datasource_id = args.get("datasource_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing datasource_id".to_string(),
                data: None,
            })?;
        
        let tables = args.get("tables")
            .and_then(|v| v.as_array())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing or invalid tables array".to_string(),
                data: None,
            })?;
        
        let table_names: Vec<&str> = tables
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        
        if table_names.is_empty() {
            return Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: "Tables array cannot be empty".to_string(),
                data: None,
            });
        }
        
        // Get connector
        let source = self.get_datasource_connector(datasource_id).await?;
        let connector = create_connector(&source.source_type, &source.connection_config)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to create connector: {}", e),
                data: None,
            })?;
        
        // Get schemas for specified tables
        let schemas = connector.get_tables_schema(table_names)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to get schemas: {}", e),
                data: None,
            })?;
        
        Ok(format!(
            "Schema for tables in '{}':\n{}",
            source.name,
            serde_json::to_string_pretty(&schemas).unwrap()
        ))
    }
    
    async fn search_schema(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        let datasource_id = args.get("datasource_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing datasource_id".to_string(),
                data: None,
            })?;
        
        let pattern = args.get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing pattern".to_string(),
                data: None,
            })?;
        
        // Get connector
        let source = self.get_datasource_connector(datasource_id).await?;
        let connector = create_connector(&source.source_type, &source.connection_config)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to create connector: {}", e),
                data: None,
            })?;
        
        // Search for tables
        let results = connector.search_tables(pattern)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to search tables: {}", e),
                data: None,
            })?;
        
        Ok(format!(
            "Tables matching '{}' in '{}':\n{}",
            pattern,
            source.name,
            serde_json::to_string_pretty(&results).unwrap()
        ))
    }
    
    async fn get_related_schema(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        let datasource_id = args.get("datasource_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing datasource_id".to_string(),
                data: None,
            })?;
        
        let table = args.get("table")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing table name".to_string(),
                data: None,
            })?;
        
        // Get connector
        let source = self.get_datasource_connector(datasource_id).await?;
        let connector = create_connector(&source.source_type, &source.connection_config)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to create connector: {}", e),
                data: None,
            })?;
        
        // Get related tables
        let related = connector.get_related_tables(table)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to get related tables: {}", e),
                data: None,
            })?;
        
        Ok(format!(
            "Table '{}' and its relationships in '{}':\n{}",
            table,
            source.name,
            serde_json::to_string_pretty(&related).unwrap()
        ))
    }
    
    async fn get_schema_stats(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        let datasource_id = args.get("datasource_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing datasource_id".to_string(),
                data: None,
            })?;
        
        // Get connector
        let source = self.get_datasource_connector(datasource_id).await?;
        let connector = create_connector(&source.source_type, &source.connection_config)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to create connector: {}", e),
                data: None,
            })?;
        
        // Get database statistics
        let stats = connector.get_database_stats()
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to get database statistics: {}", e),
                data: None,
            })?;
        
        Ok(format!(
            "Database statistics for '{}':\n{}",
            source.name,
            serde_json::to_string_pretty(&stats).unwrap()
        ))
    }
    
    // Helper methods
    async fn get_datasource_connector(&self, datasource_id: &str) -> Result<DataSourceInfo, JsonRpcError> {
        let source = sqlx::query(
            "SELECT name, source_type, connection_config 
             FROM data_sources 
             WHERE id = $1 AND project_id = $2"
        )
        .bind(datasource_id)
        .bind(&self.project_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Database error: {}", e),
            data: None,
        })?
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Data source not found. The datasource_id does not exist or has been deleted. Use datasource_list to see available data sources.".to_string(),
            data: None,
        })?;
        
        Ok(DataSourceInfo {
            name: source.get("name"),
            source_type: source.get("source_type"),
            connection_config: source.get("connection_config"),
        })
    }
    
    fn format_inspection_result(&self, name: &str, analysis: &Value) -> String {
        let table_count = analysis["statistics"]["table_count"]
            .as_u64()
            .unwrap_or(0);
        
        let total_size = analysis["statistics"]["total_size_human"]
            .as_str()
            .unwrap_or("unknown");
        
        let total_rows = analysis["statistics"]["total_rows"]
            .as_u64()
            .unwrap_or(0);
        
        // Check if we have the actual schema
        let has_schema = analysis.get("schema").is_some();
        
        // Determine strategy based on table count
        let strategy = if table_count <= 20 {
            "full"
        } else if table_count <= 100 {
            "summary"
        } else {
            "statistical"
        };
        
        // Build response based on strategy
        match strategy {
            "full" => {
                // For small databases, include full schema if available
                if has_schema {
                    format!(
                        "Database '{}' inspection (Full Schema - {} tables, {}):\n\n{}",
                        name,
                        table_count,
                        total_size,
                        serde_json::to_string_pretty(&analysis).unwrap()
                    )
                } else {
                    format!(
                        "Database '{}' inspection (Analysis - {} tables, {}):\n\n{}",
                        name,
                        table_count,
                        total_size,
                        serde_json::to_string_pretty(&analysis).unwrap()
                    )
                }
            },
            "summary" => {
                // For medium databases, show key tables
                let mut result = format!(
                    "Database '{}' inspection (Summary - {} tables, {}, {} rows):\n\n",
                    name, table_count, total_size, total_rows
                );
                
                result.push_str("Key Tables (showing top 10 by importance):\n");
                if let Some(tables) = analysis["key_tables"].as_array() {
                    for table in tables.iter().take(10) {
                        if let Some(name) = table["name"].as_str() {
                            let row_count = table["row_count"].as_u64().unwrap_or(0);
                            let connections = table["connections"].as_u64().unwrap_or(0);
                            result.push_str(&format!(
                                "  • {} ({} rows, {} connections)\n",
                                name, row_count, connections
                            ));
                        }
                    }
                }
                
                result.push_str("\nUse schema_get to see specific table details.\n");
                result.push_str("Use schema_search to find tables by pattern.\n");
                result
            },
            _ => {
                // For large databases, show statistics
                format!(
                    "Database '{}' inspection (Statistical Overview - {} tables, {}):\n\n\
                    This is a large database. Use these tools to explore:\n\
                    • schema_search(pattern) - Find tables by name pattern\n\
                    • schema_get(tables) - Get specific table schemas\n\
                    • schema_get_related(table) - Get table with relationships\n\
                    • schema_stats() - Get detailed statistics\n\n\
                    Top tables by size:\n{}",
                    name,
                    table_count,
                    total_size,
                    self.format_top_tables(analysis)
                )
            }
        }
    }
    
    fn format_top_tables(&self, analysis: &Value) -> String {
        let mut result = String::new();
        if let Some(tables) = analysis["largest_tables"].as_array() {
            for table in tables.iter().take(5) {
                if let Some(name) = table["name"].as_str() {
                    let size = table["size_human"].as_str().unwrap_or("unknown");
                    let rows = table["row_count"].as_u64().unwrap_or(0);
                    result.push_str(&format!("  • {} ({}, {} rows)\n", name, size, rows));
                }
            }
        }
        result
    }
    
    // Helper method to parse connection config - handles both URL and individual components
    fn parse_connection_config(&self, config: &Value, source_type: &str) -> Result<Value, JsonRpcError> {
        let config_obj = config.as_object().ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "connection_config must be an object".to_string(),
            data: None,
        })?;
        
        // If URL is provided, parse it into individual components while keeping the original URL
        if let Some(url) = config_obj.get("url").and_then(|v| v.as_str()) {
            let parsed = self.parse_connection_url(url, source_type)?;
            
            // Merge parsed components with any existing individual components (individual components take precedence)
            let mut result = parsed;
            for (key, value) in config_obj {
                result.insert(key.clone(), value.clone());
            }
            
            Ok(json!(result))
        } else {
            // No URL provided, just use the individual components as-is
            // But try to construct a URL from individual components for storage
            let mut result = config_obj.clone();
            
            if let Some(constructed_url) = self.construct_connection_url(config_obj, source_type) {
                result.insert("url".to_string(), json!(constructed_url));
            }
            
            Ok(json!(result))
        }
    }
    
    // Parse connection URL into individual components
    fn parse_connection_url(&self, url: &str, source_type: &str) -> Result<serde_json::Map<String, Value>, JsonRpcError> {
        let mut components = serde_json::Map::new();
        components.insert("url".to_string(), json!(url));
        
        // Parse based on source type
        match source_type.to_lowercase().as_str() {
            "postgres" | "postgresql" => {
                if let Some(parsed) = self.parse_postgres_url(url) {
                    components.extend(parsed);
                } else {
                    return Err(JsonRpcError {
                        code: INVALID_PARAMS,
                        message: "Invalid PostgreSQL connection URL format".to_string(),
                        data: None,
                    });
                }
            }
            "mysql" => {
                if let Some(parsed) = self.parse_mysql_url(url) {
                    components.extend(parsed);
                } else {
                    return Err(JsonRpcError {
                        code: INVALID_PARAMS,
                        message: "Invalid MySQL connection URL format".to_string(),
                        data: None,
                    });
                }
            }
            "sqlite" => {
                components.insert("path".to_string(), json!(url.replace("sqlite://", "")));
            }
            _ => {
                // For unknown types, try generic URL parsing
                if let Some(parsed) = self.parse_generic_url(url) {
                    components.extend(parsed);
                }
            }
        }
        
        Ok(components)
    }
    
    // Parse PostgreSQL URL format: postgres://user:pass@host:port/database
    fn parse_postgres_url(&self, url: &str) -> Option<serde_json::Map<String, Value>> {
        let url = url.strip_prefix("postgres://").or_else(|| url.strip_prefix("postgresql://"))?;
        
        let mut components = serde_json::Map::new();
        
        // Split by @ to separate auth and host parts
        let parts: Vec<&str> = url.splitn(2, '@').collect();
        if parts.len() != 2 {
            return None;
        }
        
        // Parse auth part (user:pass)
        let auth_parts: Vec<&str> = parts[0].splitn(2, ':').collect();
        if !auth_parts.is_empty() {
            // URL decode the username
            let username = urlencoding::decode(auth_parts[0]).unwrap_or_else(|_| auth_parts[0].into());
            components.insert("username".to_string(), json!(username.to_string()));
        }
        if auth_parts.len() >= 2 {
            // URL decode the password to handle special characters
            let password = urlencoding::decode(auth_parts[1]).unwrap_or_else(|_| auth_parts[1].into());
            components.insert("password".to_string(), json!(password.to_string()));
        }
        
        // Parse host part (host:port/database)
        let host_part = parts[1];
        let host_db_parts: Vec<&str> = host_part.splitn(2, '/').collect();
        
        if host_db_parts.len() >= 2 {
            components.insert("database".to_string(), json!(host_db_parts[1]));
        }
        
        // Parse host:port
        if let Some(host_port) = host_db_parts.first() {
            let host_port_parts: Vec<&str> = host_port.splitn(2, ':').collect();
            components.insert("host".to_string(), json!(host_port_parts[0]));
            
            if host_port_parts.len() >= 2 {
                if let Ok(port) = host_port_parts[1].parse::<i32>() {
                    components.insert("port".to_string(), json!(port));
                }
            } else {
                components.insert("port".to_string(), json!(5432)); // Default PostgreSQL port
            }
        }
        
        Some(components)
    }
    
    // Parse MySQL URL format: mysql://user:pass@host:port/database
    fn parse_mysql_url(&self, url: &str) -> Option<serde_json::Map<String, Value>> {
        let url = url.strip_prefix("mysql://")?;
        
        let mut components = serde_json::Map::new();
        
        // Split by @ to separate auth and host parts
        let parts: Vec<&str> = url.splitn(2, '@').collect();
        if parts.len() != 2 {
            return None;
        }
        
        // Parse auth part (user:pass)
        let auth_parts: Vec<&str> = parts[0].splitn(2, ':').collect();
        if !auth_parts.is_empty() {
            // URL decode the username
            let username = urlencoding::decode(auth_parts[0]).unwrap_or_else(|_| auth_parts[0].into());
            components.insert("username".to_string(), json!(username.to_string()));
        }
        if auth_parts.len() >= 2 {
            // URL decode the password to handle special characters
            let password = urlencoding::decode(auth_parts[1]).unwrap_or_else(|_| auth_parts[1].into());
            components.insert("password".to_string(), json!(password.to_string()));
        }
        
        // Parse host part (host:port/database)
        let host_part = parts[1];
        let host_db_parts: Vec<&str> = host_part.splitn(2, '/').collect();
        
        if host_db_parts.len() >= 2 {
            components.insert("database".to_string(), json!(host_db_parts[1]));
        }
        
        // Parse host:port
        if let Some(host_port) = host_db_parts.first() {
            let host_port_parts: Vec<&str> = host_port.splitn(2, ':').collect();
            components.insert("host".to_string(), json!(host_port_parts[0]));
            
            if host_port_parts.len() >= 2 {
                if let Ok(port) = host_port_parts[1].parse::<i32>() {
                    components.insert("port".to_string(), json!(port));
                }
            } else {
                components.insert("port".to_string(), json!(3306)); // Default MySQL port
            }
        }
        
        Some(components)
    }
    
    // Generic URL parser for unknown database types
    fn parse_generic_url(&self, url: &str) -> Option<serde_json::Map<String, Value>> {
        // Try to parse as a generic URL format: scheme://user:pass@host:port/path
        if let Some(scheme_end) = url.find("://") {
            let after_scheme = &url[scheme_end + 3..];
            
            let mut components = serde_json::Map::new();
            
            // Split by @ to separate auth and host parts
            if let Some(at_pos) = after_scheme.find('@') {
                let auth_part = &after_scheme[..at_pos];
                let host_part = &after_scheme[at_pos + 1..];
                
                // Parse auth
                if let Some(colon_pos) = auth_part.find(':') {
                    components.insert("username".to_string(), json!(&auth_part[..colon_pos]));
                    components.insert("password".to_string(), json!(&auth_part[colon_pos + 1..]));
                } else {
                    components.insert("username".to_string(), json!(auth_part));
                }
                
                // Parse host/port/path
                if let Some(slash_pos) = host_part.find('/') {
                    let host_port = &host_part[..slash_pos];
                    let path = &host_part[slash_pos + 1..];
                    components.insert("database".to_string(), json!(path));
                    
                    if let Some(colon_pos) = host_port.find(':') {
                        components.insert("host".to_string(), json!(&host_port[..colon_pos]));
                        if let Ok(port) = host_port[colon_pos + 1..].parse::<i32>() {
                            components.insert("port".to_string(), json!(port));
                        }
                    } else {
                        components.insert("host".to_string(), json!(host_port));
                    }
                } else if let Some(colon_pos) = host_part.find(':') {
                    components.insert("host".to_string(), json!(&host_part[..colon_pos]));
                    if let Ok(port) = host_part[colon_pos + 1..].parse::<i32>() {
                        components.insert("port".to_string(), json!(port));
                    }
                } else {
                    components.insert("host".to_string(), json!(host_part));
                }
            }
            
            Some(components)
        } else {
            None
        }
    }
    
    // Construct connection URL from individual components
    fn construct_connection_url(&self, config: &serde_json::Map<String, Value>, source_type: &str) -> Option<String> {
        match source_type.to_lowercase().as_str() {
            "postgres" | "postgresql" => {
                let host = config.get("host")?.as_str()?;
                let port = config.get("port").and_then(|v| v.as_i64()).unwrap_or(5432);
                let database = config.get("database")?.as_str()?;
                let username = config.get("username")?.as_str()?;
                let password = config.get("password").and_then(|v| v.as_str()).unwrap_or("");
                
                // URL encode username and password to handle special characters
                let encoded_username = urlencoding::encode(username);
                let encoded_password = if password.is_empty() {
                    String::new()
                } else {
                    urlencoding::encode(password).to_string()
                };
                
                if encoded_password.is_empty() {
                    Some(format!("postgres://{}@{}:{}/{}", encoded_username, host, port, database))
                } else {
                    Some(format!("postgres://{}:{}@{}:{}/{}", encoded_username, encoded_password, host, port, database))
                }
            }
            "mysql" => {
                let host = config.get("host")?.as_str()?;
                let port = config.get("port").and_then(|v| v.as_i64()).unwrap_or(3306);
                let database = config.get("database")?.as_str()?;
                let username = config.get("username")?.as_str()?;
                let password = config.get("password").and_then(|v| v.as_str()).unwrap_or("");
                
                // URL encode username and password to handle special characters
                let encoded_username = urlencoding::encode(username);
                let encoded_password = if password.is_empty() {
                    String::new()
                } else {
                    urlencoding::encode(password).to_string()
                };
                
                if encoded_password.is_empty() {
                    Some(format!("mysql://{}@{}:{}/{}", encoded_username, host, port, database))
                } else {
                    Some(format!("mysql://{}:{}@{}:{}/{}", encoded_username, encoded_password, host, port, database))
                }
            }
            "sqlite" => {
                let path = config.get("path")?.as_str()?;
                Some(format!("sqlite://{}", path))
            }
            _ => None
        }
    }
}

// Helper struct for datasource info
struct DataSourceInfo {
    name: String,
    source_type: String,
    connection_config: Value,
}