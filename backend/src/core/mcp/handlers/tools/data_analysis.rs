use crate::core::mcp::types::*;
use crate::core::mcp::handlers::base::McpHandlers;
use serde_json::{json, Value};

// JSON-RPC error codes
const INVALID_PARAMS: i32 = -32602;
const INTERNAL_ERROR: i32 = -32603;
const METHOD_NOT_FOUND: i32 = -32601;

/// Get data analysis tools for the MCP tools list
pub fn get_data_analysis_tools() -> Vec<Tool> {
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
        // Analysis Management Tools
        Tool {
            name: "analysis_create".to_string(),
            description: "Create a new analysis with JavaScript code".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Human-readable title for the analysis"
                    },
                    "script_content": {
                        "type": "string",
                        "description": "JavaScript code for the analysis. Should export default object with run function."
                    },
                    "description": {
                        "type": "string",
                        "description": "Optional description of what the analysis does"
                    },
                    "parameters": {
                        "type": "object",
                        "description": "Optional parameter definitions for the analysis",
                        "additionalProperties": true
                    }
                },
                "required": ["title", "script_content"]
            }),
        },
        Tool {
            name: "analysis_list".to_string(),
            description: "List all analyses in the project".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "active_only": {
                        "type": "boolean",
                        "default": true,
                        "description": "Whether to list only active analyses (default: true)"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "default": 50,
                        "description": "Maximum number of analyses to return"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "analysis_get".to_string(),
            description: "Get detailed information about a specific analysis".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "analysis_id": {
                        "type": "string",
                        "description": "ID of the analysis to retrieve"
                    }
                },
                "required": ["analysis_id"]
            }),
        },
        Tool {
            name: "analysis_update".to_string(),
            description: "Update an existing analysis".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "analysis_id": {
                        "type": "string",
                        "description": "ID of the analysis to update"
                    },
                    "title": {
                        "type": "string",
                        "description": "New title for the analysis"
                    },
                    "script_content": {
                        "type": "string",
                        "description": "New JavaScript code for the analysis"
                    },
                    "description": {
                        "type": "string",
                        "description": "New description for the analysis"
                    }
                },
                "required": ["analysis_id"]
            }),
        },
        Tool {
            name: "analysis_delete".to_string(),
            description: "Delete an analysis (marks as inactive)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "analysis_id": {
                        "type": "string",
                        "description": "ID of the analysis to delete"
                    }
                },
                "required": ["analysis_id"]
            }),
        },
        Tool {
            name: "analysis_run".to_string(),
            description: "Execute an analysis with given parameters".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "analysis_id": {
                        "type": "string",
                        "description": "ID of the analysis to run"
                    },
                    "parameters": {
                        "type": "object",
                        "description": "Parameters to pass to the analysis",
                        "additionalProperties": true
                    },
                    "datasources": {
                        "type": "object",
                        "description": "Datasource mappings for the analysis",
                        "additionalProperties": true
                    }
                },
                "required": ["analysis_id"]
            }),
        },
        Tool {
            name: "analysis_validate".to_string(),
            description: "Validate an analysis script without running it".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "analysis_id": {
                        "type": "string",
                        "description": "ID of the analysis to validate"
                    },
                    "script_content": {
                        "type": "string",
                        "description": "Optional: script content to validate instead of stored script"
                    }
                },
                "required": ["analysis_id"]
            }),
        },
        Tool {
            name: "job_list".to_string(),
            description: "List analysis execution jobs".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "analysis_id": {
                        "type": "string",
                        "description": "Optional: filter jobs by analysis ID"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["pending", "running", "completed", "failed", "cancelled"],
                        "description": "Optional: filter jobs by status"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "default": 20,
                        "description": "Maximum number of jobs to return"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "job_get".to_string(),
            description: "Get detailed information about a specific job".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "job_id": {
                        "type": "string",
                        "description": "ID of the job to retrieve"
                    }
                },
                "required": ["job_id"]
            }),
        },
        Tool {
            name: "job_cancel".to_string(),
            description: "Cancel a running analysis job".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "job_id": {
                        "type": "string",
                        "description": "ID of the job to cancel"
                    }
                },
                "required": ["job_id"]
            }),
        },
        Tool {
            name: "job_result".to_string(),
            description: "Get the result of a completed analysis job".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "job_id": {
                        "type": "string",
                        "description": "ID of the job to get results for"
                    }
                },
                "required": ["job_id"]
            }),
        },
        Tool {
            name: "schedule_create".to_string(),
            description: "Create a scheduled execution for an analysis".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "analysis_id": {
                        "type": "string",
                        "description": "ID of the analysis to schedule"
                    },
                    "cron_expression": {
                        "type": "string",
                        "description": "Cron expression for schedule (e.g., '0 9 * * *' for daily at 9 AM)"
                    },
                    "timezone": {
                        "type": "string",
                        "default": "UTC",
                        "description": "Timezone for the schedule (default: UTC)"
                    },
                    "parameters": {
                        "type": "object",
                        "description": "Default parameters for scheduled executions",
                        "additionalProperties": true
                    },
                    "enabled": {
                        "type": "boolean",
                        "default": true,
                        "description": "Whether the schedule is enabled (default: true)"
                    }
                },
                "required": ["analysis_id", "cron_expression"]
            }),
        },
        Tool {
            name: "schedule_list".to_string(),
            description: "List scheduled analyses".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "analysis_id": {
                        "type": "string",
                        "description": "Optional: filter schedules by analysis ID"
                    },
                    "enabled_only": {
                        "type": "boolean",
                        "default": true,
                        "description": "Whether to list only enabled schedules (default: true)"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "schedule_update".to_string(),
            description: "Update a scheduled analysis".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "schedule_id": {
                        "type": "string",
                        "description": "ID of the schedule to update"
                    },
                    "cron_expression": {
                        "type": "string",
                        "description": "New cron expression"
                    },
                    "timezone": {
                        "type": "string",
                        "description": "New timezone"
                    },
                    "enabled": {
                        "type": "boolean",
                        "description": "Whether the schedule should be enabled"
                    },
                    "parameters": {
                        "type": "object",
                        "description": "New default parameters",
                        "additionalProperties": true
                    }
                },
                "required": ["schedule_id"]
            }),
        },
        Tool {
            name: "schedule_delete".to_string(),
            description: "Delete a scheduled analysis".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "schedule_id": {
                        "type": "string",
                        "description": "ID of the schedule to delete"
                    }
                },
                "required": ["schedule_id"]
            }),
        },
        Tool {
            name: "monitoring_status".to_string(),
            description: "Get system monitoring and health status".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "include_metrics": {
                        "type": "boolean",
                        "default": true,
                        "description": "Include performance metrics (default: true)"
                    },
                    "include_system_health": {
                        "type": "boolean", 
                        "default": true,
                        "description": "Include system health info (default: true)"
                    }
                },
                "additionalProperties": false
            }),
        },
    ]
}

