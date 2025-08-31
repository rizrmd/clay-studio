use salvo::prelude::*;
use salvo::websocket::{WebSocket, Message as WsMessage, WebSocketUpgrade};
use salvo::session::SessionDepotExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use futures_util::{SinkExt, StreamExt};

use crate::utils::{AppState, AppError};
use async_session::SessionStore;

// WebSocket message types from client
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Subscribe { 
        project_id: String, 
        conversation_id: Option<String> 
    },
    Unsubscribe,
    Ping,
    AskUserResponse {
        conversation_id: String,
        interaction_id: String,
        response: serde_json::Value, // Can be string or array of strings
    },
    StopStreaming {
        conversation_id: String,
    },
}

// WebSocket message types to client
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    // Connection messages
    Connected { 
        user_id: String,
        authenticated: bool,
        client_id: Option<String>,
        role: Option<String>
    },
    AuthenticationRequired,
    Subscribed { 
        project_id: String, 
        conversation_id: Option<String> 
    },
    Pong,
    // Streaming messages
    Start { 
        id: String, 
        conversation_id: String 
    },
    Progress { 
        content: String,
        conversation_id: String 
    },
    ToolUse { 
        tool: String,
        conversation_id: String 
    },
    AskUser {
        prompt_type: String,
        title: String,
        options: Option<Vec<serde_json::Value>>,
        input_type: Option<String>,
        placeholder: Option<String>,
        tool_use_id: Option<String>,
        conversation_id: String,
    },
    Content { 
        content: String,
        conversation_id: String 
    },
    Complete { 
        id: String, 
        conversation_id: String, 
        processing_time_ms: u64, 
        tool_usages: Option<Vec<crate::models::tool_usage::ToolUsage>>
    },
    Error { 
        error: String,
        conversation_id: String 
    },
    TitleUpdated {
        conversation_id: String,
        title: String
    },
    ContextUsage {
        conversation_id: String,
        total_chars: usize,
        max_chars: usize,
        percentage: f32,
        message_count: usize,
        needs_compaction: bool,
    },
}

// User connection info
#[derive(Clone, Debug)]
pub struct UserConnection {
    pub user_id: String,
    pub sender: mpsc::UnboundedSender<ServerMessage>,
    pub project_id: Option<String>,
    pub conversation_id: Option<String>,
}


