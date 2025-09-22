use crate::api::websocket::types::{ServerMessage, UserConnection};
use crate::utils::AppState;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use super::super::claude_md::update_claude_md_if_needed;

// Global storage for active WebSocket connections (keyed by connection_id, not user_id)
lazy_static::lazy_static! {
    pub static ref WS_CONNECTIONS: Arc<RwLock<HashMap<String, UserConnection>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub async fn handle_subscribe(
    project_id: String,
    conversation_id: Option<String>,
    user_id: &str,
    connection_id: &str,
    sender: &mpsc::UnboundedSender<ServerMessage>,
    state: &AppState,
) {
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

    // Update CLAUDE.md template when connecting to a project (if needed)
    if !already_subscribed {
        // Warm up datasource connection pools for this project
        if state.config.datasource_pool_warmup {
            let project_id_warmup = project_id.clone();
            let state_warmup = state.clone();
            tokio::spawn(async move {
                use crate::utils::datasource::warm_up_project_pools;
                if let Err(e) = warm_up_project_pools(&state_warmup.db_pool, &project_id_warmup).await {
                    tracing::warn!("Failed to warm up pools for project {} on WebSocket subscribe: {}", project_id_warmup, e);
                }
            });
        }

        // Check if CLAUDE.md needs updating for this project
        let project_id_clone = project_id.clone();
        let state_clone = state.clone();
        tracing::info!("WebSocket: Triggering CLAUDE.md update check for project {}", project_id_clone);
        tokio::spawn(async move {
            match update_claude_md_if_needed(&state_clone, &project_id_clone).await {
                Ok(_) => {
                    tracing::info!("WebSocket: CLAUDE.md update check completed successfully for project {}", project_id_clone);
                }
                Err(e) => {
                    tracing::error!(
                        "WebSocket: Failed to update CLAUDE.md for project {}: {}",
                        project_id_clone,
                        e
                    );
                }
            }
        });
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

pub async fn handle_unsubscribe(
    connection_id: &str,
    user_id: &str,
    state: &AppState,
) {
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

pub async fn add_connection(
    connection_id: String,
    user_id: String,
    sender: mpsc::UnboundedSender<ServerMessage>,
) {
    let user_connection = UserConnection {
        user_id: user_id.clone(),
        sender,
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

pub async fn remove_connection(connection_id: &str, user_id: &str) {
    let mut connections = WS_CONNECTIONS.write().await;
    connections.remove(connection_id);
    tracing::debug!(
        "Removed WebSocket connection: connection_id={}, user_id={}",
        connection_id,
        user_id
    );
}