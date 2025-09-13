use salvo::prelude::*;
use super::crud::{list_conversations, get_conversation, create_conversation, update_conversation, delete_conversation};

pub fn conversation_routes() -> Router {
    Router::new()
        .push(Router::with_path("/conversations")
            .get(list_conversations)
            .post(create_conversation))
        .push(Router::with_path("/conversations/{conversation_id}")
            .get(get_conversation)
            .put(update_conversation)
            .delete(delete_conversation))
}