#[handler]
pub async fn handle_websocket(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap().clone();
    
    // Try to get session from query parameter first (for compatibility)
    // Note: req.query automatically URL-decodes the parameter
    let session_from_query: Option<String> = req.query("session");
    
    // Also try to get the raw query string for debugging
    if let Some(query_str) = req.uri().query() {
        tracing::debug!("WebSocket: Raw query string: {}", query_str);
    }
    
    // Extract session data for authentication
    let (user_id, client_id, role, is_authenticated) = if let Some(session_token) = session_from_query {
        // Fallback: Load session from session token in query parameter
        tracing::info!("WebSocket: Attempting to load session from query parameter");
        tracing::debug!("WebSocket: Session token (first 50 chars): {}", 
                       &session_token.chars().take(50).collect::<String>());
        
        // The session token is the cookie value, load it from the store
        match state.session_store.load_session(session_token.clone()).await {
            Ok(Some(session)) => {
                let user_id: Option<String> = session.get("user_id");
                let client_id: Option<String> = session.get("client_id");
                let role: Option<String> = session.get("role");
                
                tracing::info!("WebSocket session loaded from query: user_id={:?}, client_id={:?}, role={:?}", 
                               user_id, client_id, role);
                
                match user_id {
                    Some(uid) => (uid, client_id, role, true),
                    None => {
                        tracing::warn!("WebSocket: Session found but no user_id");
                        ("anonymous".to_string(), None, None, false)
                    }
                }
            }
            Ok(None) => {
                tracing::warn!("WebSocket: No session found for token (session store returned None)");
                ("anonymous".to_string(), None, None, false)
            }
            Err(e) => {
                tracing::error!("WebSocket: Failed to load session from query parameter: {}", e);
                tracing::error!("WebSocket: This usually means the session format is invalid or expired");
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
            
            tracing::info!("WebSocket session data from cookie: user_id={:?}, client_id={:?}, role={:?}", 
                           user_id, client_id, role);
            
            match user_id {
                Some(uid) => (uid, client_id, role, true),
                None => {
                    tracing::warn!("WebSocket: Cookie session found but no user_id");
                    ("anonymous".to_string(), None, None, false)
                }
            }
        } else {
            // Check if there's a cookie at all
            if let Some(cookie) = req.cookie("clay_session") {
                tracing::warn!("WebSocket: Cookie exists but depot.session() returned None");
                tracing::debug!("WebSocket: Cookie value (first 50 chars): {}", 
                               &cookie.value().chars().take(50).collect::<String>());
            } else {
                tracing::warn!("WebSocket: No session cookie found at all");
            }
            ("anonymous".to_string(), None, None, false)
        }
    };
    
    tracing::info!("WebSocket connection request: user_id={}, authenticated={}, client_id={:?}", 
                  user_id, is_authenticated, client_id);
    
    WebSocketUpgrade::new()
        .upgrade(req, res, move |websocket| {
            handle_websocket_connection(websocket, user_id, client_id, role, is_authenticated, state)
        })
        .await
        .map_err(|e| AppError::InternalServerError(format!("WebSocket upgrade failed: {}", e)))
}

async fn handle_websocket_connection(
    websocket: WebSocket,
    user_id: String,
    client_id: Option<String>,
    role: Option<String>,
    is_authenticated: bool,
    state: AppState,
) {
    let connection_id = Uuid::new_v4().to_string();
    let (mut ws_tx, mut ws_rx) = websocket.split();
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<ServerMessage>();
    
    tracing::info!("WebSocket connected: user_id={}, connection_id={}", user_id, connection_id);
    
    // Store connection in global manager only if authenticated
    if is_authenticated {
        let user_connection = UserConnection {
            user_id: user_id.clone(),
            sender: msg_tx.clone(),
            project_id: None,
            conversation_id: None,
        };
        
        {
            let mut connections = WS_CONNECTIONS.write().await;
            connections.insert(connection_id.clone(), user_connection);
            let user_connection_count = connections.values()
                .filter(|c| c.user_id == user_id)
                .count();
            tracing::info!("User {} now has {} active WebSocket connections", user_id, user_connection_count);
        }
    }
    
    // Send authentication status message
    if is_authenticated {
        let _ = msg_tx.send(ServerMessage::Connected { 
            user_id: user_id.clone(),
            authenticated: true,
            client_id: client_id.clone(),
            role: role.clone()
        });
        tracing::info!("WebSocket authenticated: user_id={}, client_id={:?}, role={:?}", 
                      user_id, client_id, role);
    } else {
        let _ = msg_tx.send(ServerMessage::AuthenticationRequired);
        tracing::warn!("WebSocket connection not authenticated: user_id={}", user_id);
    }
    
    // Spawn task to send messages to WebSocket
    let ws_sender = tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            let json_msg = match serde_json::to_string(&msg) {
                Ok(json) => json,
                Err(e) => {
                    tracing::error!("Failed to serialize WebSocket message: {}", e);
                    continue;
                }
            };
            
            if ws_tx.send(WsMessage::text(json_msg)).await.is_err() {
                tracing::info!("WebSocket connection closed, stopping sender");
                break;
            }
        }
    });
    
    // Handle incoming messages
    while let Some(msg_result) = ws_rx.next().await {
        match msg_result {
            Ok(msg) => {
                if let Ok(text) = msg.as_str() {
                    tracing::info!("WebSocket received message: {}", text);
            match serde_json::from_str::<ClientMessage>(text) {
                        Ok(client_msg) => {
                            handle_client_message(
                                client_msg, 
                                &user_id, 
                                &connection_id, 
                                &msg_tx, 
                                &state
                            ).await;
                        },
                        Err(e) => {
                            tracing::warn!("Failed to parse WebSocket message: {} - {}", text, e);
                        }
                    }
                } else if msg.is_close() {
                    tracing::info!("WebSocket close message received");
                    break;
                }
            },
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
        }
    }
    
    // Cleanup
    ws_sender.abort();
    tracing::info!("WebSocket disconnected: user_id={}, connection_id={}", user_id, connection_id);
    
    // Remove from connection manager
    {
        let mut connections = WS_CONNECTIONS.write().await;
        connections.remove(&connection_id);
        tracing::debug!("Removed WebSocket connection: connection_id={}, user_id={}", connection_id, user_id);
    }
}

