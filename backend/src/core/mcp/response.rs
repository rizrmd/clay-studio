use serde_json::{json, Value};

/// Wraps a tool result in the MCP resource format that Claude Code CLI expects
pub fn wrap_mcp_response(result: Value) -> Value {
    // MCP tools should return results wrapped in a content array
    // with resource format for proper handling by Claude Code CLI
    json!({
        "content": [
            {
                "type": "resource",
                "resource": {
                    "uri": format!("mcp://tool-result/{}", uuid::Uuid::new_v4()),
                    "name": "Tool Result",
                    "mimeType": "application/json",
                    "text": result.to_string()
                }
            }
        ]
    })
}

/// Alternative format that some MCP clients expect
pub fn wrap_mcp_response_array(result: Value) -> Value {
    // Some MCP clients expect the result directly in a content array
    json!([{
        "type": "text",
        "text": result.to_string()
    }])
}

/// Wraps error responses in MCP format
pub fn wrap_mcp_error(error_message: &str) -> Value {
    json!({
        "content": [
            {
                "type": "resource",
                "resource": {
                    "uri": "mcp://error",
                    "name": "Error",
                    "mimeType": "text/plain",
                    "text": error_message
                }
            }
        ],
        "isError": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_mcp_response() {
        let result = json!({"status": "success", "data": [1, 2, 3]});
        let wrapped = wrap_mcp_response(result.clone());
        
        assert!(wrapped.get("content").is_some());
        let content = wrapped.get("content").unwrap().as_array().unwrap();
        assert_eq!(content.len(), 1);
        
        let resource = &content[0];
        assert_eq!(resource.get("type").unwrap().as_str().unwrap(), "resource");
    }
}