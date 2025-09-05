use crate::utils::{domain, AppError, AppState};
use salvo::prelude::*;
use salvo::session::SessionDepotExt;
use uuid::Uuid;

#[handler]
pub async fn auth_required(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    if let Some(session) = depot.session_mut() {
        let user_id: Option<String> = session.get("user_id");
        let client_id_str: Option<String> = session.get("client_id");

        if user_id.is_none() {
            let error = AppError::Unauthorized("Authentication required".to_string());
            res.render(Json(serde_json::json!({
                "error": error.to_string(),
                "code": 401
            })));
            res.status_code(StatusCode::UNAUTHORIZED);
            ctrl.skip_rest();
            return;
        }

        // Check user role first
        let role: Option<String> = session.get("role");

        // Validate domain if client_id is in session (skip for root users)
        if let Some(client_id_str) = client_id_str {
            if let Ok(client_id) = Uuid::parse_str(&client_id_str) {
                // Skip domain validation for root users
                if role.as_deref() != Some("root") {
                    // Get the app state to access the database pool
                    if let Ok(state) = depot.obtain::<AppState>() {
                        // Validate that the client can be accessed from this domain (skip for root)
                        if let Err(e) =
                            domain::validate_client_domain(&state.db_pool, client_id, req).await
                        {
                            res.render(Json(serde_json::json!({
                                "error": e.to_string(),
                                "code": 403
                            })));
                            res.status_code(StatusCode::FORBIDDEN);
                            ctrl.skip_rest();
                            return;
                        }
                    }
                }

                // Store client_id in depot for handlers to use
                depot.insert("current_client_id", client_id_str);
            }
        }

        // Store user_id in depot for handlers to use
        if let Some(user_id) = user_id {
            depot.insert("current_user_id", user_id);
        }

        // Store role in depot for handlers to use
        if let Some(role) = role {
            depot.insert("current_user_role", role);
        }
    } else {
        let error = AppError::Unauthorized("No session found".to_string());
        res.render(Json(serde_json::json!({
            "error": error.to_string(),
            "code": 401
        })));
        res.status_code(StatusCode::UNAUTHORIZED);
        ctrl.skip_rest();
    }
}

#[handler]
pub async fn auth_optional(depot: &mut Depot, _res: &mut Response, _ctrl: &mut FlowCtrl) {
    if let Some(session) = depot.session_mut() {
        let user_id: Option<String> = session.get("user_id");

        // Store user_id in depot if present
        if let Some(user_id) = user_id {
            depot.insert("current_user_id", user_id);
        }
    }
}

#[handler]
pub async fn admin_required(depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
    if let Some(session) = depot.session_mut() {
        let user_id: Option<String> = session.get("user_id");
        let role: Option<String> = session.get("role");
        let client_id: Option<String> = session.get("client_id");

        if user_id.is_none() {
            let error = AppError::Unauthorized("Authentication required".to_string());
            res.render(Json(serde_json::json!({
                "error": error.to_string(),
                "code": 401
            })));
            res.status_code(StatusCode::UNAUTHORIZED);
            ctrl.skip_rest();
            return;
        }

        // Check if user has admin or root role
        match role {
            Some(r) if r == "admin" || r == "root" => {
                // Store user_id and role in depot for handlers to use
                if let Some(user_id) = user_id {
                    depot.insert("current_user_id", user_id);
                    depot.insert("current_user_role", r.clone());

                    // Also store client_id for admin users (needed for user management)
                    if let Some(client_id) = client_id {
                        depot.insert("current_user_client_id", client_id);
                    }
                }
            }
            _ => {
                let error = AppError::Forbidden("Admin access required".to_string());
                res.render(Json(serde_json::json!({
                    "error": error.to_string(),
                    "code": 403
                })));
                res.status_code(StatusCode::FORBIDDEN);
                ctrl.skip_rest();
            }
        }
    } else {
        let error = AppError::Unauthorized("No session found".to_string());
        res.render(Json(serde_json::json!({
            "error": error.to_string(),
            "code": 401
        })));
        res.status_code(StatusCode::UNAUTHORIZED);
        ctrl.skip_rest();
    }
}

#[handler]
pub async fn root_required(depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
    if let Some(session) = depot.session_mut() {
        let user_id: Option<String> = session.get("user_id");
        let role: Option<String> = session.get("role");

        if user_id.is_none() {
            let error = AppError::Unauthorized("Authentication required".to_string());
            res.render(Json(serde_json::json!({
                "error": error.to_string(),
                "code": 401
            })));
            res.status_code(StatusCode::UNAUTHORIZED);
            ctrl.skip_rest();
            return;
        }

        // Check if user has root role
        match role {
            Some(r) if r == "root" => {
                // Store user_id and role in depot for handlers to use
                if let Some(user_id) = user_id {
                    depot.insert("current_user_id", user_id);
                    depot.insert("current_user_role", r);
                }
            }
            _ => {
                let error = AppError::Forbidden("Root access required".to_string());
                res.render(Json(serde_json::json!({
                    "error": error.to_string(),
                    "code": 403
                })));
                res.status_code(StatusCode::FORBIDDEN);
                ctrl.skip_rest();
            }
        }
    } else {
        let error = AppError::Unauthorized("No session found".to_string());
        res.render(Json(serde_json::json!({
            "error": error.to_string(),
            "code": 401
        })));
        res.status_code(StatusCode::UNAUTHORIZED);
        ctrl.skip_rest();
    }
}

/// Middleware that ensures all operations are scoped to the current user's client
/// This should be applied to most protected endpoints to prevent cross-client data access
#[handler]
pub async fn client_scoped(depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
    // Get current user's client_id from depot (set by auth_required middleware)
    let client_id = match depot.get::<String>("current_client_id") {
        Ok(client_id) => client_id,
        Err(_) => {
            let error = AppError::Unauthorized("Client context required".to_string());
            res.render(Json(serde_json::json!({
                "error": error.to_string(),
                "code": 401
            })));
            res.status_code(StatusCode::UNAUTHORIZED);
            ctrl.skip_rest();
            return;
        }
    };

    // Check if user has root role - root users can access any client
    let role = depot.get::<String>("current_user_role").ok();
    if role.as_ref().map(|s| s.as_str()) == Some("root") {
        // Root users bypass client scoping
        return;
    }

    // Validate client_id from session exists and is accessible via domain filtering
    // (this was already done in auth_required, but we ensure it's present)
    if client_id.trim().is_empty() {
        let error = AppError::Forbidden("Invalid client context".to_string());
        res.render(Json(serde_json::json!({
            "error": error.to_string(),
            "code": 403
        })));
        res.status_code(StatusCode::FORBIDDEN);
        ctrl.skip_rest();
        return;
    }

    // Client ID is valid and available in depot for handlers to use
}

/// Helper function to get the current user's client_id from depot
/// This should be used in handlers that need to filter by client_id
pub fn get_current_client_id(depot: &Depot) -> Result<Uuid, AppError> {
    let client_id_str = depot
        .get::<String>("current_client_id")
        .map_err(|_| AppError::Unauthorized("Client context not found".to_string()))?;

    Uuid::parse_str(client_id_str)
        .map_err(|_| AppError::InternalServerError("Invalid client ID format".to_string()))
}

/// Helper function to get the current user's role from depot
pub fn get_current_user_role(depot: &Depot) -> Option<String> {
    depot.get::<String>("current_user_role").ok().cloned()
}

/// Helper function to check if current user is root (bypasses client filtering)
pub fn is_current_user_root(depot: &Depot) -> bool {
    get_current_user_role(depot).as_deref() == Some("root")
}