async fn handle_client_message(
    msg: ClientMessage,
    user_id: &str,
    connection_id: &str,
    sender: &mpsc::UnboundedSender<ServerMessage>,
    state: &AppState,
) {
    match msg {
        ClientMessage::Subscribe { project_id, conversation_id } => {
            // Check if connection is authenticated before allowing subscription
            let is_authenticated = {
                let connections = WS_CONNECTIONS.read().await;
                connections.get(connection_id).is_some()
            };
            
            if !is_authenticated {
                let _ = sender.send(ServerMessage::AuthenticationRequired);
                tracing::warn!("Unauthenticated user {} tried to subscribe to project {}", 
                              user_id, project_id);
                return;
            }
            
            tracing::info!("User {} subscribing to project={}, conversation={:?}", 
                user_id, project_id, conversation_id);
            
            // Update connection's subscription in connection manager
            {
                let mut connections = WS_CONNECTIONS.write().await;
                if let Some(conn) = connections.get_mut(connection_id) {
                    conn.project_id = Some(project_id.clone());
                    conn.conversation_id = conversation_id.clone();
                }
            }
            
            // Check if there's an active stream for this conversation
            if let Some(ref conv_id) = conversation_id {
                let streams = state.active_claude_streams.read().await;
                if let Some(stream_state) = streams.get(conv_id) {
                    tracing::info!("Found active stream for conversation {}, sending current state", conv_id);
                    
                    // Send current streaming state
                    let _ = sender.send(ServerMessage::Start { 
                        id: stream_state.message_id.clone(),
                        conversation_id: conv_id.clone(),
                    });
                    
                    // Send any active tools
                    for tool in &stream_state.active_tools {
                        let _ = sender.send(ServerMessage::ToolUse { 
                            tool: tool.clone(),
                            conversation_id: conv_id.clone()
                        });
                    }
                    
                    // Send partial content if any
                    if !stream_state.partial_content.is_empty() {
                        let _ = sender.send(ServerMessage::Content { 
                            content: stream_state.partial_content.clone(),
                            conversation_id: conv_id.clone()
                        });
                    }
                }
            }
            
            let _ = sender.send(ServerMessage::Subscribed { 
                project_id, 
                conversation_id 
            });
        },
        
        ClientMessage::Unsubscribe => {
            tracing::info!("Connection {} (user {}) unsubscribing", connection_id, user_id);
            
            // Clear subscription in connection manager
            {
                let mut connections = WS_CONNECTIONS.write().await;
                if let Some(conn) = connections.get_mut(connection_id) {
                    conn.project_id = None;
                    conn.conversation_id = None;
                }
            }
        },
        
        ClientMessage::Ping => {
            let _ = sender.send(ServerMessage::Pong);
        },
        
        ClientMessage::AskUserResponse { conversation_id, interaction_id, response } => {
            tracing::info!(
                "Received ask_user response: conversation={}, interaction={}, response={:?}", 
                conversation_id, interaction_id, response
            );
            
            // Store the response in the database
            if let Err(e) = store_ask_user_response(
                &state,
                &conversation_id,
                &interaction_id,
                &response
            ).await {
                tracing::error!("Failed to store ask_user response: {}", e);
            }
        },
        
        ClientMessage::StopStreaming { conversation_id } => {
            tracing::info!(
                "Received stop streaming request: conversation={}", 
                conversation_id
            );
            
            // Remove the streaming state for this conversation
            {
                let mut streams = state.active_claude_streams.write().await;
                if streams.remove(&conversation_id).is_some() {
                    tracing::info!("Stopped streaming for conversation: {}", conversation_id);
                }
            }
        },
    }
}

// Store ask_user response in the database
async fn store_ask_user_response(
    state: &AppState,
    conversation_id: &str,
    interaction_id: &str,
    response: &serde_json::Value,
) -> Result<(), AppError> {
    // For now, store in a simple JSON column in messages table
    // In production, you might want a dedicated interaction_responses table
    
    let response_json = serde_json::to_string(response)
        .map_err(|e| AppError::InternalServerError(format!("Failed to serialize response: {}", e)))?;
    
    // Store as a system message with the interaction response
    let message_content = format!(
        "User response to interaction {}:\n{}",
        interaction_id,
        response_json
    );
    
    sqlx::query!(
        r#"
        INSERT INTO messages (id, conversation_id, role, content, created_at)
        VALUES ($1, $2, 'system', $3, NOW())
        "#,
        uuid::Uuid::new_v4().to_string(),
        conversation_id,
        message_content
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    Ok(())
}

// Global storage for active WebSocket connections (keyed by connection_id, not user_id)
lazy_static::lazy_static! {
    pub static ref WS_CONNECTIONS: Arc<RwLock<HashMap<String, UserConnection>>> = Arc::new(RwLock::new(HashMap::new()));
}


// Helper to broadcast to subscribed users
pub async fn broadcast_to_subscribers(
    project_id: &str,
    conversation_id: &str,
    message: ServerMessage,
) {
    let connections = WS_CONNECTIONS.read().await;
    let mut _sent_count = 0;
    
    for (connection_id, conn) in connections.iter() {
        // Check if connection is subscribed to this project/conversation
        if let (Some(user_project), Some(user_conversation)) = (&conn.project_id, &conn.conversation_id) {
            if user_project == project_id && user_conversation == conversation_id {
                if conn.sender.send(message.clone()).is_ok() {
                    _sent_count += 1;
                } else {
                    tracing::warn!("Failed to send message to connection {} (user {})", connection_id, conn.user_id);
                }
            }
        } else if let Some(user_project) = &conn.project_id {
            // Connection is subscribed to project but not specific conversation - still send
            if user_project == project_id {
                if conn.sender.send(message.clone()).is_ok() {
                    _sent_count += 1;
                }
            }
        }
    }
    
}