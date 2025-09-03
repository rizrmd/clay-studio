use super::base::McpHandlers;
use crate::core::mcp::types::*;
use serde_json::{json, Value};
use uuid;

impl McpHandlers {
    pub async fn handle_ask_user(
        &self,
        arguments: &serde_json::Map<String, Value>
    ) -> Result<String, JsonRpcError> {
        let question = arguments.get("question")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing question parameter".to_string(),
                data: None,
            })?;

        let options = arguments.get("options")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>());

        // Create interaction response
        let interaction_id = uuid::Uuid::new_v4().to_string();
        let interaction_spec = json!({
            "interaction_id": interaction_id,
            "interaction_type": "ask_user",
            "question": question,
            "options": options,
            "status": "pending",
            "requires_response": true,
            "created_at": chrono::Utc::now().to_rfc3339(),
        });

        // Format the response with the question and interaction spec
        let response_text = if let Some(opts) = &options {
            if opts.is_empty() {
                format!(
                    "❓ **Question for User**\n\n{}\n\n```json\n{}\n```\n\n💭 Please provide your answer.",
                    question,
                    serde_json::to_string_pretty(&interaction_spec).unwrap_or_default()
                )
            } else {
                let options_list = opts.iter()
                    .enumerate()
                    .map(|(i, opt)| format!("{}. {}", i + 1, opt))
                    .collect::<Vec<_>>()
                    .join("\n");
                
                format!(
                    "❓ **Question for User**\n\n{}\n\n**Options:**\n{}\n\n```json\n{}\n```\n\n💭 Please select an option or provide your answer.",
                    question,
                    options_list,
                    serde_json::to_string_pretty(&interaction_spec).unwrap_or_default()
                )
            }
        } else {
            format!(
                "❓ **Question for User**\n\n{}\n\n```json\n{}\n```\n\n💭 Please provide your answer.",
                question,
                serde_json::to_string_pretty(&interaction_spec).unwrap_or_default()
            )
        };

        Ok(response_text)
    }
}