use crate::models::{DataSourceContext, Message, ProjectSettings, ToolContext};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub conversation_id: String,
    pub project_id: String,
    pub messages: Vec<Message>,
    pub summary: Option<ConversationSummary>,
    pub data_sources: Vec<DataSourceContext>,
    pub available_tools: Vec<ToolContext>,
    pub project_settings: ProjectSettings,
    pub total_messages: i32,
    pub context_strategy: ContextStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: String,
    pub summary_text: String,
    pub message_count: i32,
    pub summary_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ContextStrategy {
    FullHistory,
    SummaryWithRecent,
    OnlyRecent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub project_id: String,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_title_manually_set: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by_user_id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<ConversationVisibility>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConversationVisibility {
    Private,
    Public,
}

impl ConversationVisibility {
    pub fn as_str(&self) -> &str {
        match self {
            ConversationVisibility::Private => "private",
            ConversationVisibility::Public => "public",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "private" => Ok(ConversationVisibility::Private),
            "public" => Ok(ConversationVisibility::Public),
            _ => Err(format!("Invalid visibility: {}", s)),
        }
    }
}
