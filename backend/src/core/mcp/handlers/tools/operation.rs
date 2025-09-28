use crate::core::mcp::types::*;
use crate::core::mcp::handlers::base::McpHandlers;
use crate::core::mcp::response::wrap_mcp_response;
use serde_json::Value;

// JSON-RPC error codes
const INTERNAL_ERROR: i32 = -32603;
const METHOD_NOT_FOUND: i32 = -32601;

/// Get the list of operation tool names for registration
pub fn get_operation_tools() -> Vec<&'static str> {
    vec![
        "datasource_add",
        "datasource_list",
        "datasource_remove",
        "datasource_update",
        "datasource_detail",
        "connection_test",
        "datasource_query",
        "datasource_inspect",
        "schema_get",
        "schema_search",
        "schema_related",
        "schema_stats",
        "context_read",
        "context_update",
        "context_compile",
    ]
}

/// Check if a tool name is an operation tool
pub fn is_operation_tool(tool_name: &str) -> bool {
    // Delegate to operation_impl module
    super::operation_impl::is_operation_tool(tool_name)
}

/// Get operation tool definitions for MCP tools list
pub fn get_operation_tool_definitions() -> Vec<Tool> {
    // Delegate to operation_impl module for tool definitions
    super::operation_impl::get_operation_tools()
}

/// Handle operation tool calls with proper MCP wrapping
pub async fn handle_tool_call(
    handlers: &McpHandlers,
    tool_name: &str,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    // Get the result from the specific handler
    let result = match tool_name {
        // Datasource management tools
        "datasource_add" => handle_datasource_tool(handlers, tool_name, arguments).await?,
        "datasource_list" => handle_datasource_tool(handlers, tool_name, arguments).await?,
        "datasource_remove" => handle_datasource_tool(handlers, tool_name, arguments).await?,
        "datasource_update" => handle_datasource_tool(handlers, tool_name, arguments).await?,
        "datasource_detail" => handle_datasource_tool(handlers, tool_name, arguments).await?,
        "connection_test" => handle_datasource_tool(handlers, tool_name, arguments).await?,
        
        // Schema tools
        "schema_get" => handle_schema_tool(handlers, tool_name, arguments).await?,
        "schema_search" => handle_schema_tool(handlers, tool_name, arguments).await?,
        "schema_related" => handle_schema_tool(handlers, tool_name, arguments).await?,
        "schema_stats" => handle_schema_tool(handlers, tool_name, arguments).await?,
        
        // Query tools
        "datasource_query" => handle_query_tool(handlers, tool_name, arguments).await?,
        "datasource_inspect" => handle_query_tool(handlers, tool_name, arguments).await?,
        
        // Context tools
        "context_read" => handle_context_tool(handlers, tool_name, arguments).await?,
        "context_update" => handle_context_tool(handlers, tool_name, arguments).await?,
        "context_compile" => handle_context_tool(handlers, tool_name, arguments).await?,
        
        _ => {
            return Err(JsonRpcError {
                code: METHOD_NOT_FOUND,
                message: format!("Unknown operation tool: {}", tool_name),
                data: None,
            });
        }
    };
    
    // If we have a tool_use_id in arguments, update the database directly
    if let Some(args) = arguments {
        if let Some(tool_use_id) = args.get("__mcp_tool_use_id__").and_then(|v| v.as_str()) {
            tracing::info!("📝 Operation: Received tool_use_id {} for tool {}, updating database", tool_use_id, tool_name);
            update_tool_usage_in_db(handlers, tool_use_id, &result).await;
        } else {
            tracing::warn!("⚠️ Operation: No __mcp_tool_use_id__ found in arguments for tool {}", tool_name);
            tracing::debug!("Operation: Available args keys: {:?}", args.as_object().map(|o| o.keys().collect::<Vec<_>>()));
        }
    } else {
        tracing::warn!("⚠️ Operation: No arguments provided for tool {}", tool_name);
    }
    
    Ok(result)
}