/// Check if a tool name is a data analysis tool
pub fn is_data_analysis_tool(tool_name: &str) -> bool {
    matches!(tool_name,
        // Datasource tools
        "datasource_add" | "datasource_list" | "datasource_remove" | "datasource_update" |
        "connection_test" | "datasource_detail" | "datasource_query" | "datasource_inspect" |
        "schema_get" | "schema_search" | "schema_related" | "schema_stats" |
        // Analysis management tools
        "analysis_create" | "analysis_list" | "analysis_get" | "analysis_update" | "analysis_delete" |
        "analysis_run" | "analysis_validate" |
        // Job management tools  
        "job_list" | "job_get" | "job_cancel" | "job_result" |
        // Schedule management tools
        "schedule_create" | "schedule_list" | "schedule_update" | "schedule_delete" |
        // Monitoring tools
        "monitoring_status"
    )
}

/// Handle data analysis tool calls
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
        // Analysis management tools
        "analysis_create" => {
            handle_analysis_create(handlers, arguments).await
        },
        "analysis_list" => {
            handle_analysis_list(handlers, arguments).await
        },
        "analysis_get" => {
            handle_analysis_get(handlers, arguments).await
        },
        "analysis_update" => {
            handle_analysis_update(handlers, arguments).await
        },
        "analysis_delete" => {
            handle_analysis_delete(handlers, arguments).await
        },
        "analysis_run" => {
            handle_analysis_run(handlers, arguments).await
        },
        "analysis_validate" => {
            handle_analysis_validate(handlers, arguments).await
        },
        // Job management tools
        "job_list" => {
            handle_job_list(handlers, arguments).await
        },
        "job_get" => {
            handle_job_get(handlers, arguments).await
        },
        "job_cancel" => {
            handle_job_cancel(handlers, arguments).await
        },
        "job_result" => {
            handle_job_result(handlers, arguments).await
        },
        // Schedule management tools
        "schedule_create" => {
            handle_schedule_create(handlers, arguments).await
        },
        "schedule_list" => {
            handle_schedule_list(handlers, arguments).await
        },
        "schedule_update" => {
            handle_schedule_update(handlers, arguments).await
        },
        "schedule_delete" => {
            handle_schedule_delete(handlers, arguments).await
        },
        // Monitoring tools
        "monitoring_status" => {
            handle_monitoring_status(handlers, arguments).await
        },
        _ => Err(JsonRpcError {
            code: METHOD_NOT_FOUND,
            message: format!("Unknown data analysis tool: {}", tool_name),
            data: None,
        })
    }
}

