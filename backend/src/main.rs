// Forbid unwrap and expect to prevent panics
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

mod api;
mod core;
mod models;
mod utils;

use dotenv::dotenv;
use salvo::conn::tcp::TcpAcceptor;
use salvo::prelude::*;
use salvo::serve_static::StaticDir;
use salvo::session::SessionHandler;
use std::time::Duration;
use tokio::signal;

use crate::api::{
    admin, auth, client_management, clients, conversations_forget, datasources, projects, prompt, tool_usage,
    upload, user_management, websocket,
};
use crate::core::sessions::PostgresSessionStore;
use crate::utils::middleware::{
    auth::{admin_required, auth_required, root_required},
    client_scoped, inject_state,
};
use crate::utils::{get_app_state, AppState, Config};

/// Kill process using the specified port
async fn kill_process_using_port(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    eprintln!("🔪 Killing process using port {}...", port);

    // Find the PID using the port
    let output = Command::new("lsof")
        .arg("-ti")
        .arg(format!(":{}", port))
        .output()?;

    if output.status.success() {
        let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !pid_str.is_empty() {
            eprintln!("📍 Found process {} using port {}", pid_str, port);

            // Kill the process
            let kill_output = Command::new("kill").arg("-9").arg(&pid_str).output()?;

            if kill_output.status.success() {
                eprintln!(
                    "✅ Successfully killed process {} using port {}",
                    pid_str, port
                );
                // Give the process time to fully release the port
                tokio::time::sleep(Duration::from_millis(1000)).await;
            } else {
                let stderr = String::from_utf8_lossy(&kill_output.stderr);
                eprintln!("❌ Failed to kill process {}: {}", pid_str, stderr);
                return Err(format!("Failed to kill process {}: {}", pid_str, stderr).into());
            }
        }
    }

    Ok(())
}

/// Bind to address with port conflict resolution
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
                    eprintln!(
                        "⚠️  Port {} is in use (attempt {}/{})",
                        socket_addr.port(),
                        attempt,
                        max_retries
                    );

                    // Try to kill the process using the port
                    if let Err(kill_err) = kill_process_using_port(socket_addr.port()).await {
                        eprintln!(
                            "⚠️  Failed to kill process using port {}: {}",
                            socket_addr.port(),
                            kill_err
                        );

                        if attempt < max_retries {
                            eprintln!("⚠️  Retrying in 1 second...");
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            continue;
                        }
                    } else {
                        // Successfully killed process, try binding again immediately
                        continue;
                    }
                }

                eprintln!("❌ Failed to bind to {}: {}", address, e);
                std::process::exit(1);
            }
        }
    }

    eprintln!(
        "❌ Failed to bind to {} after {} attempts",
        address, max_retries
    );
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

/// Start the MCP server instance
async fn start_mcp_server() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    // Get MCP server path - look for the built binary
    let mcp_server_path = {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        
        // Development paths (target directory)
        let release_path = current_dir.join("target/release/mcp_server");
        let debug_path = current_dir.join("target/debug/mcp_server");
        
        // Production path (same directory as main binary or /app)
        let production_path = current_dir.join("mcp_server");
        let app_path = std::path::PathBuf::from("/app/mcp_server");
        
        // Check paths in order: debug (dev), production (same dir), /app (container), release (dev fallback)
        if debug_path.exists() {
            debug_path.canonicalize().unwrap_or(debug_path)
        } else if production_path.exists() {
            production_path.canonicalize().unwrap_or(production_path)
        } else if app_path.exists() {
            app_path.canonicalize().unwrap_or(app_path)
        } else if release_path.exists() {
            release_path.canonicalize().unwrap_or(release_path)
        } else {
            return Err("MCP server binary not found. Expected locations: target/debug/mcp_server, ./mcp_server, /app/mcp_server, or target/release/mcp_server".into());
        }
    };

    tracing::info!("🔧 Starting MCP server at {:?}", mcp_server_path);

    // Start the MCP server as a background process
    let mut child = Command::new(mcp_server_path)
        .arg("--http")
        .arg("--port")
        .arg("7670")
        .spawn()?;

    // Give the server a moment to start
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Check if the process is still running
    match child.try_wait() {
        Ok(Some(status)) => {
            return Err(format!("MCP server exited immediately with status: {}", status).into());
        }
        Ok(None) => {
            tracing::info!("✅ MCP server started successfully on port 7670");
        }
        Err(e) => {
            return Err(format!("Failed to check MCP server status: {}", e).into());
        }
    }

    // Detach the process so it continues running independently
    std::mem::forget(child);

    Ok(())
}

