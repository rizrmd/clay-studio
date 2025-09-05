use super::base::McpHandlers;
use crate::core::mcp::types::*;
use chrono::Utc;
use serde_json::{json, Value};
use uuid;

impl McpHandlers {
    /// Get all available MCP tool names for Claude CLI allowed tools configuration
    pub fn get_all_available_mcp_tools() -> Vec<String> {
        let mut tools = Vec::new();

        // Data analysis tools (from handle_tools_list - lines 121-349)
        let data_analysis_tools = vec![
            "datasource_add",
            "datasource_list", 
            "datasource_remove",
            "datasource_update",
            "connection_test",
            "datasource_detail",
            "datasource_query",
            "datasource_inspect",
            "schema_get",
            "schema_search",
            "schema_related",
            "schema_stats",
        ];

        // Add data analysis tools with mcp prefix
        for tool in data_analysis_tools {
            tools.push(format!("mcp__data-analysis__{}", tool));
        }

        // Interaction tools (from handle_tools_list - lines 352-487)
        let interaction_tools = vec![
            "ask_user",
            "export_excel", 
            "show_table",
            "show_chart",
        ];

        // Add interaction tools with mcp prefix
        for tool in interaction_tools {
            tools.push(format!("mcp__interaction__{}", tool));
        }

        // Add standard web tools
        tools.extend(vec![
            "WebSearch".to_string(),
            "WebFetch".to_string(),
        ]);

        tools
    }

