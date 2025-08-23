use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    #[serde(default)]
    pub registration_enabled: bool,
    #[serde(default = "default_max_users")]
    pub max_users: i32,
    #[serde(default)]
    pub require_invite_code: bool,
    pub invite_code: Option<String>,
    #[serde(default)]
    pub features: ClientFeatures,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientFeatures {
    #[serde(default = "default_true")]
    pub chat_enabled: bool,
    #[serde(default = "default_true")]
    pub projects_enabled: bool,
    #[serde(default = "default_true")]
    pub data_sources_enabled: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            registration_enabled: false, // Default to disabled
            max_users: 10,
            require_invite_code: false,
            invite_code: None,
            features: ClientFeatures::default(),
        }
    }
}

fn default_max_users() -> i32 {
    10
}

fn default_true() -> bool {
    true
}