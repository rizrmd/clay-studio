use crate::utils::AppState;

pub async fn handle_stop_streaming(conversation_id: String, state: &AppState) {
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