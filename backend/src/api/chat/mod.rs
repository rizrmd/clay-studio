// Chat and conversation management
pub mod conversations;
pub mod websocket;
pub mod chat_ws;
pub mod tool_usages;

use salvo::prelude::*;

pub fn chat_routes() -> Router {
    Router::new()
        .push(conversations::routes::conversation_routes())
        .push(tool_usages::tool_usage_routes())
}
