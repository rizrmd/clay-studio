use crate::core::mcp::types::*;
use crate::core::mcp::handlers::base::McpHandlers;
use serde_json::{json, Value};

// JSON-RPC error codes
const INVALID_PARAMS: i32 = -32602;
const INTERNAL_ERROR: i32 = -32603;
const METHOD_NOT_FOUND: i32 = -32601;

/// Get operation tools for the MCP tools list
pub fn get_operation_tools() -> Vec<Tool> {
    vec![
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
                                "description": "Connection URL (e.g., postgres://user:pass@host:port/db or postgresql://user:pass@host:port/db)"
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
                "properties": {
                    "active_only": {
                        "type": "boolean",
                        "description": "Only return active datasources",
                        "default": false
                    }
                },
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
            description: "Get schema information for a datasource. Can return complete schema or specific table schema.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "datasource_id": {
                        "type": "string",
                        "description": "ID of the datasource"
                    },
                    "table_name": {
                        "type": "string",
                        "description": "Optional: specific table name to get schema for. If not provided, returns complete database schema."
                    },
                    "use_cache": {
                        "type": "boolean",
                        "default": true,
                        "description": "Optional: use cached schema if available (default: true)"
                    },
                    "summary_only": {
                        "type": "boolean", 
                        "default": false,
                        "description": "Optional: return only table names and column counts instead of full schema (default: false)"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "description": "Optional: maximum number of tables to return when getting complete schema"
                    },
                    "offset": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Optional: number of tables to skip when getting complete schema"
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
        // Context management tools
        Tool {
            name: "context_read".to_string(),
            description: "Read the current context for the project".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "context_update".to_string(),
            description: "Update the project's context".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "context": {
                        "type": "string",
                        "description": "The new context content (markdown format)"
                    }
                },
                "required": ["context"]
            }),
        },
        Tool {
            name: "context_compile".to_string(),
            description: "Compile the project's context and get the result".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
    ]
}

/// Check if a tool name is an operation tool
pub fn is_operation_tool(tool_name: &str) -> bool {
    matches!(tool_name,
        // Datasource tools
        "datasource_add" | "datasource_list" | "datasource_remove" | "datasource_update" |
        "connection_test" | "datasource_detail" | "datasource_query" | "datasource_inspect" |
        // Schema tools
        "schema_get" | "schema_search" | "schema_related" | "schema_stats" |
        // Context tools
        "context_read" | "context_update" | "context_compile"
    )
}

/// Handle operation tool calls
pub async fn handle_tool_call(
    handlers: &McpHandlers,
    tool_name: &str,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    match tool_name {
        "datasource_add" => {
            use crate::core::mcp::handlers::base::McpHandlers as DataSourceHandler;
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            let result = DataSourceHandler::handle_datasource_add(handlers, args).await?;
            serde_json::from_str(&result).map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Invalid JSON response: {}", e),
                data: None,
            })
        },
        "datasource_list" => {
            use crate::core::mcp::handlers::base::McpHandlers as DataSourceHandler;
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            let result = DataSourceHandler::handle_datasource_list(handlers, args).await?;
            serde_json::from_str(&result).map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Invalid JSON response: {}", e),
                data: None,
            })
        },
        "datasource_remove" => {
            use crate::core::mcp::handlers::base::McpHandlers as DataSourceHandler;
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            let result = DataSourceHandler::handle_datasource_remove(handlers, args).await?;
            serde_json::from_str(&result).map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Invalid JSON response: {}", e),
                data: None,
            })
        },
        "datasource_update" => {
            use crate::core::mcp::handlers::base::McpHandlers as DataSourceHandler;
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            let result = DataSourceHandler::handle_datasource_update(handlers, args).await?;
            serde_json::from_str(&result).map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Invalid JSON response: {}", e),
                data: None,
            })
        },
        "connection_test" => {
            use crate::core::mcp::handlers::base::McpHandlers as DataSourceHandler;
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            let result = DataSourceHandler::handle_connection_test(handlers, args).await?;
            serde_json::from_str(&result).map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Invalid JSON response: {}", e),
                data: None,
            })
        },
        "datasource_detail" => {
            use crate::core::mcp::handlers::base::McpHandlers as DataSourceHandler;
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            let result = DataSourceHandler::handle_datasource_detail(handlers, args).await?;
            serde_json::from_str(&result).map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Invalid JSON response: {}", e),
                data: None,
            })
        },
        "datasource_query" => {
            use crate::core::mcp::handlers::base::McpHandlers as DataSourceHandler;
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            let result = DataSourceHandler::handle_datasource_query(handlers, args).await?;
            serde_json::from_str(&result).map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Invalid JSON response: {}", e),
                data: None,
            })
        },
        "datasource_inspect" => {
            use crate::core::mcp::handlers::base::McpHandlers as DataSourceHandler;
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            let result = DataSourceHandler::handle_datasource_inspect(handlers, args).await?;
            serde_json::from_str(&result).map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Invalid JSON response: {}", e),
                data: None,
            })
        },
        "schema_get" => {
            use crate::core::mcp::handlers::base::McpHandlers as SchemaHandler;
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            let result = SchemaHandler::handle_schema_get(handlers, args).await?;
            serde_json::from_str(&result).map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Invalid JSON response: {}", e),
                data: None,
            })
        },
        "schema_search" => {
            use crate::core::mcp::handlers::base::McpHandlers as SchemaHandler;
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            let result = SchemaHandler::handle_schema_search(handlers, args).await?;
            serde_json::from_str(&result).map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Invalid JSON response: {}", e),
                data: None,
            })
        },
        "schema_related" => {
            use crate::core::mcp::handlers::base::McpHandlers as SchemaHandler;
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            let result = SchemaHandler::handle_schema_related(handlers, args).await?;
            serde_json::from_str(&result).map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Invalid JSON response: {}", e),
                data: None,
            })
        },
        "schema_stats" => {
            use crate::core::mcp::handlers::base::McpHandlers as SchemaHandler;
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            let result = SchemaHandler::handle_schema_stats(handlers, args).await?;
            serde_json::from_str(&result).map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Invalid JSON response: {}", e),
                data: None,
            })
        },
        // Context tools
        "context_read" => {
            handle_context_tool(handlers, tool_name, arguments).await
        },
        "context_update" => {
            handle_context_tool(handlers, tool_name, arguments).await
        },
        "context_compile" => {
            handle_context_tool(handlers, tool_name, arguments).await
        },
        _ => Err(JsonRpcError {
            code: METHOD_NOT_FOUND,
            message: format!("Unknown operation tool: {}", tool_name),
            data: None,
        })
    }
}

