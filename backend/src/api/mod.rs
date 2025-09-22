// Clean API organization without confusing _domain naming

// Core business modules  
pub mod auth;
pub mod projects;
pub mod chat;

// Supporting modules
pub mod admin;
pub mod uploads;

// Re-exports for common functionality
pub use chat::websocket;

use salvo::prelude::*;

/// Main API router that combines all modules
pub fn api_routes() -> Router {
    Router::new()
        .push(auth::auth_routes())
        .push(projects::project_routes())
        .push(chat::chat_routes())
        .push(admin::admin_routes())
        .push(uploads::upload_routes())
}