// Analysis Management Handlers
async fn handle_analysis_create(
    handlers: &McpHandlers,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let args = arguments
        .and_then(|v| v.as_object())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid arguments".to_string(),
            data: None,
        })?;

    let title = args.get("title")
        .and_then(|v| v.as_str())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Missing required field: title".to_string(),
            data: None,
        })?;

    let script_content = args.get("script_content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Missing required field: script_content".to_string(),
            data: None,
        })?;

    let description = args.get("description")
        .and_then(|v| v.as_str());

    // Create metadata with description if provided
    let metadata = if let Some(desc) = description {
        serde_json::json!({"description": desc})
    } else {
        serde_json::json!({})
    };

    // For now, we'll use a default project_id - in full implementation this would come from context
    let project_id = handlers.project_id.parse::<uuid::Uuid>()
        .map_err(|_| JsonRpcError {
            code: INTERNAL_ERROR,
            message: "Invalid project_id in context".to_string(),
            data: None,
        })?;

    let analysis_id = uuid::Uuid::new_v4();

    match sqlx::query!(
        r#"
        INSERT INTO analyses (id, title, script_content, metadata, project_id, is_active, version, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
        "#,
        analysis_id,
        title,
        script_content,
        metadata,
        project_id.to_string(),
        true,
        1i32
    )
    .execute(&handlers.db_pool)
    .await {
        Ok(_) => {
            Ok(json!({
                "analysis_id": analysis_id,
                "title": title,
                "status": "created",
                "message": "Analysis created successfully",
                "project_id": project_id
            }))
        }
        Err(e) => {
            Err(JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to create analysis: {}", e),
                data: None,
            })
        }
    }
}

async fn handle_analysis_list(
    handlers: &McpHandlers,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let default_args = serde_json::Map::new();
    let args = arguments
        .and_then(|v| v.as_object())
        .unwrap_or(&default_args);

    let active_only = args.get("active_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let limit = args.get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(50)
        .min(100); // Cap at 100

    // Always fetch all analyses and filter in Rust to avoid sqlx Record type issues
    let query_result = sqlx::query!(
        r#"
        SELECT id, title, metadata, created_at, updated_at, version, is_active
        FROM analyses 
        WHERE project_id = $1
        ORDER BY updated_at DESC
        LIMIT $2
        "#,
        handlers.project_id,
        limit
    )
    .fetch_all(&handlers.db_pool)
    .await;

    match query_result {
        Ok(rows) => {
            let analyses: Vec<serde_json::Value> = rows.into_iter()
                .filter(|row| !active_only || row.is_active.unwrap_or(false))
                .map(|row| {
                    // Extract description from metadata if it exists
                    let description = row.metadata.as_ref()
                        .and_then(|m| m.get("description"))
                        .and_then(|d| d.as_str())
                        .unwrap_or("");

                    json!({
                        "analysis_id": row.id,
                        "title": row.title,
                        "description": description,
                        "version": row.version,
                        "is_active": row.is_active,
                        "created_at": row.created_at,
                        "updated_at": row.updated_at,
                        "metadata": row.metadata
                    })
                }).collect();

            Ok(json!({
                "analyses": analyses,
                "count": analyses.len(),
                "project_id": handlers.project_id
            }))
        }
        Err(e) => {
            Err(JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to list analyses: {}", e),
                data: None,
            })
        }
    }
}

