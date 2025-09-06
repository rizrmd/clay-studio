use futures_util::{SinkExt, StreamExt};
use salvo::prelude::*;
use salvo::websocket::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::utils::{get_app_state, AppError, AppState};

pub mod types;
pub mod auth;
pub mod handlers;
pub mod broadcast;
pub mod claude_md;

use types::{ClientMessage, ServerMessage};
use auth::extract_session_data;
use handlers::{
    conversation::{
        handle_create_conversation, handle_list_conversations, handle_get_conversation,
        handle_update_conversation, handle_delete_conversation, handle_get_conversation_messages,
        store_ask_user_response
    },
    subscription::{handle_subscribe, handle_unsubscribe, add_connection, remove_connection},
    streaming::handle_stop_streaming,
};

// Re-export for backward compatibility
pub use broadcast::{broadcast_to_subscribers, broadcast_activity_to_project};
pub use types::{ServerMessage as WebSocketServerMessage};

#[handler]
pub async fn handle_websocket(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?.clone();

    // Extract session data for authentication
    let (user_id, client_id, role, is_authenticated) = 
        extract_session_data(req, depot, &state).await;

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
        add_connection(connection_id.clone(), user_id.clone(), msg_tx.clone()).await;
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
    remove_connection(&connection_id, &user_id).await;
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
            handle_subscribe(
                project_id,
                conversation_id,
                user_id,
                connection_id,
                sender,
                state,
            )
            .await;
        }

        ClientMessage::Unsubscribe => {
            handle_unsubscribe(connection_id, user_id, state).await;
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
            handle_stop_streaming(conversation_id, state).await;
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
                        handle_subscribe(
                            project_id.clone(),
                            Some(conversation_id.clone()),
                            user_id,
                            connection_id,
                            sender,
                            state,
                        )
                        .await;

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