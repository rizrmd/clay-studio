use crate::core::datasources::shared_service;
use crate::core::mcp::types::*;
use crate::core::projects::manager::ProjectManager;
use crate::utils::claude_md_template;
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use uuid;

// Import tools functionality
use super::tools;

#[derive(Clone)]
pub struct McpHandlers {
    pub project_id: String,
    #[allow(dead_code)]
    pub client_id: String,
    #[allow(dead_code)]
    pub server_type: String,
    pub db_pool: PgPool,
}

impl McpHandlers {
    /// Verify that the client and project exist in the database
    #[allow(dead_code)]
    pub async fn verify_client_and_project_exist(&self) -> Result<(), String> {
        // Optimized: Run client and project existence checks in parallel
        let client_uuid = uuid::Uuid::parse_str(&self.client_id)
            .map_err(|e| format!("Invalid client ID format: {}", e))?;

        let (client_exists, project_exists) = tokio::join!(
            sqlx::query("SELECT 1 FROM clients WHERE id = $1 AND deleted_at IS NULL")
                .bind(client_uuid)
                .fetch_optional(&self.db_pool),
            sqlx::query("SELECT 1 FROM projects WHERE id = $1 AND deleted_at IS NULL")
                .bind(&self.project_id)
                .fetch_optional(&self.db_pool)
        );

        let client_exists = client_exists
            .map_err(|e| format!("Database error checking client existence: {}", e))?;

        if client_exists.is_none() {
            return Err(format!("Client {} does not exist", &self.client_id));
        }

        let project_exists = project_exists
            .map_err(|e| format!("Database error checking project existence: {}", e))?;

        if project_exists.is_none() {
            return Err(format!("Project {} does not exist", &self.project_id));
        }

        Ok(())
    }

    /// Refresh CLAUDE.md with current datasource information
    #[allow(dead_code)]
    pub async fn refresh_claude_md(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Optimized: Fetch datasources and project name in parallel
        let (data_sources, project_name) = tokio::join!(
            sqlx::query(
                "SELECT id, name, source_type, schema_info FROM data_sources WHERE project_id = $1 AND deleted_at IS NULL"
            )
            .bind(&self.project_id)
            .fetch_all(&self.db_pool),
            sqlx::query_scalar::<_, String>("SELECT name FROM projects WHERE id = $1")
                .bind(&self.project_id)
                .fetch_one(&self.db_pool)
        );

        let data_sources = data_sources?;
        let project_name = project_name?;

        if !data_sources.is_empty() {

            // Convert datasources to the format expected by the template
            let datasource_values: Vec<serde_json::Value> = data_sources
                .iter()
                .map(|ds| {
                    json!({
                        "id": ds.get::<String, _>("id"),
                        "name": ds.get::<String, _>("name"),
                        "source_type": ds.get::<String, _>("source_type"),
                        "schema_info": ds.get::<Option<String>, _>("schema_info"),
                    })
                })
                .collect();

            // Generate enhanced CLAUDE.md with datasource information
            let claude_md_content = claude_md_template::generate_claude_md_with_datasources(
                &self.project_id,
                &project_name,
                datasource_values,
            )
            .await;

            // Write to project's CLAUDE.md
            let pm = ProjectManager::new();
            let client_id = uuid::Uuid::parse_str(&self.client_id)
                .map_err(|e| format!("Invalid client ID: {}", e))?;
            pm.save_claude_md_content(client_id, &self.project_id, &claude_md_content)
                .map_err(|e| format!("Failed to save CLAUDE.md: {}", e))?;
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn execute_db_operation<F, T>(&self, operation: &str, f: F) -> Result<T, JsonRpcError>
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

    #[allow(dead_code)]
    pub fn handle_mcp_error(
        &self,
        operation: &str,
        error: Box<dyn std::error::Error + Send + Sync>,
    ) -> JsonRpcError {
        eprintln!(
            "[{}] [ERROR] MCP operation '{}' failed: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            operation,
            error
        );
        JsonRpcError {
            code: INTERNAL_ERROR,
            message: error.to_string(),
            data: None,
        }
    }

    #[allow(dead_code)]
    pub async fn get_datasource_connector(
        &self,
        datasource_id: &str,
    ) -> Result<DataSourceInfo, JsonRpcError> {
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

        let connection_config: Value = source.get("connection_config");

        Ok(DataSourceInfo {
            name: source.get("name"),
            source_type: source.get("source_type"),
            connection_config,
        })
    }

    /// Get all available MCP tool names for Claude CLI allowed tools configuration
    pub fn get_all_available_mcp_tools() -> Vec<String> {
        tools::get_all_available_mcp_tools()
    }

    pub async fn handle_initialize(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        eprintln!(
            "[{}] [INFO] Handling initialize request for project: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            self.project_id
        );
        
        // Extract the protocol version from client request and echo it back
        let client_protocol_version = params
            .as_ref()
            .and_then(|p| p.get("protocolVersion"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "2025-06-18".to_string());
            
        eprintln!(
            "[{}] [INFO] Client requested protocol version: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            client_protocol_version
        );
        
        eprintln!(
            "[{}] [INFO] MCP Server fully initialized and ready for requests",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
        
        // Get available tools to include in capabilities
        let tools_list = self.handle_tools_list(None).await?;
        let available_tools = tools_list.get("tools").and_then(|t| t.as_array()).map(|tools| {
            tools.iter().filter_map(|tool| {
                tool.get("name").and_then(|n| n.as_str()).map(|name| name.to_string())
            }).collect::<Vec<String>>()
        }).unwrap_or_default();
        
        eprintln!(
            "[{}] [DEBUG] Advertising {} tools in capabilities: {:?}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            available_tools.len(),
            available_tools
        );

        let result = InitializeResult {
            protocol_version: client_protocol_version,
            server_info: ServerInfo {
                name: "Clay Studio MCP Server".to_string(),
                version: "1.0.0".to_string(),
            },
            capabilities: Capabilities {
                resources: Some(ResourcesCapability {
                    subscribe: false,  // We don't support subscriptions yet
                    list_changed: false, // Our resource list is static
                }),
                tools: Some(ToolsCapability {
                    list_changed: false, // Our tool list is static
                }),
            },
        };

        serde_json::to_value(result).map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Failed to serialize response: {}", e),
            data: None,
        })
    }

    pub async fn handle_resources_list(&self, _params: Option<Value>) -> Result<Value, JsonRpcError> {
        eprintln!(
            "[{}] [INFO] Handling resources/list request for project: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            self.project_id
        );

        let mut resources = vec![Resource {
            uri: format!("claude://project/{}/claude.md", self.project_id),
            name: "CLAUDE.md".to_string(),
            mime_type: "text/markdown".to_string(),
            description: Some("Project documentation and datasource information".to_string()),
        }];

        // Add datasources as resources  
        let datasources = sqlx::query(
            "SELECT id, name, source_type, schema_info FROM data_sources WHERE project_id = $1 AND deleted_at IS NULL"
        )
        .bind(&self.project_id)
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Database error: {}", e),
            data: None,
        })?;

        for datasource in datasources {
            let id: String = datasource.get("id");
            let name: String = datasource.get("name");
            let source_type: String = datasource.get("source_type");
            
            resources.push(Resource {
                uri: format!("datasource://{}", id),
                name: format!("{} ({})", name, source_type),
                mime_type: "application/json".to_string(),
                description: Some(format!("Datasource: {} - {}", name, source_type)),
            });
        }

        Ok(json!({
            "resources": resources
        }))
    }

