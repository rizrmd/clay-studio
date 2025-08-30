use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsage {
    pub id: Uuid,
    pub message_id: String,
    pub tool_name: String,
    pub tool_use_id: Option<String>,
    pub parameters: Option<Value>,
    pub output: Option<Value>,
    pub execution_time_ms: Option<i64>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
}

impl ToolUsage {
    #[allow(dead_code)]
    pub fn with_output(mut self, output: Value) -> Self {
        self.output = Some(output);
        self
    }

    #[allow(dead_code)]
    pub fn with_execution_time(mut self, execution_time_ms: i64) -> Self {
        self.execution_time_ms = Some(execution_time_ms);
        self
    }
}

// For tracking tool usages during streaming
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PendingToolUsage {
    pub tool_name: String,
    pub parameters: Option<Value>,
    pub start_time: std::time::Instant,
}