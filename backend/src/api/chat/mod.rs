// Chat and conversation management
pub mod conversations;
pub mod websocket;
pub mod chat_ws;

use salvo::prelude::*;

pub fn chat_routes() -> Router {
    Router::new()
        .push(conversations::routes::conversation_routes())
}
