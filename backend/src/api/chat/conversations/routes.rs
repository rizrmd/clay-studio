use salvo::prelude::*;
use super::crud::{list_conversations, get_conversation, create_conversation, update_conversation, delete_conversation};
use crate::utils::middleware::auth::auth_required;
use crate::utils::middleware::client_scoped;

pub fn conversation_routes() -> Router {
    Router::new()
        .hoop(auth_required)
        .hoop(client_scoped)
        .push(Router::with_path("/conversations")
            .get(list_conversations)
            .post(create_conversation))
        .push(Router::with_path("/conversations/{conversation_id}")
            .get(get_conversation)
            .put(update_conversation)
            .delete(delete_conversation))
}