async fn handle_analysis_get(
    handlers: &McpHandlers,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let _ = (handlers, arguments);
    Err(JsonRpcError {
        code: INTERNAL_ERROR,
        message: "Analysis management not yet implemented - requires database migrations".to_string(),
        data: Some(json!({
            "hint": "Run database migrations first: sqlx migrate run --source ./migrations",
            "status": "analysis_system_not_initialized"
        })),
    })
}

async fn handle_analysis_update(
    handlers: &McpHandlers,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let _ = (handlers, arguments);
    Err(JsonRpcError {
        code: INTERNAL_ERROR,
        message: "Analysis management not yet implemented - requires database migrations".to_string(),
        data: Some(json!({
            "hint": "Run database migrations first: sqlx migrate run --source ./migrations",
            "status": "analysis_system_not_initialized"
        })),
    })
}

async fn handle_analysis_delete(
    handlers: &McpHandlers,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let _ = (handlers, arguments);
    Err(JsonRpcError {
        code: INTERNAL_ERROR,
        message: "Analysis management not yet implemented - requires database migrations".to_string(),
        data: Some(json!({
            "hint": "Run database migrations first: sqlx migrate run --source ./migrations",
            "status": "analysis_system_not_initialized"
        })),
    })
}

async fn handle_analysis_run(
    handlers: &McpHandlers,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let args = arguments
        .and_then(|v| v.as_object())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid arguments".to_string(),
            data: None,
        })?;

    let analysis_id = args.get("analysis_id")
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid or missing analysis_id".to_string(),
            data: None,
        })?;

    let parameters = args.get("parameters")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    // For now, create a simple job record - in full implementation this would use AnalysisService
    match sqlx::query!(
        r#"
        INSERT INTO analysis_jobs (id, analysis_id, status, parameters, triggered_by)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        uuid::Uuid::new_v4(),
        analysis_id,
        "pending",
        parameters,
        "mcp_server"
    )
    .execute(&handlers.db_pool)
    .await {
        Ok(_) => {
            Ok(json!({
                "success": true,
                "message": "Analysis job submitted successfully",
                "analysis_id": analysis_id,
                "status": "Job created and queued for execution"
            }))
        }
        Err(e) => {
            Err(JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to create analysis job: {}", e),
                data: None,
            })
        }
    }
}

async fn handle_analysis_validate(
    handlers: &McpHandlers,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let _ = (handlers, arguments);
    Err(JsonRpcError {
        code: INTERNAL_ERROR,
        message: "Analysis management not yet implemented - requires database migrations".to_string(),
        data: Some(json!({
            "hint": "Run database migrations first: sqlx migrate run --source ./migrations",
            "status": "analysis_system_not_initialized"
        })),
    })
}

// Job Management Handlers
async fn handle_job_list(
    handlers: &McpHandlers,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let default_args = serde_json::Map::new();
    let args = arguments
        .and_then(|v| v.as_object())
        .unwrap_or(&default_args);

    let analysis_id = args.get("analysis_id")
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok());

    let status_filter = args.get("status")
        .and_then(|v| v.as_str());

    let limit = args.get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(50)
        .min(100); // Cap at 100

    // Use dynamic query to handle different filter combinations
    let mut query_str = String::from(
        "SELECT aj.id, aj.analysis_id, aj.status, aj.created_at, aj.started_at, 
                aj.completed_at, a.title as analysis_title
         FROM analysis_jobs aj
         LEFT JOIN analyses a ON aj.analysis_id = a.id
         WHERE a.project_id = $1"
    );
    
    let mut param_count = 2;
    if analysis_id.is_some() {
        query_str.push_str(&format!(" AND aj.analysis_id = ${}", param_count));
        param_count += 1;
    }
    
    if status_filter.is_some() {
        query_str.push_str(&format!(" AND aj.status = ${}", param_count));
        param_count += 1;
    }
    
    query_str.push_str(&format!(" ORDER BY aj.created_at DESC LIMIT ${}", param_count));
    
    // Build and execute query
    let mut query = sqlx::query(&query_str).bind(&handlers.project_id);
    
    if let Some(aid) = analysis_id {
        query = query.bind(aid);
    }
    
    if let Some(status) = status_filter {
        query = query.bind(status);
    }
    
    query = query.bind(limit);
    
    let rows = query
        .fetch_all(&handlers.db_pool)
        .await
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Failed to list jobs: {}", e),
            data: None,
        })?;

    let jobs: Vec<serde_json::Value> = rows.into_iter()
        .map(|row| {
            use sqlx::Row;
            json!({
                "job_id": row.get::<uuid::Uuid, _>("id"),
                "analysis_id": row.get::<uuid::Uuid, _>("analysis_id"),
                "analysis_title": row.get::<Option<String>, _>("analysis_title"),
                "status": row.get::<String, _>("status"),
                "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
                "started_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("started_at"),
                "completed_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("completed_at")
            })
        }).collect();

    Ok(json!({
        "jobs": jobs,
        "count": jobs.len(),
        "project_id": handlers.project_id
    }))
}

