mod core;
mod utils;
mod api;
mod models;

use salvo::prelude::*;
use salvo::serve_static::StaticDir;
use salvo::session::SessionHandler;
use salvo::conn::tcp::TcpAcceptor;
use dotenv::dotenv;
use std::time::Duration;
use tokio::signal;

use crate::utils::{Config, AppState};
use crate::api::{admin, auth, chat, clients, client_management, conversations, conversations_forget, projects, tool_usage, upload};
use crate::utils::middleware::{inject_state, auth::{auth_required, admin_required, root_required}};
use crate::core::sessions::PostgresSessionStore;

/// Bind to address with retry logic by adding delay before binding
async fn bind_with_retry(address: &str, max_retries: u32) -> TcpAcceptor {
    // Add a small initial delay to allow any previous process to fully release the port
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    for attempt in 1..=max_retries {
        let socket_addr: std::net::SocketAddr = match address.parse() {
            Ok(addr) => addr,
            Err(_) => {
                eprintln!("❌ Invalid address format: {}", address);
                std::process::exit(1);
            }
        };

        // Test if the port is available using tokio TcpListener
        match tokio::net::TcpListener::bind(socket_addr).await {
            Ok(test_listener) => {
                // Port is available, close the test listener
                drop(test_listener);
                
                // Give a small grace period for the port to be fully released
                tokio::time::sleep(Duration::from_millis(200)).await;
                
                // Now use Salvo's TcpListener 
                eprintln!("🔗 Attempting to bind to {} (attempt {})", address, attempt);
                return TcpListener::new(address).bind().await;
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::AddrInUse {
                    eprintln!("⚠️  Port {} is in use (attempt {}/{}), retrying in 1 second...", 
                             socket_addr.port(), attempt, max_retries);
                    
                    if attempt < max_retries {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                }
                
                eprintln!("❌ Failed to bind to {}: {}", address, e);
                std::process::exit(1);
            }
        }
    }
    
    eprintln!("❌ Failed to bind to {} after {} attempts", address, max_retries);
    std::process::exit(1);
}

/// Wait for shutdown signal (SIGTERM, SIGINT, or Ctrl+C)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C signal");
        }
        _ = terminate => {
            tracing::info!("Received terminate signal");
        }
    }
}

/// Ensure global Bun installation is available for all clients
async fn ensure_global_bun_installation() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    use std::path::PathBuf;
    
    // Use CLIENTS_DIR env var, or default to ../.clients (project root)
    let clients_base = std::env::var("CLIENTS_DIR")
        .unwrap_or_else(|_| ".clients".to_string());
    
    let clients_base_path = PathBuf::from(&clients_base);
    let bun_path = clients_base_path.join("bun");
    let bun_executable = bun_path.join("bin/bun");
    
    if bun_executable.exists() {
        tracing::info!("Global Bun installation found at {:?}", bun_executable);
        return Ok(());
    }
    
    tracing::info!("Global Bun installation not found, installing to {:?}", bun_path);
    
    // Create the Bun installation directory
    std::fs::create_dir_all(&clients_base_path)?;
    std::fs::create_dir_all(&bun_path)?;
    
    // Download and install Bun
    let output = Command::new("bash")
        .arg("-c")
        .arg("curl -fsSL https://bun.sh/install | bash")
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin:/usr/local/bin")
        .env("HOME", clients_base_path.to_str().unwrap())
        .env("BUN_INSTALL", bun_path.to_str().unwrap())
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("Failed to install Bun globally. stdout: {}, stderr: {}", stdout, stderr).into());
    }
    
    tracing::info!("Global Bun installation completed successfully at {:?}", bun_executable);
    Ok(())
}

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

    // Ensure global Bun installation is available for all clients
    ensure_global_bun_installation().await?;

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
            Router::with_path("/messages/{message_id}/tool-usages")
                .get(tool_usage::get_message_tool_usages)
        )
        .push(
            Router::with_path("/messages/{message_id}/tool-usage/{tool_name}")
                .get(tool_usage::get_tool_usage_by_name)
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
            Router::with_path("/conversations/new-from-message")
                .post(conversations::create_conversation_from_message)
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

    // Admin routes (accessible to admin and root roles)
    let admin_router = Router::new()
        .hoop(admin_required)
        .push(Router::with_path("/admin").push(client_management::admin_routes()))
        .push(Router::with_path("/admin").push(admin::admin_router()));
    
    // Root routes (accessible only to root role)
    let root_router = Router::new()
        .hoop(root_required)
        .push(Router::with_path("/root").push(client_management::root_routes()));

    // API routes with state injection and session handling
    let api_router = Router::new()
        .hoop(session_handler)
        .hoop(inject_state(state))
        .push(public_router)
        .push(protected_router)
        .push(admin_router)
        .push(root_router);

    // Static file serving for frontend
    let static_path = std::env::var("STATIC_FILES_PATH")
        .unwrap_or_else(|_| "/Users/riz/Developer/clay-studio/frontend/dist".to_string());
    
    tracing::info!("Static files path: {}", static_path);
    tracing::info!("STATIC_FILES_PATH env var: {:?}", std::env::var("STATIC_FILES_PATH"));
    
    // List files in static directory for debugging
    if let Ok(entries) = std::fs::read_dir(&static_path) {
        tracing::info!("Files in static directory:");
        for entry in entries {
            if let Ok(entry) = entry {
                tracing::info!("  - {:?}", entry.path());
            }
        }
    }
    
    // Configure static service for assets (no fallback)
    let assets_service = StaticDir::new(&static_path)
        .include_dot_files(false)
        .fallback("index.html");
    
    
    // Main router - Assets first (most specific), then API, then SPA fallback
    let router = Router::new()
        .push(Router::with_path("/api").push(api_router))
        .push(Router::with_path("{**path}").get(assets_service));

    // Bind with retry logic and socket reuse
    let acceptor = bind_with_retry(&config.server_address, 5).await;
    
    // Print startup message
    println!("🚀 Clay Studio backend listening on {} (with retry logic)", config.server_address);
    
    let service = Service::new(router);
    let server = Server::new(acceptor);
    
    // Run server with graceful shutdown handling
    tokio::select! {
        _ = server.serve(service) => {
            println!("🛑 Server stopped");
        }
        _ = shutdown_signal() => {
            println!("🛑 Clay Studio backend shutting down gracefully");
        }
    }
    
    Ok(())
}

#[handler]
async fn health_check(res: &mut Response) {
    res.render(Json(serde_json::json!({
        "status": "ok",
        "service": "clay-studio-backend"
    })));
}