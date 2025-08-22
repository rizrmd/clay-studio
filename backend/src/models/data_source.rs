use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceContext {
    pub id: String,
    pub name: String,
    pub source_type: String,
    pub connection_config: Value,
    pub schema_info: Option<Value>,
    pub preview_data: Option<Value>,
    pub table_list: Option<Vec<String>>,
    pub last_tested_at: Option<String>,
    pub is_active: bool,
}