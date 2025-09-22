use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectShare {
    pub id: String,
    pub project_id: String,
    pub share_token: String,
    pub share_type: ShareType,
    pub settings: ShareSettings,
    pub is_public: bool,
    pub is_read_only: bool,
    pub max_messages_per_session: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub view_count: i32,
    pub last_accessed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareType {
    NewChat,
    AllHistory,
    SpecificConversations,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareSettings {
    // UI customization
    pub theme: Option<String>,
    pub custom_css: Option<String>,
    pub show_branding: Option<bool>,
    
    // Feature toggles
    pub allow_file_upload: Option<bool>,
    pub show_conversation_list: Option<bool>,
    pub show_project_name: Option<bool>,
    pub enable_markdown: Option<bool>,
    
    // Layout settings
    pub layout_mode: Option<String>, // "full", "compact", "minimal"
    pub width: Option<String>,
    pub height: Option<String>,
    
    // Custom branding
    pub title: Option<String>,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    
    // Additional metadata
    pub metadata: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectShareConversation {
    pub id: String,
    pub project_share_id: String,
    pub conversation_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectShareSession {
    pub id: String,
    pub project_share_id: String,
    pub session_token: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub referrer: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub message_count: i32,
    pub max_messages: i32,
}

// Request/Response types for API

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateShareRequest {
    pub share_type: ShareType,
    pub settings: ShareSettings,
    pub is_read_only: Option<bool>,
    pub max_messages_per_session: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
    pub conversation_ids: Option<Vec<String>>, // For specific_conversations type
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateShareRequest {
    pub settings: Option<ShareSettings>,
    pub is_read_only: Option<bool>,
    pub max_messages_per_session: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
    pub conversation_ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShareResponse {
    pub share: ProjectShare,
    pub conversations: Option<Vec<ProjectShareConversation>>,
    pub embed_url: String,
    pub embed_codes: EmbedCodes,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbedCodes {
    pub iframe_simple: String,
    pub iframe_responsive: String,
    pub javascript_sdk: String,
    pub react_component: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SharedProjectData {
    pub share: ProjectShare,
    pub project: crate::models::project::Project,
    pub conversations: Vec<crate::models::conversation::Conversation>,
    pub session: Option<ProjectShareSession>,
}

// Default implementations

impl Default for ShareSettings {
    fn default() -> Self {
        ShareSettings {
            theme: Some("light".to_string()),
            custom_css: None,
            show_branding: Some(true),
            allow_file_upload: Some(false),
            show_conversation_list: Some(true),
            show_project_name: Some(true),
            enable_markdown: Some(true),
            layout_mode: Some("full".to_string()),
            width: None,
            height: None,
            title: None,
            description: None,
            logo_url: None,
            metadata: None,
        }
    }
}

// Utility functions

impl ProjectShare {
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp < Utc::now())
            .unwrap_or(false)
    }
    
    pub fn is_active(&self) -> bool {
        self.deleted_at.is_none() && !self.is_expired()
    }
    
    pub fn generate_embed_codes(&self, base_url: &str) -> EmbedCodes {
        let embed_url = format!("{}/embed/{}", base_url, self.share_token);
        
        EmbedCodes {
            iframe_simple: format!(
                r#"<iframe src="{}" width="400" height="600" frameborder="0"></iframe>"#,
                embed_url
            ),
            iframe_responsive: format!(
                r#"<div style="position: relative; padding-bottom: 75%; height: 0;">
  <iframe src="{}" style="position: absolute; top: 0; left: 0; width: 100%; height: 100%;" frameborder="0"></iframe>
</div>"#,
                embed_url
            ),
            javascript_sdk: format!(
                r#"<div id="clay-chat"></div>
<script src="{}/embed.js"></script>
<script>
  ClayStudio.embed({{
    token: '{}',
    container: '#clay-chat',
    theme: '{}',
    readOnly: {}
  }});
</script>"#,
                base_url,
                self.share_token,
                self.settings.theme.as_deref().unwrap_or("light"),
                self.is_read_only
            ),
            react_component: format!(
                r#"import {{ ClayChat }} from '@clay-studio/embed';

function App() {{
  return (
    <ClayChat 
      shareToken="{}"
      theme="{}"
      readOnly={{{}}}
    />
  );
}}"#,
                self.share_token,
                self.settings.theme.as_deref().unwrap_or("light"),
                self.is_read_only
            ),
        }
    }
}

impl ProjectShareSession {
    #[allow(dead_code)]
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }
    
    #[allow(dead_code)]
    pub fn can_send_message(&self) -> bool {
        self.message_count < self.max_messages && !self.is_expired()
    }
    
    #[allow(dead_code)]
    pub fn increment_message_count(&mut self) {
        self.message_count += 1;
        self.last_activity_at = Utc::now();
    }
}