/// Ensure global Bun installation is available for all clients
async fn ensure_global_bun_installation() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;
    use std::process::Command;

    // Use CLIENTS_DIR env var, or default to ../.clients (project root)
    let clients_base = std::env::var("CLIENTS_DIR").unwrap_or_else(|_| ".clients".to_string());

    let clients_base_path = PathBuf::from(&clients_base);
    let bun_path = clients_base_path.join("bun");
    let bun_executable = bun_path.join("bin/bun");

    if bun_executable.exists() {
        tracing::info!("Global Bun installation found at {:?}", bun_executable);
        return Ok(());
    }

    tracing::info!(
        "Global Bun installation not found, installing to {:?}",
        bun_path
    );

    // Create the Bun installation directory
    std::fs::create_dir_all(&clients_base_path)?;
    std::fs::create_dir_all(&bun_path)?;

    // Download and install Bun
    let output = Command::new("bash")
        .arg("-c")
        .arg("curl -fsSL https://bun.sh/install | bash")
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin:/usr/local/bin")
        .env("HOME", clients_base_path.to_str().ok_or("Invalid clients base path")?)
        .env("BUN_INSTALL", bun_path.to_str().ok_or("Invalid bun path")?)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Failed to install Bun globally. stdout: {}, stderr: {}",
            stdout, stderr
        )
        .into());
    }

    tracing::info!(
        "Global Bun installation completed successfully at {:?}",
        bun_executable
    );
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
                .add_directive("sqlx=warn".parse()?),
        )
        .init();

    // Log the current user running the server
    let current_user = std::process::Command::new("whoami")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    tracing::info!("🔐 Server running as user: {}", current_user);

    if current_user == "root" {
        let is_production = std::env::var("RUST_ENV").unwrap_or_default() == "production";
        if is_production {
            tracing::info!(
                "⚙️  Running as root in production - Claude CLI will use 'su nobody' workaround"
            );
        } else {
            tracing::warn!(
                "⚠️  WARNING: Running as root in development! Claude CLI may not work properly."
            );
            tracing::warn!("⚠️  Consider running as a non-root user for development.");
        }
    }

    let config = Config::from_env()?;
    let state = AppState::new(&config).await?;

    // Ensure global Bun installation is available for all clients
    ensure_global_bun_installation().await?;

    // Start the MCP server instance
    start_mcp_server().await?;

    // Session configuration
    let default_secret = "clay-studio-secret-key-change-in-production-this-is-64-bytes-long";
    let session_secret =
        std::env::var("SESSION_SECRET").unwrap_or_else(|_| default_secret.to_string());

    // Ensure the session secret is at least 64 bytes
    let session_key = if session_secret.len() < 64 {
        format!(
            "{}{}",
            session_secret,
            "0".repeat(64 - session_secret.len())
        )
    } else {
        session_secret
    };

    // Use PostgreSQL session store for persistence
    let postgres_store = PostgresSessionStore::new(state.db.clone());

    let session_handler = SessionHandler::builder(postgres_store, session_key.as_bytes())
        .cookie_name("clay_session")
        .cookie_path("/")
        .same_site_policy(salvo::http::cookie::SameSite::Lax)
        .build()
        .unwrap();

    // Public routes (no auth required)
    let public_router = Router::new()
        .push(Router::with_path("/health").get(health_check))
        .push(Router::with_path("/health/database").get(database_health_check))
        .push(Router::with_path("/auth").push(auth::auth_routes()))
        .push(Router::new().push(clients::client_routes())) // Allow client creation during initial setup
        .push(Router::with_path("/debug/connections").get(api::debug::get_active_connections));

    // WebSocket route (auth checked after connection)
    let ws_router = Router::new().push(Router::with_path("/ws").get(websocket::handle_websocket));

    // Protected routes (auth required + client scoped)
    let protected_router = Router::new()
        .hoop(auth_required)
        .hoop(client_scoped)
        .push(Router::with_path("/auth").push(auth::auth_protected_routes()))
        .push(Router::with_path("/prompt/stream").post(prompt::handle_prompt_stream))
        .push(
            Router::with_path("/projects")
                .get(projects::list_projects)
                .post(projects::create_project),
        )
        .push(
            Router::with_path("/projects/{project_id}/context").get(projects::get_project_context),
        )
        .push(
            Router::with_path("/projects/{project_id}/queries")
                .get(projects::list_queries)
                .post(projects::save_query),
        )
        .push(
            Router::with_path("/projects/{project_id}/claude-md")
                .get(projects::get_claude_md)
                .put(projects::save_claude_md)
                .post(projects::refresh_claude_md),
        )
        .push(Router::with_path("/projects/{project_id}").delete(projects::delete_project))
        // Datasources routes
        .push(datasources::datasource_routes())
        // Conversation routes - more specific paths first
        .push(
            Router::with_path("/conversations/{conversation_id}/forget-after")
                .put(conversations_forget::forget_messages_after)
                .delete(conversations_forget::restore_forgotten_messages)
                .get(conversations_forget::get_forgotten_status),
        )
        .push(
            Router::with_path("/messages/{message_id}/tool-usages")
                .get(tool_usage::get_message_tool_usages),
        )
        .push(
            Router::with_path("/messages/{message_id}/tool-usage/{tool_name}")
                .get(tool_usage::get_tool_usage_by_name),
        )
        .push(
            Router::with_path("/tool-usages/{tool_usage_id}")
                .get(tool_usage::get_tool_usage_by_id),
        )
        .push(Router::with_path("/upload").post(upload::handle_file_upload))
        .push(Router::with_path("/uploads").get(upload::handle_list_uploads))
        .push(
            Router::with_path("/uploads/{file_id}/description")
                .put(upload::handle_update_file_description),
        )
        .push(Router::with_path("/uploads/<id>").delete(upload::handle_delete_upload))
        .push(
            Router::with_path("/uploads/{client_id}/{project_id}/{file_name}")
                .get(upload::handle_file_download),
        )
        .push(
            Router::with_path("/files/excel/{client_id}/{project_id}/{export_id}")
                .get(upload::handle_excel_download),
        );

    // Admin routes (accessible to admin and root roles)
    let admin_router = Router::new()
        .hoop(admin_required)
        .push(Router::with_path("/admin").push(client_management::admin_routes()))
        .push(Router::with_path("/admin").push(user_management::admin_routes()))
        .push(Router::with_path("/admin").push(admin::admin_router()));

    // Root routes (accessible only to root role)
    let root_router = Router::new()
        .hoop(root_required)
        .push(Router::with_path("/root/clients").push(client_management::root_routes()))
        .push(Router::with_path("/root/clients").push(user_management::root_routes()));

    // API routes with state injection and session handling
    let api_router = Router::new()
        .hoop(session_handler)
        .hoop(inject_state(state))
        .push(public_router)
        .push(ws_router)
        .push(protected_router)
        .push(admin_router)
        .push(root_router);

    // Static file serving for frontend
    let static_path = std::env::var("STATIC_FILES_PATH")
        .unwrap_or_else(|_| "/Users/riz/Developer/clay-studio/frontend/dist".to_string());

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
    println!(
        "🚀 Clay Studio backend listening on {} (with retry logic)",
        config.server_address
    );

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

#[handler]
async fn database_health_check(depot: &mut Depot, res: &mut Response) {
    let state = match get_app_state(depot) {
        Ok(state) => state,
        Err(_) => {
            res.status_code(salvo::http::StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(serde_json::json!({
                "status": "error",
                "error": "Failed to get application state",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })));
            return;
        }
    };

    match state.health_check().await {
        Ok(_) => {
            // Log pool statistics
            state.log_pool_stats("Health Check Endpoint").await;

            res.render(Json(serde_json::json!({
                "status": "healthy",
                "database": "connected",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })));
        }
        Err(e) => {
            tracing::error!("Database health check failed via endpoint: {}", e);
            res.status_code(salvo::http::StatusCode::SERVICE_UNAVAILABLE);
            res.render(Json(serde_json::json!({
                "status": "unhealthy",
                "database": "disconnected",
                "error": e.to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            })));
        }
    }
}
