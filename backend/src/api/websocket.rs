use futures_util::{SinkExt, StreamExt};
use salvo::prelude::*;
use salvo::session::SessionDepotExt;
use salvo::websocket::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::utils::{get_app_state, AppError, AppState};
use async_session::SessionStore;

// WebSocket message types from client
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Subscribe {
        project_id: String,
        conversation_id: Option<String>,
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
    SendMessage {
        project_id: String,
        conversation_id: String,
        content: String,
        uploaded_file_paths: Option<Vec<String>>,
    },
    // Conversation management
    CreateConversation {
        project_id: String,
        title: Option<String>,
    },
    ListConversations {
        project_id: String,
    },
    GetConversation {
        conversation_id: String,
    },
    UpdateConversation {
        conversation_id: String,
        title: Option<String>,
    },
    DeleteConversation {
        conversation_id: String,
    },
    BulkDeleteConversations {
        conversation_ids: Vec<String>,
    },
    GetConversationMessages {
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
        role: Option<String>,
    },
    AuthenticationRequired,
    Subscribed {
        project_id: String,
        conversation_id: Option<String>,
    },
    ConversationRedirect {
        old_conversation_id: String,
        new_conversation_id: String,
    },
    Pong,
    // Streaming messages
    Start {
        id: String,
        conversation_id: String,
    },
    Progress {
        content: serde_json::Value,
        conversation_id: String,
    },
    ToolUse {
        tool: String,
        tool_usage_id: String,
        conversation_id: String,
    },
    ToolComplete {
        tool: String,
        tool_usage_id: String,
        execution_time_ms: i64,
        output: Option<serde_json::Value>,
        conversation_id: String,
    },
    #[allow(dead_code)]
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
        conversation_id: String,
    },
    Complete {
        id: String,
        conversation_id: String,
        processing_time_ms: u64,
        tool_usages: Option<Vec<crate::models::tool_usage::ToolUsage>>,
    },
    Error {
        error: String,
        conversation_id: String,
    },
    ConversationActivity {
        conversation_id: String,
        user_id: String,
        user_name: String,
        activity_type: String,
        timestamp: String,
        message_preview: Option<String>,
    },
    // Conversation management responses
    ConversationList {
        conversations: Vec<crate::models::Conversation>,
    },
    ConversationCreated {
        conversation: crate::models::Conversation,
    },
    ConversationDetails {
        conversation: crate::models::Conversation,
    },
    ConversationUpdated {
        conversation: crate::models::Conversation,
    },
    ConversationDeleted {
        conversation_id: String,
    },
    ConversationsBulkDeleted {
        conversation_ids: Vec<String>,
        failed_ids: Vec<String>,
    },
    ConversationMessages {
        conversation_id: String,
        messages: Vec<crate::models::Message>,
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
    let state = get_app_state(depot)?.clone();

    // Try to get session from query parameter first (for compatibility)
    // Note: req.query automatically URL-decodes the parameter
    let session_from_query: Option<String> = req.query("session");

    // Also try to get the raw query string for debugging
    if let Some(query_str) = req.uri().query() {
        tracing::debug!("WebSocket: Raw query string: {}", query_str);
    }

    // Extract session data for authentication
    let (user_id, client_id, role, is_authenticated) = if let Some(session_token) =
        session_from_query
    {
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
    };

    tracing::info!(
        "WebSocket connection request: user_id={}, authenticated={}, client_id={:?}",
        user_id,
        is_authenticated,
        client_id
    );

    WebSocketUpgrade::new()
        .upgrade(req, res, move |websocket| {
            handle_websocket_connection(
                websocket,
                user_id,
                client_id,
                role,
                is_authenticated,
                state,
            )
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

    tracing::info!(
        "WebSocket connected: user_id={}, connection_id={}",
        user_id,
        connection_id
    );

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
            let user_connection_count = connections
                .values()
                .filter(|c| c.user_id == user_id)
                .count();
            tracing::info!(
                "User {} now has {} active WebSocket connections",
                user_id,
                user_connection_count
            );
        }
    }

    // Send authentication status message
    if is_authenticated {
        let _ = msg_tx.send(ServerMessage::Connected {
            user_id: user_id.clone(),
            authenticated: true,
            client_id: client_id.clone(),
            role: role.clone(),
        });
        tracing::info!(
            "WebSocket authenticated: user_id={}, client_id={:?}, role={:?}",
            user_id,
            client_id,
            role
        );
    } else {
        let _ = msg_tx.send(ServerMessage::AuthenticationRequired);
        tracing::warn!(
            "WebSocket connection not authenticated: user_id={}",
            user_id
        );
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
                                &client_id,
                                &connection_id,
                                &msg_tx,
                                &state,
                            )
                            .await;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse WebSocket message: {} - {}", text, e);
                        }
                    }
                } else if msg.is_close() {
                    tracing::info!("WebSocket close message received");
                    break;
                }
            }
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
        }
    }

    // Cleanup
    ws_sender.abort();
    tracing::info!(
        "WebSocket disconnected: user_id={}, connection_id={}",
        user_id,
        connection_id
    );

    // Remove from connection manager
    {
        let mut connections = WS_CONNECTIONS.write().await;
        connections.remove(&connection_id);
        tracing::debug!(
            "Removed WebSocket connection: connection_id={}, user_id={}",
            connection_id,
            user_id
        );
    }
}

