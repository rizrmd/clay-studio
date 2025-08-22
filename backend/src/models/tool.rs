use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContext {
    pub name: String,
    pub category: String,
    pub description: String,
    pub parameters: Value,
    pub applicable: bool,
    pub usage_examples: Vec<String>,
}