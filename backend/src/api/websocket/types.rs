use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

// WebSocket message types from client
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Subscribe {
        project_id: String,
        conversation_id: Option<String>,
    },
    Unsubscribe,
    Ping,
    AskUserResponse {
        conversation_id: String,
        interaction_id: String,
        response: serde_json::Value, // Can be string or array of strings
    },
    StopStreaming {
        conversation_id: String,
    },
    SendMessage {
        project_id: String,
        conversation_id: String,
        content: String,
        uploaded_file_paths: Option<Vec<String>>,
    },
    // Conversation management
    CreateConversation {
        project_id: String,
        title: Option<String>,
    },
    ListConversations {
        project_id: String,
    },
    GetConversation {
        conversation_id: String,
    },
    UpdateConversation {
        conversation_id: String,
        title: Option<String>,
    },
    DeleteConversation {
        conversation_id: String,
    },
    BulkDeleteConversations {
        conversation_ids: Vec<String>,
    },
    GetConversationMessages {
        conversation_id: String,
    },
}

// WebSocket message types to client
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    // Connection messages
    Connected {
        user_id: String,
        authenticated: bool,
        client_id: Option<String>,
        role: Option<String>,
    },
    AuthenticationRequired,
    Subscribed {
        project_id: String,
        conversation_id: Option<String>,
    },
    ConversationRedirect {
        old_conversation_id: String,
        new_conversation_id: String,
    },
    Pong,
    // Streaming messages
    Start {
        id: String,
        conversation_id: String,
    },
    Progress {
        content: serde_json::Value,
        conversation_id: String,
    },
    ToolUse {
        tool: String,
        tool_usage_id: String,
        conversation_id: String,
    },
    ToolComplete {
        tool: String,
        tool_usage_id: String,
        execution_time_ms: i64,
        output: Option<serde_json::Value>,
        conversation_id: String,
    },
    #[allow(dead_code)]
    AskUser {
        prompt_type: String,
        title: String,
        options: Option<Vec<serde_json::Value>>,
        input_type: Option<String>,
        placeholder: Option<String>,
        tool_use_id: Option<String>,
        conversation_id: String,
    },
    Content {
        content: String,
        conversation_id: String,
    },
    Complete {
        id: String,
        conversation_id: String,
        processing_time_ms: u64,
        tool_usages: Option<Vec<crate::models::tool_usage::ToolUsage>>,
    },
    Error {
        error: String,
        conversation_id: String,
    },
    ConversationActivity {
        conversation_id: String,
        user_id: String,
        user_name: String,
        activity_type: String,
        timestamp: String,
        message_preview: Option<String>,
    },
    // Conversation management responses
    ConversationList {
        conversations: Vec<crate::models::Conversation>,
    },
    ConversationCreated {
        conversation: crate::models::Conversation,
    },
    ConversationDetails {
        conversation: crate::models::Conversation,
    },
    ConversationUpdated {
        conversation: crate::models::Conversation,
    },
    ConversationDeleted {
        conversation_id: String,
    },
    ConversationsBulkDeleted {
        conversation_ids: Vec<String>,
        failed_ids: Vec<String>,
    },
    ConversationMessages {
        conversation_id: String,
        messages: Vec<crate::models::Message>,
    },
}

// User connection info
#[derive(Clone, Debug)]
pub struct UserConnection {
    pub user_id: String,
    pub sender: mpsc::UnboundedSender<ServerMessage>,
    pub project_id: Option<String>,
    pub conversation_id: Option<String>,
}