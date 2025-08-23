use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "installing")]
    Installing,
    #[serde(rename = "active")]
    Active,
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
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientUpdateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}