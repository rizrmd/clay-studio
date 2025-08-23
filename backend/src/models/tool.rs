use serde::{Deserialize, Serialize};
use serde_json::Value;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
    pub usage_examples: Option<Value>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContext {
    pub name: String,
    pub category: String,
    pub description: String,
    pub parameters: Value,
    pub applicable: bool,
    pub usage_examples: Vec<String>,
}