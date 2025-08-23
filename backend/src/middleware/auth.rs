use salvo::prelude::*;
use salvo::session::SessionDepotExt;
use crate::error::AppError;

#[handler]
pub async fn auth_required(depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
    if let Some(session) = depot.session_mut() {
        let user_id: Option<String> = session.get("user_id");

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