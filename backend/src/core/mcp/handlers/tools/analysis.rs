use crate::core::mcp::types::*;
use crate::core::mcp::handlers::base::McpHandlers;
use serde_json::{json, Value};
use uuid::Uuid;

// JSON-RPC error codes
const INVALID_PARAMS: i32 = -32602;
const INTERNAL_ERROR: i32 = -32603;
const METHOD_NOT_FOUND: i32 = -32601;

/// Get analysis tools for the MCP tools list
pub fn get_analysis_tools() -> Vec<Tool> {
    vec![
        // Analysis Management Tools
        Tool {
            name: "create".to_string(),
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
            name: "list".to_string(),
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
            name: "get".to_string(),
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
            name: "update".to_string(),
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
            name: "delete".to_string(),
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
            name: "run".to_string(),
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
            name: "validate".to_string(),
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
        // Job Management Tools (jobs are for analyses)
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
    ]
}

/// Check if a tool name is an analysis tool
pub fn is_analysis_tool(tool_name: &str) -> bool {
    matches!(tool_name,
        "create" | "list" | "get" | "update" | 
        "delete" | "run" | "validate" |
        "job_list" | "job_get" | "job_cancel" | "job_result"
    )
}

/// Handle analysis tool calls
pub async fn handle_tool_call(
    handlers: &McpHandlers,
    tool_name: &str,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    match tool_name {
        "create" => handle_analysis_create(handlers, arguments).await,
        "list" => handle_analysis_list(handlers, arguments).await,
        "get" => handle_analysis_get(handlers, arguments).await,
        "update" => handle_analysis_update(handlers, arguments).await,
        "delete" => handle_analysis_delete(handlers, arguments).await,
        "run" => handle_analysis_run(handlers, arguments).await,
        "validate" => handle_analysis_validate(handlers, arguments).await,
        "job_list" => handle_job_list(handlers, arguments).await,
        "job_get" => handle_job_get(handlers, arguments).await,
        "job_cancel" => handle_job_cancel(handlers, arguments).await,
        "job_result" => handle_job_result(handlers, arguments).await,
        _ => Err(JsonRpcError {
            code: METHOD_NOT_FOUND,
            message: format!("Unknown analysis tool: {}", tool_name),
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

    let metadata = if let Some(desc) = description {
        json!({"description": desc})
    } else {
        json!({})
    };

    let project_id = handlers.project_id.parse::<Uuid>()
        .map_err(|_| JsonRpcError {
            code: INTERNAL_ERROR,
            message: "Invalid project_id in context".to_string(),
            data: None,
        })?;

    let analysis_id = Uuid::new_v4();

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
        .min(100);

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
            let analyses: Vec<Value> = rows.into_iter()
                .filter(|row| !active_only || row.is_active.unwrap_or(false))
                .map(|row| {
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
    let args = arguments
        .and_then(|v| v.as_object())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid arguments".to_string(),
            data: None,
        })?;

    let analysis_id = args.get("analysis_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid or missing analysis_id".to_string(),
            data: None,
        })?;

    let row = sqlx::query!(
        r#"
        SELECT 
            id,
            title,
            script_content,
            metadata,
            created_at,
            updated_at,
            is_active
        FROM analyses 
        WHERE id = $1 AND project_id = $2
        "#,
        analysis_id,
        handlers.project_id
    )
    .fetch_optional(&handlers.db_pool)
    .await
    .map_err(|e| JsonRpcError {
        code: INTERNAL_ERROR,
        message: format!("Database error: {}", e),
        data: None,
    })?;

    if let Some(record) = row {
        Ok(json!({
            "status": "success",
            "analysis": {
                "id": record.id,
                "title": record.title,
                "script_content": record.script_content,
                "metadata": record.metadata,
                "created_at": record.created_at,
                "updated_at": record.updated_at,
                "is_active": record.is_active
            }
        }))
    } else {
        Err(JsonRpcError {
            code: INVALID_PARAMS,
            message: format!("Analysis {} not found", analysis_id),
            data: None,
        })
    }
}

async fn handle_analysis_update(
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
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid or missing analysis_id".to_string(),
            data: None,
        })?;

    let title = args.get("title").and_then(|v| v.as_str());
    let script_content = args.get("script_content").and_then(|v| v.as_str());

    if title.is_none() && script_content.is_none() {
        return Err(JsonRpcError {
            code: INVALID_PARAMS,
            message: "No fields to update. Provide at least one of: title, script_content".to_string(),
            data: None,
        });
    }

    let result = sqlx::query!(
        r#"
        UPDATE analyses 
        SET 
            title = COALESCE($3, title),
            script_content = COALESCE($4, script_content),
            updated_at = NOW()
        WHERE id = $1 AND project_id = $2
        RETURNING id, title
        "#,
        analysis_id,
        handlers.project_id,
        title,
        script_content
    )
    .fetch_optional(&handlers.db_pool)
    .await
    .map_err(|e| JsonRpcError {
        code: INTERNAL_ERROR,
        message: format!("Database error: {}", e),
        data: None,
    })?;

    if let Some(record) = result {
        Ok(json!({
            "status": "success",
            "message": "Analysis updated successfully",
            "analysis": {
                "id": record.id,
                "title": record.title
            }
        }))
    } else {
        Err(JsonRpcError {
            code: INVALID_PARAMS,
            message: format!("Analysis {} not found or no permission", analysis_id),
            data: None,
        })
    }
}

async fn handle_analysis_delete(
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
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid or missing analysis_id".to_string(),
            data: None,
        })?;

    let result = sqlx::query!(
        r#"
        UPDATE analyses 
        SET 
            is_active = false,
            updated_at = NOW()
        WHERE id = $1 AND project_id = $2 AND is_active = true
        RETURNING id, title
        "#,
        analysis_id,
        handlers.project_id
    )
    .fetch_optional(&handlers.db_pool)
    .await
    .map_err(|e| JsonRpcError {
        code: INTERNAL_ERROR,
        message: format!("Database error: {}", e),
        data: None,
    })?;

    if let Some(record) = result {
        Ok(json!({
            "status": "success",
            "message": "Analysis deleted (marked as inactive)",
            "analysis": {
                "id": record.id,
                "title": record.title
            }
        }))
    } else {
        Err(JsonRpcError {
            code: INVALID_PARAMS,
            message: format!("Analysis {} not found or already deleted", analysis_id),
            data: None,
        })
    }
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
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid or missing analysis_id".to_string(),
            data: None,
        })?;

    let parameters = args.get("parameters")
        .cloned()
        .unwrap_or(json!({}));

    match sqlx::query!(
        r#"
        INSERT INTO analysis_jobs (id, analysis_id, status, parameters, triggered_by)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        Uuid::new_v4(),
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
    let args = arguments
        .and_then(|v| v.as_object())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid arguments".to_string(),
            data: None,
        })?;

    let analysis_id = args.get("analysis_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid or missing analysis_id".to_string(),
            data: None,
        })?;

    let script_content = if let Some(content) = args.get("script_content").and_then(|v| v.as_str()) {
        content.to_string()
    } else {
        let row = sqlx::query!(
            r#"
            SELECT script_content 
            FROM analyses 
            WHERE id = $1 AND project_id = $2
            "#,
            analysis_id,
            handlers.project_id
        )
        .fetch_optional(&handlers.db_pool)
        .await
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Database error: {}", e),
            data: None,
        })?;

        row.map(|r| r.script_content)
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: format!("Analysis {} not found", analysis_id),
                data: None,
            })?
    };

    // Basic JavaScript validation
    let mut validation_errors = Vec::new();
    let mut warnings = Vec::new();
    
    if !script_content.contains("export default") && !script_content.contains("module.exports") {
        warnings.push("Script should export a default function or module");
    }
    
    if !script_content.contains("async") && !script_content.contains("function") {
        validation_errors.push("Script must contain at least one function");
    }
    
    if script_content.contains("console.log") {
        warnings.push("Consider using context.log() instead of console.log()");
    }
    
    if script_content.contains("require(") && !script_content.contains("import ") {
        warnings.push("Consider using ES6 imports instead of require()");
    }

    if script_content.contains("eval(") || script_content.contains("Function(") {
        validation_errors.push("Script contains potentially dangerous eval() or Function() constructor");
    }
    
    if script_content.contains("process.exit") || script_content.contains("process.kill") {
        validation_errors.push("Script should not terminate the process");
    }

    let is_valid = validation_errors.is_empty();
    
    Ok(json!({
        "status": if is_valid { "success" } else { "error" },
        "valid": is_valid,
        "analysis_id": analysis_id,
        "validation": {
            "errors": validation_errors,
            "warnings": warnings,
            "script_length": script_content.len(),
            "line_count": script_content.lines().count()
        },
        "message": if is_valid {
            "Script validation passed"
        } else {
            "Script validation failed"
        }
    }))
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
        .and_then(|s| Uuid::parse_str(s).ok());

    let status_filter = args.get("status")
        .and_then(|v| v.as_str());

    let limit = args.get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(50)
        .min(100);

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

    let jobs: Vec<Value> = rows.into_iter()
        .map(|row| {
            use sqlx::Row;
            json!({
                "job_id": row.get::<Uuid, _>("id"),
                "analysis_id": row.get::<Uuid, _>("analysis_id"),
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
        .and_then(|s| Uuid::parse_str(s).ok())
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
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Invalid or missing job_id".to_string(),
            data: None,
        })?;

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
            if job.project_id != handlers.project_id {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "Job not found or access denied".to_string(),
                    data: None,
                });
            }

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
        .and_then(|s| Uuid::parse_str(s).ok())
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
                        "result": row.result.unwrap_or(json!({}))
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