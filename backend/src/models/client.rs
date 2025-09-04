use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "installing")]
    Installing,
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "suspended")]
    Suspended,
    #[serde(rename = "error")]
    Error,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: ClientStatus,
    #[serde(rename = "installPath")]
    pub install_path: String,
    pub domains: Option<Vec<String>>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCreateRequest {
    pub name: String,
    pub description: Option<String>,
    pub domains: Option<Vec<String>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientUpdateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub domains: Option<Vec<String>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientAdminResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: ClientStatus,
    #[serde(rename = "installPath")]
    pub install_path: String,
    pub domains: Option<Vec<String>>,
    #[serde(rename = "userCount")]
    pub user_count: i64,
    #[serde(rename = "projectCount")]
    pub project_count: i64,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRootResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: ClientStatus,
    #[serde(rename = "installPath")]
    pub install_path: String,
    pub domains: Option<Vec<String>>,
    pub config: serde_json::Value,
    #[serde(rename = "hasClaudeToken")]
    pub has_claude_token: bool,
    #[serde(rename = "userCount")]
    pub user_count: i64,
    #[serde(rename = "projectCount")]
    pub project_count: i64,
    #[serde(rename = "conversationCount")]
    pub conversation_count: i64,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
    #[serde(rename = "deletedAt")]
    pub deleted_at: Option<DateTime<Utc>>,
}
