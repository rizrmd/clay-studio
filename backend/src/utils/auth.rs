use salvo::prelude::*;
use salvo::session::SessionDepotExt;
use crate::utils::{AppError, AppState, domain};
use uuid::Uuid;

#[handler]
pub async fn auth_required(req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
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

        // Validate domain if client_id is in session
        if let Some(client_id_str) = client_id_str {
            if let Ok(client_id) = Uuid::parse_str(&client_id_str) {
                // Get the app state to access the database pool
                if let Ok(state) = depot.obtain::<AppState>() {
                    // Validate that the client can be accessed from this domain
                    if let Err(e) = domain::validate_client_domain(&state.db_pool, client_id, req).await {
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
        }

        // Store user_id in depot for handlers to use
        if let Some(user_id) = user_id {
            depot.insert("current_user_id", user_id);
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
                    depot.insert("current_user_role", r);
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