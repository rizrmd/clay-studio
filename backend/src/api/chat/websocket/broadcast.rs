use crate::api::websocket::types::ServerMessage;
use crate::api::websocket::handlers::subscription::WS_CONNECTIONS;

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