    pub async fn handle_resources_read(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Missing parameters".to_string(),
            data: None,
        })?;

        let uri = params
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing uri parameter".to_string(),
                data: None,
            })?;

        eprintln!(
            "[{}] [INFO] Handling resources/read request for URI: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            uri
        );

        // Check if this is a CLAUDE.md request
        if uri == format!("claude://project/{}/claude.md", self.project_id) {
            // Get CLAUDE.md content for this project
            let content = sqlx::query_scalar::<_, Option<String>>(
                "SELECT claude_md FROM projects WHERE id = $1",
            )
            .bind(&self.project_id)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Database error: {}", e),
                data: None,
            })?
            .flatten()
            .unwrap_or_else(|| {
                "# Clay Studio Project\n\nNo CLAUDE.md content available for this project."
                    .to_string()
            });

            Ok(json!({
                "contents": [
                    {
                        "uri": uri,
                        "mimeType": "text/markdown",
                        "text": content
                    }
                ]
            }))
        } else if uri.starts_with("datasource://") {
            // Handle datasource resource request
            let datasource_id = uri.strip_prefix("datasource://").unwrap_or("");
            
            // Get datasource information
            let datasource = sqlx::query(
                "SELECT id, name, source_type, config, schema_info FROM data_sources WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL"
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
                message: format!("Datasource not found: {}", datasource_id),
                data: None,
            })?;

            let name: String = datasource.get("name");
            let source_type: String = datasource.get("source_type");
            let schema_info: Option<String> = datasource.get("schema_info");

            let mut content = json!({
                "id": datasource_id,
                "name": name,
                "source_type": source_type,
                "schema_info": schema_info
            });

            // Add schema information if available
            if let Some(schema) = schema_info {
                if let Ok(schema_json) = serde_json::from_str::<Value>(&schema) {
                    content["schema"] = schema_json;
                }
            }

            Ok(json!({
                "contents": [
                    {
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&content).unwrap_or_else(|_| content.to_string())
                    }
                ]
            }))
        } else {
            Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: format!("Unknown resource URI: {}", uri),
                data: None,
            })
        }
    }

    pub async fn handle_tools_list(&self, _params: Option<Value>) -> Result<Value, JsonRpcError> {
        // Only advertise tools specific to this server type
        let tools_list = match self.server_type.as_str() {
            "analysis" => tools::analysis::get_analysis_tools(),
            "interaction" => tools::interaction::get_interaction_tools(),
            "operation" => tools::operation::get_operation_tool_definitions(),
            _ => {
                // Default to empty list for unknown server types
                vec![]
            }
        };
        
        Ok(json!({
            "tools": tools_list
        }))
    }

    pub async fn handle_tools_call(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Missing parameters".to_string(),
            data: None,
        })?;

        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing tool name".to_string(),
                data: None,
            })?;

        // Strip the MCP prefix if present (e.g., "mcp__operation__datasource_list" -> "datasource_list")
        let clean_tool_name = if tool_name.starts_with("mcp__") {
            // Find the second "__" and take everything after it
            if let Some(pos) = tool_name.find("__").and_then(|first| {
                tool_name[first + 2..].find("__").map(|second| first + 2 + second + 2)
            }) {
                &tool_name[pos..]
            } else {
                tool_name
            }
        } else {
            tool_name
        };

        let arguments = params.get("arguments");

        eprintln!(
            "[{}] [INFO] Handling tools/call request for tool: {} (cleaned: {})",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            tool_name,
            clean_tool_name
        );

        // Route to appropriate tool handler based on tool name
        match clean_tool_name {
            name if tools::analysis::is_analysis_tool(name) => {
                tools::analysis::handle_tool_call(self, name, arguments).await
            }
            name if tools::interaction::is_interaction_tool(name) => {
                tools::interaction::handle_tool_call(self, name, arguments).await
            }
            name if tools::operation::is_operation_tool(name) => {
                tools::operation::handle_tool_call(self, name, arguments).await
            }
            _ => Err(JsonRpcError {
                code: METHOD_NOT_FOUND,
                message: format!("Unknown tool: {}", clean_tool_name),
                data: None,
            })
        }
    }

    // Stub implementations for missing methods
    pub async fn handle_datasource_inspect(
        &self, 
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        let datasource_id = arguments
            .get("datasource_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: datasource_id".to_string(),
                data: None,
            })?;

        // Get datasource details
        let row = sqlx::query(
            "SELECT name, source_type, connection_config, schema_info FROM data_sources 
             WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL"
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

        if let Some(row) = row {
            let name: String = row.get("name");
            let source_type: String = row.get("source_type");
            let existing_schema: Option<Value> = row.get("schema_info");

            // For now, return the existing schema info
            // In the future, we can implement schema refresh logic
            let result = json!({
                "status": "success",
                "datasource_id": datasource_id,
                "name": name,
                "source_type": source_type,
                "schema": existing_schema,
                "message": "Schema inspection returns cached schema. Live refresh not yet implemented."
            });
            Ok(serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()))
        } else {
            Err(JsonRpcError {
                code: -32602,
                message: format!("Datasource {} not found", datasource_id),
                data: None,
            })
        }
    }

    pub async fn handle_show_table(
        &self, 
        arguments: Option<&serde_json::Value>
    ) -> Result<serde_json::Value, JsonRpcError> {
        let args = arguments
            .and_then(|v| v.as_object())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Invalid arguments".to_string(),
                data: None,
            })?;

        // Get required data parameter
        let data = args.get("data")
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: data".to_string(),
                data: None,
            })?;

        // Validate data has required structure
        let columns = data.get("columns")
            .and_then(|v| v.as_array())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing or invalid 'columns' in data".to_string(),
                data: None,
            })?;

        let rows = data.get("rows")
            .and_then(|v| v.as_array())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing or invalid 'rows' in data".to_string(),
                data: None,
            })?;

        // Get optional title
        let title = args.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Table");

        // Create response
        Ok(json!({
            "status": "success",
            "display_type": "table",
            "title": title,
            "data": {
                "columns": columns,
                "rows": rows,
                "row_count": rows.len(),
                "column_count": columns.len()
            },
            "message": "Table data prepared for display"
        }))
    }

    pub async fn handle_show_chart(
        &self, 
        arguments: Option<&serde_json::Value>
    ) -> Result<serde_json::Value, JsonRpcError> {
        let args = arguments
            .and_then(|v| v.as_object())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Invalid arguments".to_string(),
                data: None,
            })?;

        // Get required parameters
        let data = args.get("data")
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: data".to_string(),
                data: None,
            })?;

        let chart_type = args.get("chart_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: chart_type".to_string(),
                data: None,
            })?;

        // Validate chart type
        let valid_types = ["line", "bar", "pie", "donut", "area", "scatter", "bubble", "heatmap", "radar", "polar"];
        if !valid_types.contains(&chart_type) {
            return Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: format!("Invalid chart_type '{}'. Must be one of: {:?}", chart_type, valid_types),
                data: None,
            });
        }

        // Validate data structure based on chart type
        match chart_type {
            "pie" | "donut" => {
                // Pie/donut charts need labels and values
                let labels = data.get("labels").and_then(|v| v.as_array());
                let values = data.get("values").and_then(|v| v.as_array());
                
                if labels.is_none() || values.is_none() {
                    return Err(JsonRpcError {
                        code: INVALID_PARAMS,
                        message: format!("{} chart requires 'labels' and 'values' arrays in data", chart_type),
                        data: None,
                    });
                }
            },
            _ => {
                // Other charts need categories and series
                let categories = data.get("categories").and_then(|v| v.as_array());
                let series = data.get("series").and_then(|v| v.as_array());
                
                if categories.is_none() || series.is_none() {
                    return Err(JsonRpcError {
                        code: INVALID_PARAMS,
                        message: format!("{} chart requires 'categories' and 'series' arrays in data", chart_type),
                        data: None,
                    });
                }
            }
        }

        // Get optional title
        let title = args.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Chart");

        // Create response
        Ok(json!({
            "status": "success",
            "display_type": "chart",
            "chart_type": chart_type,
            "title": title,
            "data": data,
            "message": format!("{} chart data prepared for display", chart_type)
        }))
    }


    pub async fn handle_schema_get(
        &self, 
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        // Use the existing get_schema method from schema.rs
        self.get_schema(arguments).await
    }

    pub async fn handle_schema_search(
        &self, 
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        let datasource_id = arguments.get("datasource_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: datasource_id".to_string(),
                data: None,
            })?;

        let search_term = arguments.get("search_term")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: search_term".to_string(),
                data: None,
            })?;

        // Get cached schema and search through it
        let row = sqlx::query(
            "SELECT schema_info FROM data_sources WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL"
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

        let mut results = Vec::new();
        if let Some(row) = row {
            if let Some(schema_info) = row.get::<Option<Value>, _>("schema_info") {
                let search_lower = search_term.to_lowercase();
                
                // Search through tables
                if let Some(tables) = schema_info.get("tables").and_then(|t| t.as_object()) {
                    for (table_name, table_data) in tables {
                        // Check if table name matches
                        if table_name.to_lowercase().contains(&search_lower) {
                            results.push(json!({
                                "type": "table",
                                "name": table_name,
                                "match_in": "table_name"
                            }));
                        }
                        
                        // Search in columns
                        if let Some(columns) = table_data.as_array() {
                            for column in columns {
                                if let Some(col_name) = column.get("name").and_then(|n| n.as_str()) {
                                    if col_name.to_lowercase().contains(&search_lower) {
                                        results.push(json!({
                                            "type": "column",
                                            "table": table_name,
                                            "column": col_name,
                                            "data_type": column.get("type"),
                                            "match_in": "column_name"
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let result = json!({
            "status": "success",
            "datasource_id": datasource_id,
            "search_term": search_term,
            "results": results,
            "count": results.len()
        });
        
        Ok(serde_json::to_string(&result).unwrap())
    }

    pub async fn handle_schema_related(
        &self, 
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        let datasource_id = arguments.get("datasource_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: datasource_id".to_string(),
                data: None,
            })?;

        let table_name = arguments.get("table_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: table_name".to_string(),
                data: None,
            })?;

        // For now, return a simplified response
        // Full implementation would query foreign keys and relationships
        let result = json!({
            "status": "success",
            "datasource_id": datasource_id,
            "table_name": table_name,
            "relationships": [
                // This would be populated with actual foreign key relationships
                // by querying information_schema or equivalent
            ],
            "message": "Relationship detection requires database-specific implementation"
        });
        
        Ok(serde_json::to_string(&result).unwrap())
    }

    pub async fn handle_schema_stats(
        &self, 
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        let datasource_id = arguments.get("datasource_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: datasource_id".to_string(),
                data: None,
            })?;

        // Get schema info from cache
        let row = sqlx::query(
            "SELECT name, source_type, schema_info FROM data_sources WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL"
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

        if let Some(row) = row {
            let name: String = row.get("name");
            let source_type: String = row.get("source_type");
            let schema_info = row.get::<Option<Value>, _>("schema_info");

            let mut table_count = 0;
            let mut total_columns = 0;
            let mut table_stats = Vec::new();

            if let Some(schema) = schema_info {
                if let Some(tables) = schema.get("tables").and_then(|t| t.as_object()) {
                    table_count = tables.len();
                    
                    for (table_name, table_data) in tables {
                        let column_count = if let Some(columns) = table_data.as_array() {
                            columns.len()
                        } else if let Some(columns) = table_data.get("columns").and_then(|v| v.as_array()) {
                            columns.len()
                        } else {
                            0
                        };
                        
                        total_columns += column_count;
                        
                        table_stats.push(json!({
                            "table_name": table_name,
                            "column_count": column_count
                        }));
                    }
                }
            }

            let result = json!({
                "status": "success",
                "datasource_id": datasource_id,
                "datasource_name": name,
                "source_type": source_type,
                "stats": {
                    "table_count": table_count,
                    "total_columns": total_columns,
                    "average_columns_per_table": if table_count > 0 { total_columns as f64 / table_count as f64 } else { 0.0 },
                    "tables": table_stats
                }
            });
            
            Ok(serde_json::to_string(&result).unwrap())
        } else {
            Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: format!("Datasource {} not found", datasource_id),
                data: None,
            })
        }
    }

    // Datasource handler methods
    pub async fn handle_datasource_add(
        &self, 
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        self.add_datasource(arguments).await
    }

    pub async fn handle_datasource_list(
        &self, 
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        // Check if active_only filter is requested
        let active_only = arguments.get("active_only")
            .and_then(|v| {
                // Handle both boolean and string "true"/"false"
                if let Some(b) = v.as_bool() {
                    Some(b)
                } else if let Some(s) = v.as_str() {
                    match s.to_lowercase().as_str() {
                        "true" => Some(true),
                        "false" => Some(false),
                        _ => None
                    }
                } else {
                    None
                }
            })
            .unwrap_or(false);

        // Query datasources based on filter
        let query = if active_only {
            "SELECT id, name, source_type, is_active, created_at FROM data_sources 
             WHERE project_id = $1 AND deleted_at IS NULL AND is_active = true
             ORDER BY created_at DESC"
        } else {
            "SELECT id, name, source_type, is_active, created_at FROM data_sources 
             WHERE project_id = $1 AND deleted_at IS NULL 
             ORDER BY created_at DESC"
        };

        let datasources = sqlx::query(query)
            .bind(&self.project_id)
            .fetch_all(&self.db_pool)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Database error: {}", e),
                data: None,
            })?;

        let mut datasource_list = Vec::new();
        for row in datasources {
            let id: String = row.get("id");
            let name: String = row.get("name");
            let source_type: String = row.get("source_type");
            let is_active: bool = row.get("is_active");
            let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");

            datasource_list.push(json!({
                "id": id,
                "name": name,
                "source_type": source_type,
                "is_active": is_active,
                "created_at": created_at.to_rfc3339()
            }));
        }

        // Wrap the array in an object to match expected format
        let result = json!({
            "datasources": datasource_list,
            "count": datasource_list.len(),
            "status": "success"
        });

        Ok(serde_json::to_string(&result).unwrap_or_else(|_| r#"{"datasources":[],"count":0,"status":"error"}"#.to_string()))
    }

    pub async fn handle_datasource_remove(
        &self, 
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        self.remove_datasource(arguments).await
    }

    pub async fn handle_datasource_update(
        &self, 
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        let datasource_id = arguments
            .get("datasource_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: datasource_id".to_string(),
                data: None,
            })?;

        // Build update query dynamically based on provided fields
        let mut update_fields = Vec::new();
        let mut param_count = 3; // Starting at $3 (after id and project_id)

        if let Some(_name) = arguments.get("name").and_then(|v| v.as_str()) {
            update_fields.push(format!("name = ${}", param_count));
            param_count += 1;
        }

        if let Some(_source_type) = arguments.get("source_type").and_then(|v| v.as_str()) {
            update_fields.push(format!("source_type = ${}", param_count));
            param_count += 1;
        }

        if let Some(_config) = arguments.get("config") {
            update_fields.push(format!("connection_config = ${}", param_count));
            param_count += 1;
        }

        if arguments.get("is_active").and_then(|v| v.as_bool()).is_some() {
            update_fields.push(format!("is_active = ${}", param_count));
            let _ = param_count + 1; // Last parameter, increment not used
        }

        if update_fields.is_empty() {
            return Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: "No fields to update".to_string(),
                data: None,
            });
        }

        update_fields.push("updated_at = NOW()".to_string());

        let update_query = format!(
            "UPDATE data_sources SET {} WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL RETURNING id",
            update_fields.join(", ")
        );

        // Execute update with dynamic bindings
        let mut query = sqlx::query(&update_query)
            .bind(datasource_id)
            .bind(&self.project_id);

        if let Some(name) = arguments.get("name").and_then(|v| v.as_str()) {
            query = query.bind(name);
        }

        if let Some(source_type) = arguments.get("source_type").and_then(|v| v.as_str()) {
            query = query.bind(source_type);
        }

        if let Some(config) = arguments.get("config") {
            query = query.bind(config);
        }

        if let Some(is_active) = arguments.get("is_active").and_then(|v| v.as_bool()) {
            query = query.bind(is_active);
        }

        let result = query
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Database error: {}", e),
                data: None,
            })?;

        if result.is_some() {
            let response = json!({
                "success": true,
                "message": format!("Datasource {} updated successfully", datasource_id)
            });
            Ok(serde_json::to_string(&response).unwrap_or_else(|_| r#"{"success":false}"#.to_string()))
        } else {
            Err(JsonRpcError {
                code: -32602,
                message: format!("Datasource {} not found or no permission", datasource_id),
                data: None,
            })
        }
    }

    pub async fn handle_connection_test(
        &self, 
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        use crate::core::datasources::shared_service;
        
        // Check if testing by datasource_id or by config
        if let Some(datasource_id) = arguments.get("datasource_id").and_then(|v| v.as_str()) {
            // Test existing datasource connection
            let row = sqlx::query(
                "SELECT source_type, connection_config FROM data_sources 
                 WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL"
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

            if let Some(row) = row {
                let source_type: String = row.get("source_type");
                let mut connection_config: Value = row.get("connection_config");
                
                // Add datasource ID to config (needed by connectors)
                if connection_config.is_object() {
                    let config_obj = connection_config.as_object_mut().unwrap();
                    config_obj.insert("id".to_string(), Value::String(datasource_id.to_string()));
                }

                match shared_service::test_datasource_connection(&source_type, &connection_config).await {
                    Ok(_) => {
                        let result = json!({
                            "success": true,
                            "message": "Connection test successful"
                        });
                        Ok(serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()))
                    },
                    Err(e) => {
                        let result = json!({
                            "success": false,
                            "message": format!("Connection test failed: {}", e)
                        });
                        Ok(serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()))
                    }
                }
            } else {
                Err(JsonRpcError {
                    code: -32602,
                    message: format!("Datasource {} not found", datasource_id),
                    data: None,
                })
            }
        } else {
            // Test new connection with provided config
            let source_type = arguments
                .get("source_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError {
                    code: INVALID_PARAMS,
                    message: "Missing required parameter: source_type".to_string(),
                    data: None,
                })?;
            
            let config = arguments
                .get("config")
                .ok_or_else(|| JsonRpcError {
                    code: INVALID_PARAMS,
                    message: "Missing required parameter: config".to_string(),
                    data: None,
                })?;

            match shared_service::test_datasource_connection(source_type, config).await {
                Ok(_) => {
                    let result = json!({
                        "success": true,
                        "message": "Connection test successful"
                    });
                    Ok(serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()))
                },
                Err(e) => {
                    let result = json!({
                        "success": false,
                        "message": format!("Connection test failed: {}", e)
                    });
                    Ok(serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()))
                }
            }
        }
    }

    pub async fn handle_datasource_detail(
        &self,
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        let datasource_id = arguments
            .get("datasource_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: datasource_id".to_string(),
                data: None,
            })?;

        // Query datasource details including connection_config
        let row = sqlx::query(
            "SELECT id, name, source_type, is_active, created_at, updated_at, schema_info, connection_config
             FROM data_sources
             WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL"
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

        if let Some(row) = row {
            let id: String = row.get("id");
            let name: String = row.get("name");
            let source_type: String = row.get("source_type");
            let is_active: bool = row.get("is_active");
            let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
            let updated_at: Option<chrono::DateTime<chrono::Utc>> = row.get("updated_at");
            let schema_info: Option<Value> = row.get("schema_info");
            let connection_config: Value = row.get("connection_config");

            let detail = json!({
                "status": "success",
                "id": id,
                "name": name,
                "source_type": source_type,
                "is_active": is_active,
                "created_at": created_at.to_rfc3339(),
                "updated_at": updated_at.map(|dt| dt.to_rfc3339()),
                "connection_details": connection_config,
                "schema_info": schema_info
            });

            Ok(serde_json::to_string(&detail).unwrap_or_else(|_| "{}".to_string()))
        } else {
            Err(JsonRpcError {
                code: -32602,
                message: format!("Datasource {} not found", datasource_id),
                data: None,
            })
        }
    }

    pub async fn handle_datasource_query(
        &self, 
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        self.query_datasource(arguments).await
    }
    
    /// Execute a query using connection pooling
    /// This method provides an easy way for MCP handlers to use the global connection pool
    #[allow(dead_code)]
    pub async fn execute_query_with_pooling(
        &self,
        datasource_id: &str,
        query: &str,
    ) -> Result<serde_json::Value, JsonRpcError> {
        // Execute query using shared service with connection pooling
        shared_service::execute_query_on_datasource(
            datasource_id,
            &self.project_id,
            query,
            &self.db_pool
        ).await
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Failed to execute query: {}", e),
            data: None,
        })
    }

    // File access handlers
    pub async fn handle_file_list(
        &self, 
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        use crate::models::file_upload::FileUpload;
        
        let client_uuid = uuid::Uuid::parse_str(&self.client_id)
            .map_err(|e| JsonRpcError {
                code: INVALID_PARAMS,
                message: format!("Invalid client ID: {}", e),
                data: None,
            })?;

        let conversation_id = arguments.get("conversation_id").and_then(|v| v.as_str());

        let files = if let Some(conv_id) = conversation_id {
            sqlx::query_as::<_, FileUpload>(
                "SELECT * FROM file_uploads 
                 WHERE client_id = $1 AND project_id = $2 AND conversation_id = $3 
                 ORDER BY created_at DESC"
            )
            .bind(client_uuid)
            .bind(&self.project_id)
            .bind(conv_id)
            .fetch_all(&self.db_pool)
            .await
        } else {
            sqlx::query_as::<_, FileUpload>(
                "SELECT * FROM file_uploads 
                 WHERE client_id = $1 AND project_id = $2 
                 ORDER BY created_at DESC"
            )
            .bind(client_uuid)
            .bind(&self.project_id)
            .fetch_all(&self.db_pool)
            .await
        }
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Database error: {}", e),
            data: None,
        })?;

        let file_responses: Vec<_> = files.iter().map(|f| f.to_response()).collect();
        
        let response = json!({
            "status": "success",
            "message": format!("Found {} uploaded files", files.len()),
            "files": file_responses
        });

        serde_json::to_string(&response)
            .map_err(|e| JsonRpcError {
                code: -32603,
                message: format!("Failed to serialize response: {}", e),
                data: None,
            })
    }

    pub async fn handle_file_read(
        &self, 
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        // Use the safer implementation from file_safety module
        self.handle_file_read_safe(arguments).await
    }
    
    /// Legacy implementation - kept for reference
    #[allow(dead_code)]
    async fn handle_file_read_legacy(
        &self, 
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        use crate::models::file_upload::FileUpload;
        
        let file_id = arguments.get("file_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: file_id".to_string(),
                data: None,
            })?;

        let file_uuid = uuid::Uuid::parse_str(file_id)
            .map_err(|e| JsonRpcError {
                code: INVALID_PARAMS,
                message: format!("Invalid file ID: {}", e),
                data: None,
            })?;

        let client_uuid = uuid::Uuid::parse_str(&self.client_id)
            .map_err(|e| JsonRpcError {
                code: INVALID_PARAMS,
                message: format!("Invalid client ID: {}", e),
                data: None,
            })?;

        let file = sqlx::query_as::<_, FileUpload>(
            "SELECT * FROM file_uploads 
             WHERE id = $1 AND client_id = $2 AND project_id = $3"
        )
        .bind(file_uuid)
        .bind(client_uuid)
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
            message: "File not found".to_string(),
            data: None,
        })?;

        let response = if let Some(content) = file.file_content {
            json!({
                "status": "success",
                "message": "File content retrieved successfully",
                "file": {
                    "id": file.id.to_string(),
                    "name": file.file_name,
                    "original_name": file.original_name,
                    "mime_type": file.mime_type,
                    "size": file.file_size,
                    "content": content,
                    "description": file.description,
                    "auto_description": file.auto_description,
                    "created_at": file.created_at
                }
            })
        } else {
            // For binary files, read from filesystem
            use std::fs;
            match fs::read_to_string(&file.file_path) {
                Ok(content) => json!({
                    "status": "success",
                    "message": "File content retrieved from filesystem",
                    "file": {
                        "id": file.id.to_string(),
                        "name": file.file_name,
                        "original_name": file.original_name,
                        "mime_type": file.mime_type,
                        "size": file.file_size,
                        "content": content,
                        "description": file.description,
                        "auto_description": file.auto_description,
                        "created_at": file.created_at
                    }
                }),
                Err(_) => json!({
                    "status": "error",
                    "message": "File is binary or cannot be read as text",
                    "file": {
                        "id": file.id.to_string(),
                        "name": file.file_name,
                        "original_name": file.original_name,
                        "mime_type": file.mime_type,
                        "size": file.file_size,
                        "content": null,
                        "description": file.description,
                        "auto_description": file.auto_description,
                        "created_at": file.created_at,
                        "file_path": file.file_path
                    }
                })
            }
        };

        serde_json::to_string(&response)
            .map_err(|e| JsonRpcError {
                code: -32603,
                message: format!("Failed to serialize response: {}", e),
                data: None,
            })
    }

    pub async fn handle_file_search(
        &self, 
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        use crate::models::file_upload::FileUpload;
        
        let query = arguments.get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: query".to_string(),
                data: None,
            })?;

        let file_type = arguments.get("file_type").and_then(|v| v.as_str());
        let conversation_id = arguments.get("conversation_id").and_then(|v| v.as_str());

        let client_uuid = uuid::Uuid::parse_str(&self.client_id)
            .map_err(|e| JsonRpcError {
                code: INVALID_PARAMS,
                message: format!("Invalid client ID: {}", e),
                data: None,
            })?;

        let mut sql = "SELECT * FROM file_uploads 
                       WHERE client_id = $1 AND project_id = $2".to_string();
        let mut params: Vec<String> = vec![
            client_uuid.to_string(),
            self.project_id.clone(),
        ];

        if let Some(conv_id) = conversation_id {
            sql.push_str(" AND conversation_id = $3");
            params.push(conv_id.to_string());
        }

        // Add search conditions - ONLY search metadata, not content!
        // Content searching should use file_search_content on individual files
        let search_clause = if conversation_id.is_some() {
            " AND (file_name ILIKE $4 OR original_name ILIKE $4 OR description ILIKE $4 OR auto_description ILIKE $4)"
        } else {
            " AND (file_name ILIKE $3 OR original_name ILIKE $3 OR description ILIKE $3 OR auto_description ILIKE $3)"
        };
        sql.push_str(search_clause);
        params.push(format!("%{}%", query));

        if let Some(ftype) = file_type {
            let type_clause = if conversation_id.is_some() { " AND mime_type ILIKE $5" } else { " AND mime_type ILIKE $4" };
            sql.push_str(type_clause);
            params.push(format!("%{}%", ftype));
        }

        sql.push_str(" ORDER BY created_at DESC");

        // Use a simpler approach for the search query - NO content search for performance
        let files = sqlx::query_as::<_, FileUpload>(
            "SELECT * FROM file_uploads 
             WHERE client_id = $1 AND project_id = $2 
             AND (file_name ILIKE $3 OR original_name ILIKE $3 OR description ILIKE $3 OR auto_description ILIKE $3)
             ORDER BY created_at DESC"
        )
        .bind(client_uuid)
        .bind(&self.project_id)
        .bind(format!("%{}%", query))
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Database error: {}", e),
            data: None,
        })?;

        let file_responses: Vec<_> = files.iter().map(|f| f.to_response()).collect();
        
        let response = json!({
            "status": "success",
            "message": format!("Found {} files matching '{}' (searched metadata only)", files.len(), query),
            "files": file_responses,
            "search_query": query,
            "note": "This search only looks at file names and descriptions, not file contents. To search within a specific file's content, use 'file_search_content' with the file ID."
        });

        serde_json::to_string(&response)
            .map_err(|e| JsonRpcError {
                code: -32603,
                message: format!("Failed to serialize response: {}", e),
                data: None,
            })
    }

    pub async fn handle_file_metadata(
        &self, 
        arguments: &serde_json::Map<String, serde_json::Value>
    ) -> Result<String, JsonRpcError> {
        use crate::models::file_upload::FileUpload;
        
        let file_id = arguments.get("file_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: file_id".to_string(),
                data: None,
            })?;

        let file_uuid = uuid::Uuid::parse_str(file_id)
            .map_err(|e| JsonRpcError {
                code: INVALID_PARAMS,
                message: format!("Invalid file ID: {}", e),
                data: None,
            })?;

        let client_uuid = uuid::Uuid::parse_str(&self.client_id)
            .map_err(|e| JsonRpcError {
                code: INVALID_PARAMS,
                message: format!("Invalid client ID: {}", e),
                data: None,
            })?;

        let file = sqlx::query_as::<_, FileUpload>(
            "SELECT * FROM file_uploads 
             WHERE id = $1 AND client_id = $2 AND project_id = $3"
        )
        .bind(file_uuid)
        .bind(client_uuid)
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
            message: "File not found".to_string(),
            data: None,
        })?;

        let response = json!({
            "status": "success",
            "message": "File metadata retrieved successfully",
            "file": {
                "id": file.id.to_string(),
                "name": file.file_name,
                "original_name": file.original_name,
                "file_path": file.file_path,
                "size": file.file_size,
                "mime_type": file.mime_type,
                "description": file.description,
                "auto_description": file.auto_description,
                "conversation_id": file.conversation_id,
                "has_content": file.file_content.is_some(),
                "is_text_file": file.file_content.is_some(),
                "created_at": file.created_at,
                "updated_at": file.updated_at,
                "metadata": file.metadata
            }
        });

        serde_json::to_string(&response)
            .map_err(|e| JsonRpcError {
                code: -32603,
                message: format!("Failed to serialize response: {}", e),
                data: None,
            })
    }
}