/// Handle context-related tools
async fn handle_context_tool(
    handlers: &McpHandlers,
    tool_name: &str,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let empty_map = serde_json::Map::new();
    let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
    let project_id = handlers.project_id.clone();
    
    let result = match tool_name {
        "context_read" => {
            // Read only the raw context from projects table (NOT the compiled version or CLAUDE.md)
            let row = sqlx::query!(
                r#"
                SELECT context
                FROM projects 
                WHERE id = $1
                "#,
                project_id
            )
            .fetch_optional(&handlers.db_pool)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Database error: {}", e),
                data: None,
            })?;
            
            match row {
                Some(record) => json!({
                    "success": true,
                    "context": record.context,
                    "message": "Raw context retrieved successfully"
                }),
                None => json!({
                    "success": false,
                    "error": "Project not found"
                })
            }
        },
        "context_update" => {
            let context = args.get("context")
                .and_then(|v| v.as_str())
                .ok_or(JsonRpcError {
                    code: INVALID_PARAMS,
                    message: "context is required".to_string(),
                    data: None,
                })?;
            
            // Update context and clear compiled cache
            sqlx::query!(
                r#"
                UPDATE projects 
                SET 
                    context = $1,
                    context_compiled = NULL,
                    context_compiled_at = NULL,
                    updated_at = NOW()
                WHERE id = $2
                "#,
                context,
                project_id
            )
            .execute(&handlers.db_pool)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Database error: {}", e),
                data: None,
            })?;
            
            json!({
                "success": true,
                "message": "Context updated successfully"
            })
        },
        "context_compile" => {
            use crate::utils::context_compiler::ContextCompiler;
            
            // Compile the context
            let compiler = ContextCompiler::new(handlers.db_pool.clone());
            match compiler.compile_context(&project_id).await {
                Ok(compiled) => {
                    // Update the compiled context in database
                    sqlx::query!(
                        r#"
                        UPDATE projects 
                        SET 
                            context_compiled = $1,
                            context_compiled_at = NOW()
                        WHERE id = $2
                        "#,
                        compiled.clone(),
                        project_id
                    )
                    .execute(&handlers.db_pool)
                    .await
                    .map_err(|e| JsonRpcError {
                        code: INTERNAL_ERROR,
                        message: format!("Database error: {}", e),
                        data: None,
                    })?;
                    
                    json!({
                        "success": true,
                        "compiled_content": compiled,
                        "message": "Context compiled successfully"
                    })
                },
                Err(e) => json!({
                    "success": false,
                    "error": e.to_string()
                })
            }
        },
        _ => unreachable!()
    };
    
    Ok(result)
}

