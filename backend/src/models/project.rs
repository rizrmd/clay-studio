use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::models::{DataSourceContext, ToolContext};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub user_id: String,
    pub client_id: Option<String>,
    pub name: String,
    pub settings: Option<Value>,
    pub organization_settings: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub project_id: String,
    pub user_id: String,
    pub client_id: Option<String>,
    pub name: String,
    pub settings: Value,
    pub organization_settings: Value,
    pub default_analysis_preferences: AnalysisPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisPreferences {
    pub auto_suggest_visualizations: bool,
    pub preferred_chart_types: Vec<String>,
    pub default_aggregation_functions: Vec<String>,
    pub enable_statistical_insights: bool,
    pub context_length_preference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContextResponse {
    pub project_id: String,
    pub project_settings: ProjectSettings,
    pub data_sources: Vec<DataSourceContext>,
    pub available_tools: Vec<ToolContext>,
    pub total_conversations: i32,
    pub recent_activity: Vec<RecentActivity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentActivity {
    pub activity_type: String,
    pub description: String,
    pub timestamp: String,
    pub conversation_id: Option<String>,
}

impl Default for AnalysisPreferences {
    fn default() -> Self {
        AnalysisPreferences {
            auto_suggest_visualizations: true,
            preferred_chart_types: vec!["line".to_string(), "bar".to_string()],
            default_aggregation_functions: vec!["sum".to_string(), "avg".to_string()],
            enable_statistical_insights: true,
            context_length_preference: "medium".to_string(),
        }
    }
}