/// Handle datasource-related tools
async fn handle_datasource_tool(
    handlers: &McpHandlers,
    tool_name: &str,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let empty_map = serde_json::Map::new();
    let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
    
    // Call the appropriate handler method
    let result_str = match tool_name {
        "datasource_add" => handlers.handle_datasource_add(args).await?,
        "datasource_list" => handlers.handle_datasource_list(args).await?,
        "datasource_remove" => handlers.handle_datasource_remove(args).await?,
        "datasource_update" => handlers.handle_datasource_update(args).await?,
        "datasource_detail" => handlers.handle_datasource_detail(args).await?,
        "connection_test" => handlers.handle_connection_test(args).await?,
        _ => unreachable!(),
    };
    
    // Parse and wrap the result
    let result_json = serde_json::from_str(&result_str).map_err(|e| JsonRpcError {
        code: INTERNAL_ERROR,
        message: format!("Invalid JSON response: {}", e),
        data: None,
    })?;
    
    Ok(wrap_mcp_response(result_json))
}

/// Handle schema-related tools
async fn handle_schema_tool(
    handlers: &McpHandlers,
    tool_name: &str,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let empty_map = serde_json::Map::new();
    let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
    
    let result_str = match tool_name {
        "schema_get" => handlers.handle_schema_get(args).await?,
        "schema_search" => handlers.handle_schema_search(args).await?,
        "schema_related" => handlers.handle_schema_related(args).await?,
        "schema_stats" => handlers.handle_schema_stats(args).await?,
        _ => unreachable!(),
    };
    
    let result_json = serde_json::from_str(&result_str).map_err(|e| JsonRpcError {
        code: INTERNAL_ERROR,
        message: format!("Invalid JSON response: {}", e),
        data: None,
    })?;
    
    Ok(wrap_mcp_response(result_json))
}

/// Handle query and inspection tools
async fn handle_query_tool(
    handlers: &McpHandlers,
    tool_name: &str,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    let empty_map = serde_json::Map::new();
    let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
    
    let result_str = match tool_name {
        "datasource_query" => handlers.handle_datasource_query(args).await?,
        "datasource_inspect" => handlers.handle_datasource_inspect(args).await?,
        _ => unreachable!(),
    };
    
    let result_json = serde_json::from_str(&result_str).map_err(|e| JsonRpcError {
        code: INTERNAL_ERROR,
        message: format!("Invalid JSON response: {}", e),
        data: None,
    })?;
    
    Ok(wrap_mcp_response(result_json))
}


/// Handle context-related tools
async fn handle_context_tool(
    handlers: &McpHandlers,
    tool_name: &str,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    // Delegate to operation_impl for actual handling
    super::operation_impl::handle_tool_call(handlers, tool_name, arguments).await
}

/// Update tool_usages table directly from MCP server
async fn update_tool_usage_in_db(
    handlers: &McpHandlers,
    tool_use_id: &str,
    result: &Value
) {
    // Extract the actual result from the MCP wrapper
    let actual_result = if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
        if let Some(first) = content.first() {
            if let Some(text) = first.get("resource").and_then(|r| r.get("text")).and_then(|t| t.as_str()) {
                // Parse the text back to JSON
                serde_json::from_str(text).unwrap_or(result.clone())
            } else {
                result.clone()
            }
        } else {
            result.clone()
        }
    } else {
        result.clone()
    };
    
    // Update the database
    if let Err(e) = sqlx::query(
        "UPDATE tool_usages SET output = $1, updated_at = NOW() WHERE tool_use_id = $2"
    )
    .bind(&actual_result)
    .bind(tool_use_id)
    .execute(&handlers.db_pool)
    .await {
        tracing::error!("Failed to update tool_usages from MCP server: {}", e);
    } else {
        tracing::info!("Successfully updated tool_usages for tool_use_id: {}", tool_use_id);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_tool_categorization() {
        assert!(matches!("datasource_add", "datasource_add" | "datasource_list" | "datasource_remove" | "datasource_update" | "datasource_detail" | "connection_test"));
        assert!(matches!("schema_get", "schema_get" | "schema_search" | "schema_related" | "schema_stats"));
    }
}