async fn handle_client_message(
    msg: ClientMessage,
    user_id: &str,
    client_id: &Option<String>,
    connection_id: &str,
    sender: &mpsc::UnboundedSender<ServerMessage>,
    state: &AppState,
) {
    match msg {
        ClientMessage::Subscribe {
            project_id,
            conversation_id,
        } => {
            // Check if connection is authenticated and if already subscribed
            let (is_authenticated, already_subscribed) = {
                let connections = WS_CONNECTIONS.read().await;
                if let Some(conn) = connections.get(connection_id) {
                    let already_sub = conn.project_id.as_ref() == Some(&project_id)
                        && conn.conversation_id == conversation_id;
                    (true, already_sub)
                } else {
                    (false, false)
                }
            };

            if !is_authenticated {
                let _ = sender.send(ServerMessage::AuthenticationRequired);
                tracing::warn!(
                    "Unauthenticated user {} tried to subscribe to project {}",
                    user_id,
                    project_id
                );
                return;
            }

            // Skip if already subscribed to the same project and conversation
            if already_subscribed {
                tracing::debug!(
                    "User {} already subscribed to project={}, conversation={:?}, skipping",
                    user_id,
                    project_id,
                    conversation_id
                );
                // Still send subscribed confirmation for client's state tracking
                let _ = sender.send(ServerMessage::Subscribed {
                    project_id,
                    conversation_id,
                });
                return;
            }

            // Check if conversation exists in database (skip for "new" conversations)
            if let Some(ref conv_id) = conversation_id {
                if conv_id != "new" {
                    let conversation_exists = sqlx::query_scalar::<_, bool>(
                        "SELECT EXISTS(SELECT 1 FROM conversations WHERE id = $1)",
                    )
                    .bind(conv_id)
                    .fetch_one(&state.db_pool)
                    .await
                    .unwrap_or(false);

                    if !conversation_exists {
                        tracing::warn!(
                            "User {} tried to subscribe to non-existent conversation {}",
                            user_id,
                            conv_id
                        );
                        // Send redirect to "new" conversation instead of "subscribed"
                        let _ = sender.send(ServerMessage::ConversationRedirect {
                            old_conversation_id: conv_id.clone(),
                            new_conversation_id: "new".to_string(),
                        });
                        return;
                    }
                }
            }

            tracing::info!(
                "User {} subscribing to project={}, conversation={:?}",
                user_id,
                project_id,
                conversation_id
            );

            // Add subscriber to conversation cache and preload messages if conversation is specified
            if let Some(ref conv_id) = conversation_id {
                if conv_id != "new" {
                    // Add as subscriber
                    state
                        .add_conversation_subscriber(conv_id, connection_id)
                        .await;

                    // Get messages from cache and send them
                    match state.get_conversation_messages(conv_id).await {
                        Ok(messages) => {
                            tracing::info!(
                                "Sending {} cached messages for conversation {}",
                                messages.len(),
                                conv_id
                            );
                            let _ = sender.send(ServerMessage::ConversationMessages {
                                conversation_id: conv_id.clone(),
                                messages,
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to get conversation messages: {}", e);
                        }
                    }
                }
            }

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
                    // Only replay if there are events to replay (message is still streaming)
                    if !stream_state.progress_events.is_empty() {
                        tracing::info!(
                            "Found active stream for conversation {}, replaying {} events",
                            conv_id,
                            stream_state.progress_events.len()
                        );

                        // First send the Start event to initialize the stream
                        let _ = sender.send(ServerMessage::Start {
                            id: stream_state.message_id.clone(),
                            conversation_id: conv_id.clone(),
                        });

                        // Replay all stored events in order
                        for event in &stream_state.progress_events {
                            if let Some(event_type) = event.get("type").and_then(|t| t.as_str()) {
                                match event_type {
                                    "progress" => {
                                        if let Some(content) = event.get("content") {
                                            let _ = sender.send(ServerMessage::Progress {
                                                content: content.clone(),
                                                conversation_id: conv_id.clone(),
                                            });
                                        }
                                    }
                                    "tool_use" => {
                                        if let (Some(tool), Some(tool_usage_id)) = (
                                            event.get("tool").and_then(|t| t.as_str()),
                                            event.get("tool_usage_id").and_then(|t| t.as_str()),
                                        ) {
                                            let _ = sender.send(ServerMessage::ToolUse {
                                                tool: tool.to_string(),
                                                tool_usage_id: tool_usage_id.to_string(),
                                                conversation_id: conv_id.clone(),
                                            });
                                        }
                                    }
                                    "tool_complete" => {
                                        if let (
                                            Some(tool),
                                            Some(tool_usage_id),
                                            Some(execution_time_ms),
                                        ) = (
                                            event.get("tool").and_then(|t| t.as_str()),
                                            event.get("tool_usage_id").and_then(|t| t.as_str()),
                                            event.get("execution_time_ms").and_then(|t| t.as_i64()),
                                        ) {
                                            let _ = sender.send(ServerMessage::ToolComplete {
                                                tool: tool.to_string(),
                                                tool_usage_id: tool_usage_id.to_string(),
                                                execution_time_ms,
                                                output: event.get("output").cloned(),
                                                conversation_id: conv_id.clone(),
                                            });
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }

                        tracing::info!(
                            "Replayed {} events for conversation {} (content: {} chars, tools: {})",
                            stream_state.progress_events.len(),
                            conv_id,
                            stream_state.partial_content.len(),
                            stream_state.active_tools.len()
                        );
                    } else {
                        tracing::info!(
                            "Found completed stream for conversation {} (no events to replay)",
                            conv_id
                        );
                        // Message is complete, no need to replay anything
                    }
                }
            }

            let _ = sender.send(ServerMessage::Subscribed {
                project_id,
                conversation_id,
            });
        }

        ClientMessage::Unsubscribe => {
            tracing::info!(
                "Connection {} (user {}) unsubscribing",
                connection_id,
                user_id
            );

            // Remove from conversation cache if subscribed to one
            {
                let connections = WS_CONNECTIONS.read().await;
                if let Some(conn) = connections.get(connection_id) {
                    if let Some(ref conv_id) = conn.conversation_id {
                        if conv_id != "new" {
                            state
                                .remove_conversation_subscriber(conv_id, connection_id)
                                .await;
                        }
                    }
                }
            }

            // Clear subscription in connection manager
            {
                let mut connections = WS_CONNECTIONS.write().await;
                if let Some(conn) = connections.get_mut(connection_id) {
                    conn.project_id = None;
                    conn.conversation_id = None;
                }
            }
        }

        ClientMessage::Ping => {
            let _ = sender.send(ServerMessage::Pong);
        }

        ClientMessage::AskUserResponse {
            conversation_id,
            interaction_id,
            response,
        } => {
            tracing::info!(
                "Received ask_user response: conversation={}, interaction={}, response={:?}",
                conversation_id,
                interaction_id,
                response
            );

            // Store the response in the database
            if let Err(e) =
                store_ask_user_response(state, &conversation_id, &interaction_id, &response).await
            {
                tracing::error!("Failed to store ask_user response: {}", e);
            }
        }

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
        }

        ClientMessage::SendMessage {
            project_id,
            conversation_id,
            content,
            uploaded_file_paths,
        } => {
            tracing::info!(
                "Received send message request: project={}, conversation={}, client_id={:?}",
                project_id,
                conversation_id,
                client_id
            );

            // Check if we have a client_id for Claude authentication
            if let Some(client_id_str) = client_id.clone() {
                tracing::info!(
                    "Starting chat message handler with client_id: {}",
                    client_id_str
                );
                let state_owned = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = crate::api::chat_ws::handle_chat_message_ws(
                        project_id,
                        conversation_id,
                        content,
                        uploaded_file_paths.unwrap_or_default(),
                        client_id_str,
                        state_owned,
                    )
                    .await
                    {
                        tracing::error!("Failed to handle chat message via WebSocket: {}", e);
                    }
                });
            } else {
                tracing::error!(
                    "No client_id available for Claude authentication - user_id: {}",
                    user_id
                );
                let _ = sender.send(ServerMessage::Error {
                    error: "Client not authenticated. Please complete setup first.".to_string(),
                    conversation_id: conversation_id.clone(),
                });
            }
        }

        ClientMessage::CreateConversation { project_id, title } => {
            tracing::info!(
                "Received create conversation request for project: {}",
                project_id
            );

            if let Some(client_id_str) = client_id.clone() {
                match handle_create_conversation(&project_id, title, &client_id_str, state).await {
                    Ok(conversation) => {
                        // Automatically subscribe the connection to the new conversation
                        let conversation_id = conversation.id.clone();

                        // Add as subscriber to conversation cache
                        state
                            .add_conversation_subscriber(&conversation_id, connection_id)
                            .await;

                        // Update connection's subscription in connection manager
                        {
                            let mut connections = WS_CONNECTIONS.write().await;
                            if let Some(conn) = connections.get_mut(connection_id) {
                                conn.project_id = Some(project_id.clone());
                                conn.conversation_id = Some(conversation_id.clone());
                            }
                        }

                        tracing::info!(
                            "Auto-subscribed user {} to new conversation {}",
                            user_id,
                            conversation_id
                        );

                        // Send creation confirmation
                        let _ = sender.send(ServerMessage::ConversationCreated { conversation });

                        // Send subscription confirmation
                        let _ = sender.send(ServerMessage::Subscribed {
                            project_id,
                            conversation_id: Some(conversation_id),
                        });
                    }
                    Err(e) => {
                        tracing::error!("Failed to create conversation: {}", e);
                        let _ = sender.send(ServerMessage::Error {
                            error: format!("Failed to create conversation: {}", e),
                            conversation_id: "".to_string(),
                        });
                    }
                }
            } else {
                let _ = sender.send(ServerMessage::Error {
                    error: "Not authenticated".to_string(),
                    conversation_id: "".to_string(),
                });
            }
        }

        ClientMessage::ListConversations { project_id } => {
            tracing::info!(
                "Received list conversations request for project: {}",
                project_id
            );

            if let Some(client_id_str) = client_id.clone() {
                match handle_list_conversations(&project_id, &client_id_str, state).await {
                    Ok(conversations) => {
                        let _ = sender.send(ServerMessage::ConversationList { conversations });
                    }
                    Err(e) => {
                        tracing::error!("Failed to list conversations: {}", e);
                        let _ = sender.send(ServerMessage::Error {
                            error: format!("Failed to list conversations: {}", e),
                            conversation_id: "".to_string(),
                        });
                    }
                }
            } else {
                let _ = sender.send(ServerMessage::Error {
                    error: "Not authenticated".to_string(),
                    conversation_id: "".to_string(),
                });
            }
        }

        ClientMessage::GetConversation { conversation_id } => {
            tracing::info!("Received get conversation request: {}", conversation_id);

            if let Some(client_id_str) = client_id.clone() {
                match handle_get_conversation(&conversation_id, &client_id_str, state).await {
                    Ok(conversation) => {
                        let _ = sender.send(ServerMessage::ConversationDetails { conversation });
                    }
                    Err(e) => {
                        tracing::error!("Failed to get conversation: {}", e);
                        let _ = sender.send(ServerMessage::Error {
                            error: format!("Failed to get conversation: {}", e),
                            conversation_id: conversation_id.clone(),
                        });
                    }
                }
            } else {
                let _ = sender.send(ServerMessage::Error {
                    error: "Not authenticated".to_string(),
                    conversation_id: conversation_id.clone(),
                });
            }
        }

        ClientMessage::UpdateConversation {
            conversation_id,
            title,
        } => {
            tracing::info!("Received update conversation request: {}", conversation_id);

            if let Some(client_id_str) = client_id.clone() {
                match handle_update_conversation(&conversation_id, title, &client_id_str, state)
                    .await
                {
                    Ok(conversation) => {
                        let _ = sender.send(ServerMessage::ConversationUpdated { conversation });
                    }
                    Err(e) => {
                        tracing::error!("Failed to update conversation: {}", e);
                        let _ = sender.send(ServerMessage::Error {
                            error: format!("Failed to update conversation: {}", e),
                            conversation_id: conversation_id.clone(),
                        });
                    }
                }
            } else {
                let _ = sender.send(ServerMessage::Error {
                    error: "Not authenticated".to_string(),
                    conversation_id: conversation_id.clone(),
                });
            }
        }

        ClientMessage::DeleteConversation { conversation_id } => {
            tracing::info!("Received delete conversation request: {}", conversation_id);

            if let Some(client_id_str) = client_id.clone() {
                match handle_delete_conversation(&conversation_id, &client_id_str, state).await {
                    Ok(_) => {
                        let _ = sender.send(ServerMessage::ConversationDeleted {
                            conversation_id: conversation_id.clone(),
                        });

                        // Remove from conversation cache
                        let _ = state.invalidate_conversation_cache(&conversation_id).await;
                    }
                    Err(e) => {
                        tracing::error!("Failed to delete conversation: {}", e);
                        let _ = sender.send(ServerMessage::Error {
                            error: format!("Failed to delete conversation: {}", e),
                            conversation_id: conversation_id.clone(),
                        });
                    }
                }
            } else {
                let _ = sender.send(ServerMessage::Error {
                    error: "Not authenticated".to_string(),
                    conversation_id: conversation_id.clone(),
                });
            }
        }

        ClientMessage::BulkDeleteConversations { conversation_ids } => {
            tracing::info!(
                "Received bulk delete conversations request: {} conversations",
                conversation_ids.len()
            );

            if let Some(client_id_str) = client_id.clone() {
                let mut deleted_ids = Vec::new();
                let mut failed_ids = Vec::new();

                for conversation_id in conversation_ids {
                    match handle_delete_conversation(&conversation_id, &client_id_str, state).await
                    {
                        Ok(_) => {
                            deleted_ids.push(conversation_id.clone());
                            // Remove from conversation cache
                            let _ = state.invalidate_conversation_cache(&conversation_id).await;
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to delete conversation {}: {}",
                                conversation_id,
                                e
                            );
                            failed_ids.push(conversation_id);
                        }
                    }
                }

                let _ = sender.send(ServerMessage::ConversationsBulkDeleted {
                    conversation_ids: deleted_ids,
                    failed_ids,
                });
            } else {
                let _ = sender.send(ServerMessage::Error {
                    error: "Not authenticated".to_string(),
                    conversation_id: "".to_string(),
                });
            }
        }

        ClientMessage::GetConversationMessages { conversation_id } => {
            tracing::info!(
                "Received get conversation messages request: {}",
                conversation_id
            );

            if let Some(client_id_str) = client_id.clone() {
                match handle_get_conversation_messages(&conversation_id, &client_id_str, state)
                    .await
                {
                    Ok(messages) => {
                        let _ = sender.send(ServerMessage::ConversationMessages {
                            conversation_id: conversation_id.clone(),
                            messages,
                        });
                    }
                    Err(e) => {
                        tracing::error!("Failed to get conversation messages: {}", e);
                        let _ = sender.send(ServerMessage::Error {
                            error: format!("Failed to get conversation messages: {}", e),
                            conversation_id: conversation_id.clone(),
                        });
                    }
                }
            } else {
                let _ = sender.send(ServerMessage::Error {
                    error: "Not authenticated".to_string(),
                    conversation_id: conversation_id.clone(),
                });
            }
        }
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

    let response_json = serde_json::to_string(response).map_err(|e| {
        AppError::InternalServerError(format!("Failed to serialize response: {}", e))
    })?;

    // Store as a system message with the interaction response
    let message_content = format!(
        "User response to interaction {}:\n{}",
        interaction_id, response_json
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

// WebSocket conversation management handlers
async fn handle_create_conversation(
    project_id: &str,
    title: Option<String>,
    client_id_str: &str,
    state: &AppState,
) -> Result<crate::models::Conversation, crate::utils::AppError> {
    let client_id = uuid::Uuid::parse_str(client_id_str)
        .map_err(|_| crate::utils::AppError::BadRequest("Invalid client ID".to_string()))?;

    // Verify project exists and belongs to client
    let project_exists = sqlx::query!(
        "SELECT id FROM projects WHERE id = $1 AND client_id = $2",
        project_id,
        client_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?;

    if project_exists.is_none() {
        return Err(crate::utils::AppError::NotFound(format!(
            "Project {} not found or access denied",
            project_id
        )));
    }

    // Generate new conversation ID
    let conversation_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    // Insert new conversation
    sqlx::query!(
        r#"
        INSERT INTO conversations (id, project_id, title, created_at, updated_at, is_title_manually_set)
        VALUES ($1, $2, $3, NOW(), NOW(), $4)
        "#,
        conversation_id,
        project_id,
        title,
        title.is_some() // Set manually if title was provided
    )
    .execute(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Failed to create conversation: {}", e)))?;

    // Return the created conversation
    let is_title_set = title.is_some();
    Ok(crate::models::Conversation {
        id: conversation_id,
        project_id: project_id.to_string(),
        title,
        created_at: now,
        updated_at: now,
        message_count: 0, // New conversation has no messages
        is_title_manually_set: Some(is_title_set),
    })
}

async fn handle_list_conversations(
    project_id: &str,
    client_id_str: &str,
    state: &AppState,
) -> Result<Vec<crate::models::Conversation>, crate::utils::AppError> {
    let client_id = uuid::Uuid::parse_str(client_id_str)
        .map_err(|_| crate::utils::AppError::BadRequest("Invalid client ID".to_string()))?;

    let conversations = sqlx::query(
        "SELECT 
            c.id, 
            c.project_id, 
            c.title, 
            (
                SELECT COUNT(*)::INTEGER 
                FROM messages m 
                WHERE m.conversation_id = c.id
                AND (m.is_forgotten = false OR m.is_forgotten IS NULL)
            ) AS message_count,
            c.created_at, 
            c.updated_at, 
            c.is_title_manually_set 
         FROM conversations c
         JOIN projects p ON c.project_id = p.id
         WHERE c.project_id = $1 AND p.client_id = $2
         ORDER BY c.created_at DESC 
         LIMIT 100",
    )
    .bind(project_id)
    .bind(client_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?;

    let mut conversation_list = Vec::new();
    for row in conversations {
        use sqlx::Row;
        conversation_list.push(crate::models::Conversation {
            id: row.try_get("id").map_err(|e| {
                crate::utils::AppError::InternalServerError(format!("Failed to get id: {}", e))
            })?,
            project_id: row.try_get("project_id").map_err(|e| {
                crate::utils::AppError::InternalServerError(format!(
                    "Failed to get project_id: {}",
                    e
                ))
            })?,
            title: row.try_get("title").ok(),
            created_at: row.try_get("created_at").map_err(|e| {
                crate::utils::AppError::InternalServerError(format!(
                    "Failed to get created_at: {}",
                    e
                ))
            })?,
            updated_at: row.try_get("updated_at").map_err(|e| {
                crate::utils::AppError::InternalServerError(format!(
                    "Failed to get updated_at: {}",
                    e
                ))
            })?,
            message_count: row.try_get("message_count").map_err(|e| {
                crate::utils::AppError::InternalServerError(format!(
                    "Failed to get message_count: {}",
                    e
                ))
            })?,
            is_title_manually_set: row.try_get("is_title_manually_set").ok(),
        });
    }

    Ok(conversation_list)
}

async fn handle_get_conversation(
    conversation_id: &str,
    client_id_str: &str,
    state: &AppState,
) -> Result<crate::models::Conversation, crate::utils::AppError> {
    let client_id = uuid::Uuid::parse_str(client_id_str)
        .map_err(|_| crate::utils::AppError::BadRequest("Invalid client ID".to_string()))?;
    use sqlx::Row;

    let conversation_row = sqlx::query(
        "SELECT 
            c.id, 
            c.project_id, 
            c.title, 
            (
                SELECT COUNT(*)::INTEGER 
                FROM messages m 
                WHERE m.conversation_id = c.id
                AND (m.is_forgotten = false OR m.is_forgotten IS NULL)
            ) AS message_count,
            c.created_at, 
            c.updated_at, 
            c.is_title_manually_set 
         FROM conversations c
         JOIN projects p ON c.project_id = p.id
         WHERE c.id = $1 AND p.client_id = $2",
    )
    .bind(conversation_id)
    .bind(client_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?
    .ok_or(crate::utils::AppError::NotFound(format!(
        "Conversation {} not found or access denied",
        conversation_id
    )))?;

    Ok(crate::models::Conversation {
        id: conversation_row.try_get("id").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!("Failed to get id: {}", e))
        })?,
        project_id: conversation_row.try_get("project_id").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!("Failed to get project_id: {}", e))
        })?,
        title: conversation_row.try_get("title").ok(),
        created_at: conversation_row.try_get("created_at").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!("Failed to get created_at: {}", e))
        })?,
        updated_at: conversation_row.try_get("updated_at").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!("Failed to get updated_at: {}", e))
        })?,
        message_count: conversation_row.try_get("message_count").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!(
                "Failed to get message_count: {}",
                e
            ))
        })?,
        is_title_manually_set: conversation_row.try_get("is_title_manually_set").ok(),
    })
}