async fn handle_job_get(
    handlers: &McpHandlers,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let args = arguments
        .and_then(|v| v.as_object())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid arguments".to_string(),
            data: None,
        })?;

    let job_id = args.get("job_id")
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid or missing job_id".to_string(),
            data: None,
        })?;

    match sqlx::query!(
        r#"
        SELECT aj.id, aj.analysis_id, aj.status, aj.parameters, aj.result, 
               aj.error_message, aj.created_at, aj.started_at, aj.completed_at,
               a.title as analysis_title
        FROM analysis_jobs aj
        LEFT JOIN analyses a ON aj.analysis_id = a.id
        WHERE aj.id = $1
        "#,
        job_id
    )
    .fetch_optional(&handlers.db_pool)
    .await {
        Ok(Some(row)) => {
            Ok(json!({
                "job_id": row.id,
                "analysis_id": row.analysis_id,
                "analysis_title": row.analysis_title,
                "status": row.status,
                "parameters": row.parameters,
                "result": row.result,
                "error_message": row.error_message,
                "created_at": row.created_at,
                "started_at": row.started_at,
                "completed_at": row.completed_at
            }))
        }
        Ok(None) => {
            Err(JsonRpcError {
                code: -32602,
                message: format!("Job {} not found", job_id),
                data: None,
            })
        }
        Err(e) => {
            Err(JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Database error: {}", e),
                data: None,
            })
        }
    }
}

async fn handle_job_cancel(
    handlers: &McpHandlers,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let args = arguments
        .and_then(|v| v.as_object())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid arguments".to_string(),
            data: None,
        })?;

    let job_id = args.get("job_id")
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid or missing job_id".to_string(),
            data: None,
        })?;

    // First check if the job exists and get its current status
    let current_job = sqlx::query!(
        r#"
        SELECT aj.status, a.project_id
        FROM analysis_jobs aj
        LEFT JOIN analyses a ON aj.analysis_id = a.id
        WHERE aj.id = $1
        "#,
        job_id
    )
    .fetch_optional(&handlers.db_pool)
    .await
    .map_err(|e| JsonRpcError {
        code: INTERNAL_ERROR,
        message: format!("Database error: {}", e),
        data: None,
    })?;

    match current_job {
        Some(job) => {
            // Verify the job belongs to the current project
            if job.project_id != handlers.project_id {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "Job not found or access denied".to_string(),
                    data: None,
                });
            }

            // Check if the job can be cancelled
            match job.status.as_str() {
                "completed" | "failed" | "cancelled" => {
                    return Err(JsonRpcError {
                        code: -32602,
                        message: format!("Job cannot be cancelled - current status: {}", job.status),
                        data: None,
                    });
                }
                _ => {}
            }

            // Update the job status to cancelled
            sqlx::query!(
                r#"
                UPDATE analysis_jobs 
                SET status = 'cancelled', 
                    completed_at = NOW(),
                    error_message = 'Job cancelled by user'
                WHERE id = $1
                "#,
                job_id
            )
            .execute(&handlers.db_pool)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to cancel job: {}", e),
                data: None,
            })?;

            Ok(json!({
                "success": true,
                "job_id": job_id,
                "status": "cancelled",
                "message": "Job successfully cancelled"
            }))
        }
        None => {
            Err(JsonRpcError {
                code: -32602,
                message: format!("Job {} not found", job_id),
                data: None,
            })
        }
    }
}

