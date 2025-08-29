use salvo::prelude::*;
use salvo::session::SessionDepotExt;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use crate::utils::AppState;

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

async fn get_client_config(pool: &PgPool, client_id: Uuid) -> Result<SystemConfig, sqlx::Error> {
    // Query the database for client-specific configuration from JSONB config column and domains column
    let row = sqlx::query!(
        r#"
        SELECT config, domains
        FROM clients 
        WHERE id = $1 AND deleted_at IS NULL
        "#,
        client_id
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            // Parse the JSONB config field for system settings
            let config_json = r.config;
            
            // Try to extract system config from the JSONB
            let registration_enabled = config_json.get("registration_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let require_invite_code = config_json.get("require_invite_code")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            
            // Get domains from the domains column (PostgreSQL array)
            let allowed_domains = r.domains.unwrap_or_default();
            
            Ok(SystemConfig {
                registration_enabled,
                require_invite_code,
                session_timeout: 86400,
                allowed_domains,
            })
        }
        None => Ok(SystemConfig::default()),
    }
}

async fn update_client_config(pool: &PgPool, client_id: Uuid, config: &SystemConfig) -> Result<(), sqlx::Error> {
    // First get existing config
    let row = sqlx::query!(
        r#"
        SELECT config
        FROM clients 
        WHERE id = $1 AND deleted_at IS NULL
        "#,
        client_id
    )
    .fetch_optional(pool)
    .await?;
    
    let mut config_json = if let Some(r) = row {
        r.config
    } else {
        serde_json::json!({})
    };
    
    // Update the config JSON with system settings (not domains - they're managed separately)
    config_json["registration_enabled"] = serde_json::json!(config.registration_enabled);
    config_json["require_invite_code"] = serde_json::json!(config.require_invite_code);
    
    // Update the config in database (domains are managed by the DomainManagement component)
    sqlx::query!(
        r#"
        UPDATE clients 
        SET config = $2,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $1
        "#,
        client_id,
        config_json
    )
    .execute(pool)
    .await?;
    
    Ok(())
}

#[handler]
pub async fn get_config(depot: &mut Depot, res: &mut Response) {
    // Get the client_id from session
    let client_id = if let Some(session) = depot.session_mut() {
        session.get::<String>("client_id")
    } else {
        None
    };

    if let Some(client_id_str) = client_id {
        if let Ok(client_uuid) = Uuid::parse_str(&client_id_str) {
            // Get the database pool
            if let Ok(state) = depot.obtain::<AppState>() {
                match get_client_config(&state.db_pool, client_uuid).await {
                    Ok(config) => {
                        res.render(Json(&config));
                        return;
                    }
                    Err(e) => {
                        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                        res.render(Json(serde_json::json!({
                            "error": format!("Failed to load configuration: {}", e)
                        })));
                        return;
                    }
                }
            }
        }
    }
    
    // Return default config if no client_id
    let config = SystemConfig::default();
    res.render(Json(&config));
}

#[handler]
pub async fn update_config(depot: &mut Depot, req: &mut Request, res: &mut Response) {
    // Get the client_id from session
    let client_id = if let Some(session) = depot.session_mut() {
        session.get::<String>("client_id")
    } else {
        None
    };

    match req.parse_json::<SystemConfig>().await {
        Ok(config) => {
            if let Some(client_id_str) = client_id {
                if let Ok(client_uuid) = Uuid::parse_str(&client_id_str) {
                    // Get the database pool
                    if let Ok(state) = depot.obtain::<AppState>() {
                        match update_client_config(&state.db_pool, client_uuid, &config).await {
                            Ok(_) => {
                                res.render(Json(serde_json::json!({
                                    "success": true,
                                    "message": "Configuration updated successfully"
                                })));
                                return;
                            }
                            Err(e) => {
                                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                                res.render(Json(serde_json::json!({
                                    "error": format!("Failed to update configuration: {}", e)
                                })));
                                return;
                            }
                        }
                    }
                }
            }
            
            // No client_id in session
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(serde_json::json!({
                "error": "No client selected"
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