async fn handle_update_conversation(
    conversation_id: &str,
    title: Option<String>,
    client_id_str: &str,
    state: &AppState,
) -> Result<crate::models::Conversation, crate::utils::AppError> {
    let client_id = uuid::Uuid::parse_str(client_id_str)
        .map_err(|_| crate::utils::AppError::BadRequest("Invalid client ID".to_string()))?;
    use sqlx::Row;

    let now = chrono::Utc::now();

    // Update in database and mark as manually set if title is provided
    // Include authorization check
    if title.is_some() {
        sqlx::query(
            "UPDATE conversations 
             SET title = $1, is_title_manually_set = true, updated_at = $2 
             FROM projects p
             WHERE conversations.id = $3 AND conversations.project_id = p.id AND p.client_id = $4",
        )
        .bind(&title)
        .bind(now)
        .bind(conversation_id)
        .bind(client_id)
    } else {
        sqlx::query(
            "UPDATE conversations 
             SET updated_at = $1 
             FROM projects p
             WHERE conversations.id = $2 AND conversations.project_id = p.id AND p.client_id = $3",
        )
        .bind(now)
        .bind(conversation_id)
        .bind(client_id)
    }
    .execute(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?;

    // Fetch updated conversation
    let updated = sqlx::query(
        "SELECT 
            c.id, 
            c.project_id, 
            c.title, 
            (
                SELECT COUNT(*)::INTEGER 
                FROM messages m 
                WHERE m.conversation_id = c.id
                AND (m.is_forgotten = false OR m.is_forgotten IS NULL)
            ) AS message_count,
            c.created_at, 
            c.updated_at, 
            c.is_title_manually_set 
         FROM conversations c
         WHERE c.id = $1",
    )
    .bind(conversation_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?;

    Ok(crate::models::Conversation {
        id: updated.try_get("id").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!("Failed to get id: {}", e))
        })?,
        project_id: updated.try_get("project_id").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!("Failed to get project_id: {}", e))
        })?,
        title: updated.try_get("title").ok(),
        created_at: updated.try_get("created_at").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!("Failed to get created_at: {}", e))
        })?,
        updated_at: updated.try_get("updated_at").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!("Failed to get updated_at: {}", e))
        })?,
        message_count: updated.try_get("message_count").map_err(|e| {
            crate::utils::AppError::InternalServerError(format!(
                "Failed to get message_count: {}",
                e
            ))
        })?,
        is_title_manually_set: updated.try_get("is_title_manually_set").ok(),
    })
}

