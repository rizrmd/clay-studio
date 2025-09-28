/// Macro to generate tool handler cases with proper MCP wrapping
#[macro_export]
macro_rules! handle_mcp_tool {
    ($tool_name:expr, $handlers:expr, $arguments:expr, $handler_method:ident) => {{
        use crate::core::mcp::handlers::base::McpHandlers;
        use crate::core::mcp::response::wrap_mcp_response;
        
        let empty_map = serde_json::Map::new();
        let args = $arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
        
        // Call the handler and get the result as a string
        let result_str = McpHandlers::$handler_method($handlers, args).await?;
        
        // Parse the string result to JSON
        let result_json = serde_json::from_str(&result_str).map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Invalid JSON response: {}", e),
            data: None,
        })?;
        
        // Wrap in MCP resource format for Claude Code CLI
        Ok(wrap_mcp_response(result_json))
    }};
}

/// Macro for tools that need custom handling
#[macro_export]
macro_rules! handle_mcp_tool_custom {
    ($handlers:expr, $arguments:expr, $handler:expr) => {{
        use crate::core::mcp::response::wrap_mcp_response;
        
        let empty_map = serde_json::Map::new();
        let args = $arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
        
        // Execute custom handler
        let result = $handler($handlers, args).await?;
        
        // Wrap in MCP resource format
        Ok(wrap_mcp_response(result))
    }};
}