async fn handle_job_result(
    handlers: &McpHandlers,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let args = arguments
        .and_then(|v| v.as_object())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid arguments".to_string(),
            data: None,
        })?;

    let job_id = args.get("job_id")
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid or missing job_id".to_string(),
            data: None,
        })?;

    match sqlx::query!(
        "SELECT status, result, error_message FROM analysis_jobs WHERE id = $1",
        job_id
    )
    .fetch_optional(&handlers.db_pool)
    .await {
        Ok(Some(row)) => {
            match row.status.as_str() {
                "completed" => {
                    Ok(json!({
                        "job_id": job_id,
                        "status": "completed",
                        "result": row.result.unwrap_or(serde_json::json!({}))
                    }))
                }
                "failed" => {
                    Ok(json!({
                        "job_id": job_id,
                        "status": "failed",
                        "error": row.error_message.unwrap_or("Unknown error".to_string())
                    }))
                }
                "running" => {
                    Ok(json!({
                        "job_id": job_id,
                        "status": "running",
                        "message": "Analysis is still running"
                    }))
                }
                "pending" => {
                    Ok(json!({
                        "job_id": job_id,
                        "status": "pending",
                        "message": "Analysis is queued for execution"
                    }))
                }
                _ => {
                    Ok(json!({
                        "job_id": job_id,
                        "status": row.status,
                        "message": "Unknown status"
                    }))
                }
            }
        }
        Ok(None) => {
            Err(JsonRpcError {
                code: -32602,
                message: format!("Job {} not found", job_id),
                data: None,
            })
        }
        Err(e) => {
            Err(JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Database error: {}", e),
                data: None,
            })
        }
    }
}

// Schedule Management Handlers
async fn handle_schedule_create(
    handlers: &McpHandlers,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let _ = (handlers, arguments);
    Err(JsonRpcError {
        code: INTERNAL_ERROR,
        message: "Schedule management not yet implemented - requires database migrations".to_string(),
        data: Some(json!({
            "hint": "Run database migrations first: sqlx migrate run --source ./migrations",
            "status": "analysis_system_not_initialized"
        })),
    })
}

async fn handle_schedule_list(
    handlers: &McpHandlers,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let _ = (handlers, arguments);
    Err(JsonRpcError {
        code: INTERNAL_ERROR,
        message: "Schedule management not yet implemented - requires database migrations".to_string(),
        data: Some(json!({
            "hint": "Run database migrations first: sqlx migrate run --source ./migrations",
            "status": "analysis_system_not_initialized"
        })),
    })
}

async fn handle_schedule_update(
    handlers: &McpHandlers,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let _ = (handlers, arguments);
    Err(JsonRpcError {
        code: INTERNAL_ERROR,
        message: "Schedule management not yet implemented - requires database migrations".to_string(),
        data: Some(json!({
            "hint": "Run database migrations first: sqlx migrate run --source ./migrations",
            "status": "analysis_system_not_initialized"
        })),
    })
}

async fn handle_schedule_delete(
    handlers: &McpHandlers,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let _ = (handlers, arguments);
    Err(JsonRpcError {
        code: INTERNAL_ERROR,
        message: "Schedule management not yet implemented - requires database migrations".to_string(),
        data: Some(json!({
            "hint": "Run database migrations first: sqlx migrate run --source ./migrations",
            "status": "analysis_system_not_initialized"
        })),
    })
}

// Monitoring Handler
async fn handle_monitoring_status(
    handlers: &McpHandlers,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let _ = (handlers, arguments);
    Err(JsonRpcError {
        code: INTERNAL_ERROR,
        message: "Monitoring not yet implemented - requires database migrations".to_string(),
        data: Some(json!({
            "hint": "Run database migrations first: sqlx migrate run --source ./migrations",
            "status": "analysis_system_not_initialized"
        })),
    })
}
// Placeholder exports for missing types
#[allow(dead_code)]
pub struct DataAnalysisHandlers;
#[allow(dead_code)]
pub struct AnalysisRequest;
#[allow(dead_code)]
pub struct AnalysisResponse;
