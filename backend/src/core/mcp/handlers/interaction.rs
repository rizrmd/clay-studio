use super::base::McpHandlers;
use crate::core::mcp::types::*;
use serde_json::{json, Value};
use uuid;

impl McpHandlers {
    pub async fn handle_ask_user(
        &self,
        arguments: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        let question = arguments
            .get("question")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing question parameter".to_string(),
                data: None,
            })?;

        let options = arguments
            .get("options")
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

        serde_json::to_string(&interaction_spec).map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Failed to serialize interaction response: {}", e),
            data: None,
        })
    }
}
