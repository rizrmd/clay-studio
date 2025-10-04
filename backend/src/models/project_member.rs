use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMember {
    pub id: Uuid,
    pub project_id: String,
    pub user_id: Uuid,
    pub role: ProjectMemberRole,
    pub joined_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectMemberRole {
    Owner,
    Member,
}

impl ProjectMemberRole {
    pub fn as_str(&self) -> &str {
        match self {
            ProjectMemberRole::Owner => "owner",
            ProjectMemberRole::Member => "member",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "owner" => Ok(ProjectMemberRole::Owner),
            "member" => Ok(ProjectMemberRole::Member),
            _ => Err(format!("Invalid role: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMemberWithUser {
    pub id: Uuid,
    pub project_id: String,
    pub user_id: Uuid,
    pub username: String,
    pub role: ProjectMemberRole,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AddProjectMemberRequest {
    pub user_id: Uuid,
    #[serde(default = "default_member_role")]
    pub role: ProjectMemberRole,
}

fn default_member_role() -> ProjectMemberRole {
    ProjectMemberRole::Member
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectMemberRequest {
    pub role: ProjectMemberRole,
}

#[derive(Debug, Deserialize)]
pub struct TransferOwnershipRequest {
    pub new_owner_user_id: Uuid,
}