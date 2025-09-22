use salvo::prelude::*;
use salvo::session::SessionDepotExt;
use async_session::SessionStore;
use crate::utils::AppState;

/// Extract session data for WebSocket authentication
/// Returns (user_id, client_id, role, is_authenticated)
pub async fn extract_session_data(
    req: &Request,
    depot: &Depot,
    state: &AppState,
) -> (String, Option<String>, Option<String>, bool) {
    // Try to get session from query parameter first (for compatibility)
    // Note: req.query automatically URL-decodes the parameter
    let session_from_query: Option<String> = req.query("session");

    // Also try to get the raw query string for debugging
    if let Some(query_str) = req.uri().query() {
        tracing::debug!("WebSocket: Raw query string: {}", query_str);
    }

    if let Some(session_token) = session_from_query {
        // Fallback: Load session from session token in query parameter
        tracing::info!("WebSocket: Attempting to load session from query parameter");
        tracing::debug!(
            "WebSocket: Session token (first 50 chars): {}",
            &session_token.chars().take(50).collect::<String>()
        );

        // The session token is the cookie value, load it from the store
        match state
            .session_store
            .load_session(session_token.clone())
            .await
        {
            Ok(Some(session)) => {
                let user_id: Option<String> = session.get("user_id");
                let client_id: Option<String> = session.get("client_id");
                let role: Option<String> = session.get("role");

                tracing::info!(
                    "WebSocket session loaded from query: user_id={:?}, client_id={:?}, role={:?}",
                    user_id,
                    client_id,
                    role
                );

                match user_id {
                    Some(uid) => (uid, client_id, role, true),
                    None => {
                        tracing::warn!("WebSocket: Session found but no user_id");
                        ("anonymous".to_string(), None, None, false)
                    }
                }
            }
            Ok(None) => {
                tracing::warn!(
                    "WebSocket: No session found for token (session store returned None)"
                );
                ("anonymous".to_string(), None, None, false)
            }
            Err(e) => {
                tracing::error!(
                    "WebSocket: Failed to load session from query parameter: {}",
                    e
                );
                tracing::error!(
                    "WebSocket: This usually means the session format is invalid or expired"
                );
                ("anonymous".to_string(), None, None, false)
            }
        }
    } else {
        // Try standard cookie-based session
        tracing::info!("WebSocket: No query parameter, checking cookie-based session");

        if let Some(session) = depot.session() {
            let user_id: Option<String> = session.get("user_id");
            let client_id: Option<String> = session.get("client_id");
            let role: Option<String> = session.get("role");

            tracing::info!(
                "WebSocket session data from cookie: user_id={:?}, client_id={:?}, role={:?}",
                user_id,
                client_id,
                role
            );

            match user_id {
                Some(uid) => (uid, client_id, role, true),
                None => {
                    tracing::warn!("WebSocket: Cookie session found but no user_id");
                    ("anonymous".to_string(), None, None, false)
                }
            }
        } else {
            // Fallback: Try to manually load session from cookie if depot.session() fails
            // This can happen during WebSocket upgrades where session middleware might not work properly
            if let Some(cookie) = req.cookie("clay_session") {
                tracing::warn!("WebSocket: Cookie exists but depot.session() returned None, attempting manual load");
                let cookie_value = cookie.value().to_string();
                tracing::info!("WebSocket: Full cookie value: {}", cookie_value);
                tracing::debug!("WebSocket: Cookie value length: {}", cookie_value.len());

                // Try to extract session ID from cookie for debugging
                if let Ok(session_id) = async_session::Session::id_from_cookie_value(&cookie_value)
                {
                    tracing::info!("WebSocket: Extracted session ID: {}", session_id);
                } else {
                    tracing::error!("WebSocket: Failed to extract session ID from cookie value");
                }

                // Try to load the session directly from the store
                // The cookie value needs to be passed as-is to load_session, which will extract the session ID
                match state.session_store.load_session(cookie_value.clone()).await {
                    Ok(Some(session)) => {
                        let user_id: Option<String> = session.get("user_id");
                        let client_id: Option<String> = session.get("client_id");
                        let role: Option<String> = session.get("role");

                        tracing::info!("WebSocket: Manually loaded session from cookie: user_id={:?}, client_id={:?}, role={:?}", 
                                       user_id, client_id, role);

                        match user_id {
                            Some(uid) => (uid, client_id, role, true),
                            None => {
                                tracing::warn!("WebSocket: Manually loaded session but no user_id");
                                ("anonymous".to_string(), None, None, false)
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::warn!("WebSocket: Manual session load returned None (cookie might be expired)");
                        ("anonymous".to_string(), None, None, false)
                    }
                    Err(e) => {
                        tracing::error!(
                            "WebSocket: Failed to manually load session from cookie: {}",
                            e
                        );
                        ("anonymous".to_string(), None, None, false)
                    }
                }
            } else {
                tracing::warn!("WebSocket: No session cookie found at all");
                ("anonymous".to_string(), None, None, false)
            }
        }
    }
}