async fn handle_delete_conversation(
    conversation_id: &str,
    client_id_str: &str,
    state: &AppState,
) -> Result<(), crate::utils::AppError> {
    let client_id = uuid::Uuid::parse_str(client_id_str)
        .map_err(|_| crate::utils::AppError::BadRequest("Invalid client ID".to_string()))?;
    // Delete from database with authorization check (messages will cascade delete)
    let result = sqlx::query(
        "DELETE FROM conversations 
         USING projects p 
         WHERE conversations.id = $1 AND conversations.project_id = p.id AND p.client_id = $2",
    )
    .bind(conversation_id)
    .bind(client_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?;

    // Check if any rows were affected (conversation existed and was deleted)
    if result.rows_affected() == 0 {
        return Err(crate::utils::AppError::NotFound(format!(
            "Conversation {} not found or access denied",
            conversation_id
        )));
    }

    Ok(())
}

async fn handle_get_conversation_messages(
    conversation_id: &str,
    client_id_str: &str,
    state: &AppState,
) -> Result<Vec<crate::models::Message>, crate::utils::AppError> {
    let client_id = uuid::Uuid::parse_str(client_id_str)
        .map_err(|_| crate::utils::AppError::BadRequest("Invalid client ID".to_string()))?;
    // Try to get from cache first
    match state.get_conversation_messages(conversation_id).await {
        Ok(messages) => Ok(messages),
        Err(_) => {
            // Fall back to direct database query with authorization check
            let message_rows = sqlx::query(
                "SELECT 
                    m.id, 
                    m.content, 
                    m.role, 
                    m.processing_time_ms,
                    m.created_at,
                    m.file_attachments,
                    COALESCE(
                        JSON_AGG(
                            JSON_BUILD_OBJECT(
                                'id', tu.id,
                                'message_id', tu.message_id,
                                'tool_name', tu.tool_name,
                                'tool_use_id', tu.tool_use_id,
                                'execution_time_ms', tu.execution_time_ms,
                                'createdAt', tu.created_at
                            )
                        ) FILTER (WHERE tu.id IS NOT NULL),
                        '[]'::json
                    ) as tool_usages
                FROM messages m
                LEFT JOIN tool_usages tu ON m.id = tu.message_id
                JOIN conversations c ON m.conversation_id = c.id
                JOIN projects p ON c.project_id = p.id
                WHERE m.conversation_id = $1 AND p.client_id = $2
                AND (m.is_forgotten = false OR m.is_forgotten IS NULL)
                GROUP BY m.id, m.content, m.role, m.processing_time_ms, m.created_at, m.file_attachments
                ORDER BY m.created_at ASC"
            )
            .bind(conversation_id)
            .bind(client_id)
            .fetch_all(&state.db_pool)
            .await
            .map_err(|e| crate::utils::AppError::InternalServerError(format!("Database error: {}", e)))?;

            let mut messages = Vec::new();
            for row in message_rows {
                use sqlx::Row;
                messages.push(crate::models::Message {
                    id: row.try_get("id").map_err(|e| {
                        crate::utils::AppError::InternalServerError(format!(
                            "Failed to get id: {}",
                            e
                        ))
                    })?,
                    content: row.try_get("content").map_err(|e| {
                        crate::utils::AppError::InternalServerError(format!(
                            "Failed to get content: {}",
                            e
                        ))
                    })?,
                    role: match row
                        .try_get::<String, _>("role")
                        .map_err(|e| {
                            crate::utils::AppError::InternalServerError(format!(
                                "Failed to get role: {}",
                                e
                            ))
                        })?
                        .as_str()
                    {
                        "user" => crate::models::MessageRole::User,
                        "assistant" => crate::models::MessageRole::Assistant,
                        "system" => crate::models::MessageRole::System,
                        _ => crate::models::MessageRole::User,
                    },
                    processing_time_ms: row.try_get("processing_time_ms").ok(),
                    created_at: row
                        .try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                        .ok()
                        .map(|dt| dt.to_rfc3339()),
                    file_attachments: row
                        .try_get::<Option<serde_json::Value>, _>("file_attachments")
                        .ok()
                        .flatten()
                        .and_then(|v| serde_json::from_value(v).ok()),
                    tool_usages: row
                        .try_get::<serde_json::Value, _>("tool_usages")
                        .ok()
                        .and_then(|v| serde_json::from_value(v).ok()),
                });
            }

            Ok(messages)
        }
    }
}

// Global storage for active WebSocket connections (keyed by connection_id, not user_id)
lazy_static::lazy_static! {
    pub static ref WS_CONNECTIONS: Arc<RwLock<HashMap<String, UserConnection>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub async fn broadcast_to_subscribers(
    project_id: &str,
    conversation_id: &str,
    message: ServerMessage,
) {
    let connections = WS_CONNECTIONS.read().await;

    for (connection_id, conn) in connections.iter() {
        // More strict matching to prevent wrong recipients
        let should_send = match (&conn.project_id, &conn.conversation_id) {
            (Some(user_project), Some(user_conversation)) => {
                // User is subscribed to specific project + conversation
                user_project == project_id && user_conversation == conversation_id
            }
            (Some(user_project), None) => {
                // User is subscribed to project only - only send for "new" conversations
                user_project == project_id && conversation_id == "new"
            }
            _ => false, // Not subscribed to anything
        };

        if should_send && conn.sender.send(message.clone()).is_err() {
            tracing::warn!(
                "Failed to send message to connection {} (user {})",
                connection_id,
                conn.user_id
            );
        }
    }
}

pub async fn broadcast_activity_to_project(
    project_id: &str,
    conversation_id: &str,
    sender_client_id: &str, // Changed from sender_user_id to be more explicit
    user_name: &str,
    activity_type: &str,
    message_preview: Option<String>,
) {
    let connections = WS_CONNECTIONS.read().await;

    for (connection_id, conn) in connections.iter() {
        // More precise filtering to prevent wrong notifications
        let should_notify = match &conn.project_id {
            Some(user_project) if user_project == project_id => {
                // User is in the same project
                let is_not_sender = conn.user_id != sender_client_id;
                let is_not_in_conversation =
                    conn.conversation_id.as_ref() != Some(&conversation_id.to_string());

                is_not_sender && is_not_in_conversation
            }
            _ => false, // Not in same project or not subscribed
        };

        if should_notify {
            let activity_message = ServerMessage::ConversationActivity {
                conversation_id: conversation_id.to_string(),
                user_id: sender_client_id.to_string(),
                user_name: user_name.to_string(),
                activity_type: activity_type.to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                message_preview: message_preview.clone(),
            };

            if conn.sender.send(activity_message).is_err() {
                tracing::warn!(
                    "Failed to send activity notification to connection {} (user {})",
                    connection_id,
                    conn.user_id
                );
            } else {
                tracing::debug!(
                    "Sent activity notification to user {} about conversation {}",
                    conn.user_id,
                    conversation_id
                );
            }
        }
    }
}
