use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskUserOption {
    pub value: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<QueryOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeMessage {
    #[serde(rename = "start")]
    Start { session_id: String },
    #[serde(rename = "progress")]
    Progress { content: serde_json::Value },
    #[serde(rename = "tool_use")]
    ToolUse { tool: String, args: Value, tool_use_id: Option<String> },
    #[serde(rename = "tool_result")]
    ToolResult { tool: String, result: Value },
    #[serde(rename = "ask_user")]
    AskUser { 
        prompt_type: String, // "checkbox" | "buttons" | "input"
        title: String,
        options: Option<Vec<AskUserOption>>,
        input_type: Option<String>, // "text" | "password" for input fields
        placeholder: Option<String>,
        tool_use_id: Option<String>,
    },
    #[serde(rename = "result")]
    Result { result: String },
    #[serde(rename = "error")]
    Error { error: String },
}