use super::base::McpHandlers;
use crate::core::mcp::types::*;
use serde_json::{json, Value};

// JSON-RPC error codes
const INVALID_PARAMS: i32 = -32602;
const INTERNAL_ERROR: i32 = -32603;

impl McpHandlers {
    pub async fn handle_ask_user(
        &self,
        arguments: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        // Check for required question parameter
        let question = arguments.get("question").and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing required parameter: question".to_string(),
                data: Some(json!({
                    "correct_format_example": {
                        "question": "Which database would you like to connect to?",
                        "options": ["PostgreSQL", "MySQL", "SQLite", "Oracle"]
                    }
                }))
            })?;

        // Validate question is not empty
        if question.trim().is_empty() {
            return Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: "Question cannot be empty".to_string(),
                data: Some(json!({
                    "correct_format_example": {
                        "question": "What would you like to do next?",
                        "options": ["Export data", "Create chart", "Run analysis"]
                    }
                }))
            });
        }

        // Validate options if provided
        let options = arguments
            .get("options")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>());

        if let Some(ref opts) = options {
            if opts.is_empty() {
                return Err(JsonRpcError {
                    code: INVALID_PARAMS,
                    message: "Options array cannot be empty if provided".to_string(),
                    data: Some(json!({
                        "correct_format_example": {
                            "question": "Select your preferred chart type:",
                            "options": ["Bar Chart", "Line Chart", "Pie Chart", "Scatter Plot"]
                        }
                    }))
                });
            }

            // Check for empty option values
            if opts.iter().any(|opt| opt.trim().is_empty()) {
                return Err(JsonRpcError {
                    code: INVALID_PARAMS,
                    message: "Options cannot contain empty values".to_string(),
                    data: Some(json!({
                        "correct_format_example": {
                            "question": "How would you like to proceed?",
                            "options": ["Continue", "Cancel", "Save and exit"]
                        }
                    }))
                });
            }
        }

        // Parameters are valid!
        let response = json!({
            "status": "success",
            "message": "Parameters valid. User question ready.",
            "question_info": {
                "question": question,
                "has_options": options.is_some(),
                "options_count": options.as_ref().map(|o| o.len()).unwrap_or(0),
                "interaction_type": "ask_user",
                "requires_response": true
            }
        });

        serde_json::to_string(&response)
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to serialize response: {}", e),
                data: None,
            })
    }
}
