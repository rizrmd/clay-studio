mod config;
mod db;
mod error;
mod handlers;
mod models;
mod state;
mod middleware;

use salvo::prelude::*;
use tracing_subscriber;
use dotenv::dotenv;

use crate::config::Config;
use crate::state::AppState;
use crate::handlers::{chat, conversations, projects};
use crate::middleware::inject_state;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    
    // Configure logging - only show warnings and errors
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("clay_studio_backend=warn".parse()?)
                .add_directive("salvo=warn".parse()?)
                .add_directive("sea_orm=warn".parse()?)
                .add_directive("sqlx=warn".parse()?)
        )
        .init();

    let config = Config::from_env()?;
    let state = AppState::new(&config).await?;

    // API routes with state injection
    let api_router = Router::new()
        .hoop(inject_state(state))
        .push(Router::with_path("/health").get(health_check))
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
        );

    // Main router - serve API routes
    let router = Router::new()
        .push(Router::with_path("/api").push(api_router));

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