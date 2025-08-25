mod core;
mod utils;
mod api;
mod models;

use salvo::prelude::*;
use salvo::serve_static::StaticDir;
use salvo::session::SessionHandler;
use dotenv::dotenv;

use crate::utils::{Config, AppState};
use crate::api::{auth, chat, clients, conversations, conversations_forget, projects, upload};
use crate::utils::middleware::{inject_state, auth::auth_required};
use crate::core::sessions::PostgresSessionStore;

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
    
    // Use PostgreSQL session store for persistence
    let postgres_store = PostgresSessionStore::new(state.db.clone());
    
    let session_handler = SessionHandler::builder(
        postgres_store,
        session_key.as_bytes(),
    )
    .cookie_name("clay_session")
    .cookie_path("/")
    .same_site_policy(salvo::http::cookie::SameSite::Lax)
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
        .push(Router::with_path("/chat/stream").post(chat::handle_chat_stream))
        .push(
            Router::with_path("/projects")
                .get(projects::list_projects)
                .post(projects::create_project)
        )
        .push(
            Router::with_path("/projects/{project_id}/context")
                .get(projects::get_project_context)
        )
        .push(
            Router::with_path("/projects/{project_id}/queries")
                .get(projects::list_queries)
                .post(projects::save_query)
        )
        .push(
            Router::with_path("/projects/{project_id}/claude-md")
                .get(projects::get_claude_md)
                .put(projects::save_claude_md)
        )
        // Conversation routes - more specific paths first
        .push(
            Router::with_path("/conversations/{conversation_id}/messages")
                .get(conversations::get_conversation_messages)
        )
        .push(
            Router::with_path("/conversations/{conversation_id}/forget-after")
                .put(conversations_forget::forget_messages_after)
                .delete(conversations_forget::restore_forgotten_messages)
                .get(conversations_forget::get_forgotten_status)
        )
        .push(
            Router::with_path("/conversations/{conversation_id}/context")
                .get(conversations::get_conversation_context)
        )
        .push(
            Router::with_path("/conversations/{conversation_id}")
                .get(conversations::get_conversation)
                .put(conversations::update_conversation)
                .delete(conversations::delete_conversation)
        )
        .push(
            Router::with_path("/conversations")
                .get(conversations::list_conversations)
                .post(conversations::create_conversation)
        )
        .push(Router::with_path("/upload").post(upload::handle_file_upload))
        .push(Router::with_path("/uploads").get(upload::handle_list_uploads))
        .push(
            Router::with_path("/uploads/{file_id}/description")
                .put(upload::handle_update_file_description)
        )
        .push(
            Router::with_path("/uploads/<id>")
                .delete(upload::handle_delete_upload)
        )
        .push(
            Router::with_path("/uploads/{client_id}/{project_id}/{file_name}")
                .get(upload::handle_file_download)
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