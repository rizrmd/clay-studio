mod config;
mod db;
mod error;
mod handlers;
mod models;
mod state;
mod middleware;
mod claude;

use salvo::prelude::*;
use salvo::serve_static::StaticDir;
use salvo::session::SessionHandler;
use salvo_session::MemoryStore;
use dotenv::dotenv;

use crate::config::Config;
use crate::state::AppState;
use crate::handlers::{auth, chat, clients, conversations, projects};
use crate::middleware::{inject_state, auth::auth_required};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    
    // Configure logging - show info level for debugging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("clay_studio_backend=info".parse()?)
                .add_directive("salvo=info".parse()?)
                .add_directive("sea_orm=warn".parse()?)
                .add_directive("sqlx=warn".parse()?)
        )
        .init();

    let config = Config::from_env()?;
    let state = AppState::new(&config).await?;

    // Session configuration
    let default_secret = "clay-studio-secret-key-change-in-production-this-is-64-bytes-long";
    let session_secret = std::env::var("SESSION_SECRET").unwrap_or_else(|_| default_secret.to_string());
    
    // Ensure the session secret is at least 64 bytes
    let session_key = if session_secret.len() < 64 {
        format!("{}{}", session_secret, "0".repeat(64 - session_secret.len()))
    } else {
        session_secret
    };
    
    let session_handler = SessionHandler::builder(
        MemoryStore::new(),
        session_key.as_bytes(),
    )
    .build()
    .unwrap();

    // Public routes (no auth required)
    let public_router = Router::new()
        .push(Router::with_path("/health").get(health_check))
        .push(Router::with_path("/auth").push(auth::auth_routes()))
        .push(Router::new().push(clients::client_routes())); // Allow client creation during initial setup

    // Protected routes (auth required)
    let protected_router = Router::new()
        .hoop(auth_required)
        .push(Router::with_path("/chat").post(chat::handle_chat))
        .push(
            Router::with_path("/conversations/<conversation_id>/context")
                .get(conversations::get_conversation_context)
        )
        .push(
            Router::with_path("/projects/<project_id>/context")
                .get(projects::get_project_context)
        )
        .push(
            Router::with_path("/conversations")
                .get(conversations::list_conversations)
                .post(conversations::create_conversation)
        )
        .push(
            Router::with_path("/conversations/<conversation_id>")
                .get(conversations::get_conversation)
                .put(conversations::update_conversation)
                .delete(conversations::delete_conversation)
        )
;

    // API routes with state injection and session handling
    let api_router = Router::new()
        .hoop(session_handler)
        .hoop(inject_state(state))
        .push(public_router)
        .push(protected_router);

    // Static file serving for frontend
    let static_path = std::env::var("STATIC_FILES_PATH")
        .unwrap_or_else(|_| "./frontend/dist".to_string());
    
    let static_service = StaticDir::new(&static_path).defaults("index.html");
    
    // Main router - API routes first, then static files as fallback
    let router = Router::new()
        .push(Router::with_path("/api").push(api_router))
        .push(Router::with_path("{*path}").get(static_service));

    let acceptor = TcpListener::new(&config.server_address).bind().await;
    
    // Print startup message directly to stdout
    let service = Service::new(router);
    Server::new(acceptor).serve(service).await;
    Ok(())
}

#[handler]
async fn health_check(res: &mut Response) {
    res.render(Json(serde_json::json!({
        "status": "ok",
        "service": "clay-studio-backend"
    })));
}