    pub async fn handle_initialize(&self, _params: Option<Value>) -> Result<Value, JsonRpcError> {
        eprintln!(
            "[{}] [INFO] Handling initialize request for project: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            self.project_id
        );
        let result = InitializeResult {
            protocol_version: "2025-06-18".to_string(),
            server_info: ServerInfo {
                name: "Clay Studio MCP Server".to_string(),
                version: "1.0.0".to_string(),
            },
            capabilities: Capabilities {
                resources: Some(ResourcesCapability {}),
                tools: Some(ToolsCapability {}),
            },
        };

        Ok(serde_json::to_value(result).map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Failed to serialize response: {}", e),
            data: None,
        })?)
    }

    pub async fn handle_resources_list(
        &self,
        _params: Option<Value>,
    ) -> Result<Value, JsonRpcError> {
        eprintln!(
            "[{}] [INFO] Handling resources/list request for project: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            self.project_id
        );

        let resources = vec![Resource {
            uri: format!("claude://project/{}/claude.md", self.project_id),
            name: "CLAUDE.md".to_string(),
            mime_type: "text/markdown".to_string(),
            description: Some("Project documentation and datasource information".to_string()),
        }];

        Ok(json!({
            "resources": resources
        }))
    }

    pub async fn handle_resources_read(
        &self,
        params: Option<Value>,
    ) -> Result<Value, JsonRpcError> {
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
        } else {
            Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: format!("Unknown resource URI: {}", uri),
                data: None,
            })
        }
    }

    pub async fn handle_tools_list(&self, _params: Option<Value>) -> Result<Value, JsonRpcError> {
        eprintln!(
            "[{}] [INFO] Handling tools/list request for project: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            self.project_id
        );

        let mut tools = vec![
            Tool {
                name: "datasource_add".to_string(),
                description: "Add a new datasource to the project with connection details"
                    .to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Human-readable name for the datasource"
                        },
                        "source_type": {
                            "type": "string",
                            "enum": ["postgresql", "mysql", "clickhouse", "sqlite", "oracle", "sqlserver"],
                            "description": "Type of database system"
                        },
                        "config": {
                            "oneOf": [
                                {
                                    "type": "string",
                                    "description": "Connection URL (e.g., postgresql://user:pass@host:port/db)"
                                },
                                {
                                    "type": "object",
                                    "properties": {
                                        "host": {"type": "string"},
                                        "port": {"type": "integer"},
                                        "database": {"type": "string"},
                                        "user": {"type": "string"},
                                        "password": {"type": "string"}
                                    },
                                    "required": ["host", "database"],
                                    "description": "Connection configuration object"
                                }
                            ],
                            "description": "Database connection configuration"
                        }
                    },
                    "required": ["name", "source_type", "config"]
                }),
            },
            Tool {
                name: "datasource_list".to_string(),
                description: "List all datasources in the project".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
            },
            Tool {
                name: "datasource_remove".to_string(),
                description: "Remove a datasource from the project".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the datasource to remove"
                        }
                    },
                    "required": ["datasource_id"]
                }),
            },
            Tool {
                name: "datasource_update".to_string(),
                description: "Update an existing datasource configuration".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the datasource to update"
                        },
                        "name": {
                            "type": "string",
                            "description": "New name for the datasource"
                        },
                        "config": {
                            "oneOf": [
                                {
                                    "type": "string",
                                    "description": "New connection URL"
                                },
                                {
                                    "type": "object",
                                    "description": "New connection configuration object"
                                }
                            ]
                        }
                    },
                    "required": ["datasource_id"]
                }),
            },
            Tool {
                name: "connection_test".to_string(),
                description: "Test the connection to a specific datasource".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the datasource to test"
                        }
                    },
                    "required": ["datasource_id"]
                }),
            },
            Tool {
                name: "datasource_detail".to_string(),
                description: "Get detailed information about a specific datasource".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the datasource"
                        }
                    },
                    "required": ["datasource_id"]
                }),
            },
            Tool {
                name: "datasource_query".to_string(),
                description: "Execute a SQL query against a datasource".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the datasource to query"
                        },
                        "query": {
                            "type": "string",
                            "description": "SQL query to execute"
                        },
                        "limit": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 1000,
                            "default": 100,
                            "description": "Maximum number of rows to return"
                        }
                    },
                    "required": ["datasource_id", "query"]
                }),
            },
            Tool {
                name: "datasource_inspect".to_string(),
                description: "Analyze a datasource to understand its schema, tables, and structure"
                    .to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the datasource to inspect"
                        }
                    },
                    "required": ["datasource_id"]
                }),
            },
            Tool {
                name: "schema_get".to_string(),
                description: "Get the complete schema information for a datasource".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the datasource"
                        }
                    },
                    "required": ["datasource_id"]
                }),
            },
            Tool {
                name: "schema_search".to_string(),
                description: "Search for tables, columns, or other schema elements".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the datasource to search"
                        },
                        "search_term": {
                            "type": "string",
                            "description": "Term to search for in table names, column names, etc."
                        }
                    },
                    "required": ["datasource_id", "search_term"]
                }),
            },
            Tool {
                name: "schema_related".to_string(),
                description: "Get tables related to a specific table (foreign keys, references)"
                    .to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the datasource"
                        },
                        "table_name": {
                            "type": "string",
                            "description": "Name of the table to find relationships for"
                        }
                    },
                    "required": ["datasource_id", "table_name"]
                }),
            },
            Tool {
                name: "schema_stats".to_string(),
                description: "Get statistical information about the datasource schema".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "datasource_id": {
                            "type": "string",
                            "description": "ID of the datasource"
                        }
                    },
                    "required": ["datasource_id"]
                }),
            },
        ];

        // Add interaction-specific tools for interaction server
        if self.server_type == "interaction" {
            tools.extend(vec![
                Tool {
                    name: "ask_user".to_string(),
                    description: "Ask the user a question and wait for their response".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "question": {
                                "type": "string",
                                "description": "The question to ask the user"
                            },
                            "options": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Optional list of predefined response options"
                            }
                        },
                        "required": ["question"]
                    }),
                },
                Tool {
                    name: "export_excel".to_string(),
                    description:
                        "Export data to an Excel file with multiple sheets and formatting options"
                            .to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "filename": {
                                "type": "string",
                                "description": "Name for the Excel file (without extension)"
                            },
                            "sheets": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "name": {
                                            "type": "string",
                                            "description": "Name of the worksheet"
                                        },
                                        "data": {
                                            "type": "array",
                                            "items": {"type": "array"},
                                            "description": "2D array of data rows"
                                        },
                                        "headers": {
                                            "type": "array",
                                            "items": {"type": "string"},
                                            "description": "Optional column headers"
                                        }
                                    },
                                    "required": ["name", "data"]
                                },
                                "description": "Array of worksheets to include in the Excel file"
                            },
                            "options": {
                                "type": "object",
                                "properties": {
                                    "auto_filter": {
                                        "type": "boolean",
                                        "default": true,
                                        "description": "Enable auto filter on headers"
                                    },
                                    "freeze_panes": {
                                        "type": "object",
                                        "properties": {
                                            "row": {"type": "integer", "minimum": 0},
                                            "col": {"type": "integer", "minimum": 0}
                                        },
                                        "description": "Freeze panes at specified row/column"
                                    },
                                    "column_widths": {
                                        "type": "object",
                                        "patternProperties": {
                                            "^\\d+$": {"type": "number", "minimum": 1}
                                        },
                                        "description": "Custom column widths by column index"
                                    }
                                },
                                "description": "Formatting and display options"
                            }
                        },
                        "required": ["filename", "sheets"]
                    }),
                },
                Tool {
                    name: "show_table".to_string(),
                    description: "Display tabular data in an interactive table format".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "data": {
                                "type": "array",
                                "items": {"type": "array"},
                                "description": "2D array of table data"
                            },
                            "headers": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Column headers"
                            },
                            "title": {
                                "type": "string",
                                "description": "Optional title for the table"
                            }
                        },
                        "required": ["data"]
                    }),
                },
                Tool {
                    name: "show_chart".to_string(),
                    description: "Display data in various chart formats".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "data": {
                                "type": "array",
                                "description": "Chart data"
                            },
                            "chart_type": {
                                "type": "string",
                                "enum": ["line", "bar", "pie", "scatter"],
                                "description": "Type of chart to display"
                            },
                            "title": {
                                "type": "string",
                                "description": "Chart title"
                            }
                        },
                        "required": ["data", "chart_type"]
                    }),
                },
            ]);
        }

        Ok(json!({
            "tools": tools
        }))
    }

    pub async fn handle_tools_call(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Missing parameters".to_string(),
            data: None,
        })?;

        let tool_name =
            params
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError {
                    code: INVALID_PARAMS,
                    message: "Missing tool name".to_string(),
                    data: None,
                })?;

        let default_args = serde_json::Map::new();
        let arguments = params
            .get("arguments")
            .and_then(|v| v.as_object())
            .unwrap_or(&default_args);

        eprintln!(
            "[{}] [INFO] Handling tools/call request for tool: {} in project: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            tool_name,
            self.project_id
        );

        let result = match tool_name {
            // Datasource Tools
            "datasource_add" => self.add_datasource(arguments).await,
            "datasource_list" => self.list_datasources(arguments).await,
            "datasource_remove" => self.remove_datasource(arguments).await,
            "datasource_update" => self.datasource_update(arguments).await,
            "connection_test" => self.test_connection(arguments).await,
            "datasource_detail" => self.get_datasource_detail(arguments).await,
            "datasource_query" => self.query_datasource(arguments).await,
            "datasource_inspect" => self.inspect_datasource(arguments).await,

            // Schema Tools
            "schema_get" => self.get_schema(arguments).await,
            "schema_search" => self.search_schema(arguments).await,
            "schema_related" => self.get_related_schema(arguments).await,
            "schema_stats" => self.get_schema_stats(arguments).await,

            // Interaction Tools (only available on interaction server)
            "ask_user" => {
                if self.server_type == "interaction" {
                    self.handle_ask_user(arguments).await
                } else {
                    Err(JsonRpcError {
                        code: -32601,
                        message: "ask_user tool is only available on interaction server"
                            .to_string(),
                        data: None,
                    })
                }
            }

            // Export Tools
            "export_excel" => {
                if self.server_type == "interaction" {
                    self.handle_export_excel(arguments).await
                } else {
                    Err(JsonRpcError {
                        code: -32601,
                        message: "export_excel tool is only available on interaction server"
                            .to_string(),
                        data: None,
                    })
                }
            }

            // Show Table Tool
            "show_table" => {
                if self.server_type == "interaction" {
                    self.handle_show_table(arguments).await
                } else {
                    Err(JsonRpcError {
                        code: -32601,
                        message: "show_table tool is only available on interaction server"
                            .to_string(),
                        data: None,
                    })
                }
            }
            // Show Chart Tool
            "show_chart" => {
                if self.server_type == "interaction" {
                    self.handle_show_chart(arguments).await
                } else {
                    Err(JsonRpcError {
                        code: -32601,
                        message: "show_chart tool is only available on interaction server"
                            .to_string(),
                        data: None,
                    })
                }
            }

            _ => Err(JsonRpcError {
                code: METHOD_NOT_FOUND,
                message: format!("Unknown tool: {}", tool_name),
                data: None,
            }),
        }?;

        // Check if result is JSON, if so use resource type with application/json
        if serde_json::from_str::<Value>(&result).is_ok() {
            Ok(json!({
                "content": [
                    {
                        "type": "resource",
                        "resource": {
                            "uri": format!("mcp://tool-result/{}", tool_name),
                            "title": format!("{} Result", tool_name),
                            "mimeType": "application/json",
                            "text": result,
                            "annotations": {
                                "audience": ["user", "assistant"],
                                "priority": 0.8
                            }
                        }
                    }
                ]
            }))
        } else {
            // Fallback to text for non-JSON responses
            Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": result
                    }
                ]
            }))
        }
    }

    pub async fn handle_show_table(
        &self,
        arguments: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        let data = arguments
            .get("data")
            .and_then(|v| v.as_array())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing data parameter".to_string(),
                data: None,
            })?;

        let headers = arguments
            .get("headers")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>());

        let title = arguments
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Data Table");

        // Validate data structure
        if data.is_empty() {
            return Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: "Data array cannot be empty".to_string(),
                data: None,
            });
        }

        // Create interaction response
        let interaction_id = uuid::Uuid::new_v4().to_string();
        let interaction_spec = json!({
            "interaction_id": interaction_id,
            "interaction_type": "show_table",
            "title": title,
            "data": data,
            "headers": headers,
            "status": "completed",
            "requires_response": false,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "features": {
                "sortable": true,
                "filterable": true,
                "exportable": true,
                "searchable": true
            }
        });


        serde_json::to_string(&interaction_spec).map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Failed to serialize response: {}", e),
            data: None,
        })
    }

    pub async fn handle_show_chart(
        &self,
        arguments: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        let data = arguments
            .get("data")
            .and_then(|v| v.as_array())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing data parameter".to_string(),
                data: None,
            })?;

        let chart_type = arguments
            .get("chart_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing chart_type parameter".to_string(),
                data: None,
            })?;

        let title = arguments
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Chart");

        // Validate chart type
        let valid_chart_types = ["line", "bar", "pie", "scatter", "area", "radar", "gauge", "map", "sankey", "treemap"];
        if !valid_chart_types.contains(&chart_type) {
            return Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: format!("Invalid chart_type. Supported types: {}", valid_chart_types.join(", ")),
                data: None,
            });
        }

        // Validate data structure
        if data.is_empty() {
            return Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: "Data array cannot be empty".to_string(),
                data: None,
            });
        }

        // Create interaction response
        let interaction_id = uuid::Uuid::new_v4().to_string();
        let interaction_spec = json!({
            "interaction_id": interaction_id,
            "interaction_type": "show_chart",
            "title": title,
            "chart_type": chart_type,
            "data": data,
            "status": "completed",
            "requires_response": false,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "options": arguments.get("options").unwrap_or(&json!({})),
            "features": {
                "interactive": true,
                "zoomable": true,
                "exportable": true,
                "responsive": true
            }
        });


        serde_json::to_string(&interaction_spec).map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Failed to serialize response: {}", e),
            data: None,
        })
    }
}
