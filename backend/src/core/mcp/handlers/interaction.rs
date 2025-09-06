use super::base::McpHandlers;
use crate::core::mcp::types::*;
use serde_json::{json, Value};

impl McpHandlers {
    pub async fn handle_ask_user(
        &self,
        arguments: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        // Check for required question parameter
        let question = match arguments.get("question").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => {
                let response = json!({
                    "status": "error",
                    "error": "Invalid parameter format for ask_user",
                    "message": "Missing required 'question' parameter",
                    "correct_format_example": {
                        "question": "Which database would you like to connect to?",
                        "options": ["PostgreSQL", "MySQL", "SQLite", "Oracle"]
                    }
                });
                return Ok(serde_json::to_string(&response).unwrap());
            }
        };

        // Validate question is not empty
        if question.trim().is_empty() {
            let response = json!({
                "status": "error",
                "error": "Invalid question parameter",
                "message": "Question cannot be empty",
                "correct_format_example": {
                    "question": "What would you like to do next?",
                    "options": ["Export data", "Create chart", "Run analysis"]
                }
            });
            return Ok(serde_json::to_string(&response).unwrap());
        }

        // Validate options if provided
        let options = arguments
            .get("options")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>());

        if let Some(ref opts) = options {
            if opts.is_empty() {
                let response = json!({
                    "status": "error",
                    "error": "Invalid options parameter",
                    "message": "Options array cannot be empty if provided",
                    "correct_format_example": {
                        "question": "Select your preferred chart type:",
                        "options": ["Bar Chart", "Line Chart", "Pie Chart", "Scatter Plot"]
                    }
                });
                return Ok(serde_json::to_string(&response).unwrap());
            }

            // Check for empty option values
            if opts.iter().any(|opt| opt.trim().is_empty()) {
                let response = json!({
                    "status": "error",
                    "error": "Invalid option values",
                    "message": "Options cannot contain empty values",
                    "correct_format_example": {
                        "question": "How would you like to proceed?",
                        "options": ["Continue", "Cancel", "Save and exit"]
                    }
                });
                return Ok(serde_json::to_string(&response).unwrap());
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

        Ok(serde_json::to_string(&response).unwrap())
    }
}
