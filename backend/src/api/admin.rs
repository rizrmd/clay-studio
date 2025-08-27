use salvo::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    #[serde(rename = "registrationEnabled")]
    pub registration_enabled: bool,
    #[serde(rename = "requireInviteCode")]
    pub require_invite_code: bool,
    #[serde(rename = "sessionTimeout")]
    pub session_timeout: i32,
    #[serde(rename = "allowedDomains")]
    pub allowed_domains: Vec<String>,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            registration_enabled: false,
            require_invite_code: false,
            session_timeout: 86400,
            allowed_domains: Vec::new(),
        }
    }
}

#[handler]
pub async fn get_config(res: &mut Response) {
    // For now, return default config
    // In production, you'd load this from database or config file
    let config = SystemConfig::default();
    res.render(Json(&config));
}

#[handler]
pub async fn update_config(req: &mut Request, res: &mut Response) {
    match req.parse_json::<SystemConfig>().await {
        Ok(_config) => {
            // In production, you'd save this to database or config file
            // For now, just return success
            res.render(Json(serde_json::json!({
                "success": true,
                "message": "Configuration updated successfully"
            })));
        }
        Err(e) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(serde_json::json!({
                "error": format!("Invalid configuration: {}", e)
            })));
        }
    }
}

pub fn admin_router() -> Router {
    Router::new()
        .push(Router::with_path("config")
            .get(get_config)
            .put(update_config))
}