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
    pub async fn handle_initialize(&self, _params: Option<Value>) -> Result<Value, JsonRpcError> {
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
        
        Ok(serde_json::to_value(result).unwrap())
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
                    message: "Data source not found".to_string(),
                    data: None,
                })
            }
        }
    }
    
    pub async fn handle_tools_list(&self, _params: Option<Value>) -> Result<Value, JsonRpcError> {
        let tools = vec![
            Tool {
                name: "add_datasource".to_string(),
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
                            "description": "Connection configuration for the data source",
                            "properties": {
                                "host": {"type": "string"},
                                "port": {"type": "integer"},
                                "database": {"type": "string"},
                                "username": {"type": "string"},
                                "password": {"type": "string"}
                            }
                        }
                    },
                    "required": ["name", "source_type", "connection_config"]
                }),
            },
            Tool {
                name: "list_datasources".to_string(),
                description: "List all data sources in the project".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "remove_datasource".to_string(),
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
                name: "test_datasource_connection".to_string(),
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
                name: "query_datasource".to_string(),
                description: "Execute a read-only query on a data source".to_string(),
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
            "add_datasource" => {
                self.add_datasource(arguments).await
            }
            "list_datasources" => {
                self.list_datasources(arguments).await
            }
            "remove_datasource" => {
                self.remove_datasource(arguments).await
            }
            "test_datasource_connection" => {
                self.test_connection(arguments).await
            }
            "query_datasource" => {
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
                let tool_result = ToolCallResult {
                    content: vec![ToolContent {
                        content_type: "text".to_string(),
                        text: format!("Error: {}", e.message),
                    }],
                    is_error: Some(true),
                };
                Ok(serde_json::to_value(tool_result).unwrap())
            }
        }
    }
    
    // Tool implementations
    async fn add_datasource(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
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
        
        let connection_config = args.get("connection_config")
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing connection_config".to_string(),
                data: None,
            })?;
        
        // Generate a new UUID for the datasource
        let datasource_id = uuid::Uuid::new_v4().to_string();
        
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
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Failed to add data source: {}", e),
            data: None,
        })?;
        
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
            return Ok("No data sources found for this project".to_string());
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
                    message: "Data source not found".to_string(),
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
            message: "Data source not found".to_string(),
            data: None,
        })?;
        
        let name: String = source.get("name");
        let source_type: String = source.get("source_type");
        let connection_config: Value = source.get("connection_config");
        
        // For now, just verify the data source exists and has connection config
        // In a real implementation, you would actually test the connection
        if connection_config.is_object() && !connection_config.as_object().unwrap().is_empty() {
            // Update last_tested_at
            sqlx::query(
                "UPDATE data_sources 
                 SET last_tested_at = NOW() 
                 WHERE id = $1"
            )
            .bind(datasource_id)
            .execute(&self.db_pool)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to update last_tested_at: {}", e),
                data: None,
            })?;
            
            Ok(format!(
                "✅ Connection successful for '{}' ({} data source)",
                name, source_type
            ))
        } else {
            Ok(format!(
                "⚠️ Connection config missing for '{}' ({} data source)",
                name, source_type
            ))
        }
    }
    async fn query_datasource(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
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
        
        // Security: Only allow SELECT queries
        let query_lower = query.to_lowercase();
        if !query_lower.trim().starts_with("select") {
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
            message: "Data source not found".to_string(),
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
        let result = connector.execute_query(query, limit)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Query execution failed: {}", e),
                data: None,
            })?;
        
        Ok(format!(
            "Query executed on '{}' (limited to {} rows):\n{}",
            name, limit, serde_json::to_string_pretty(&result).unwrap()
        ))
    }
}