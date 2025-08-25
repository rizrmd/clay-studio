use salvo::prelude::*;
use salvo::sse::{self, SseEvent};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use sqlx::Row;
use tokio::sync::mpsc;

use crate::models::client::{ClientCreateRequest, ClientStatus};
use crate::models::client_config::ClientConfig;
use crate::utils::AppState;
use crate::utils::AppError;
use crate::core::claude::ClaudeManager;


#[derive(Debug, Serialize)]
pub struct ClientResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: ClientStatus,
    #[serde(rename = "installPath")]
    pub install_path: String,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[handler]
pub async fn create_client(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let create_request: ClientCreateRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    // Check if there's already an incomplete client (no claude_token)
    let existing_client = sqlx::query("SELECT id, name, description, status, install_path, created_at, updated_at FROM clients WHERE claude_token IS NULL LIMIT 1")
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    if let Some(client_row) = existing_client {
        // Return the existing incomplete client instead of creating a new one
        let client_id: Uuid = client_row.get("id");
        let created_at: chrono::DateTime<chrono::Utc> = client_row.get("created_at");
        let updated_at: chrono::DateTime<chrono::Utc> = client_row.get("updated_at");
        let status_str: String = client_row.get("status");
        let status = match status_str.as_str() {
            "pending" => ClientStatus::Pending,
            "installing" => ClientStatus::Installing,
            "active" => ClientStatus::Active,
            "error" => ClientStatus::Error,
            _ => ClientStatus::Pending,
        };
        
        let client_response = ClientResponse {
            id: client_id.to_string(),
            name: client_row.get("name"),
            description: client_row.get("description"),
            status,
            install_path: client_row.get("install_path"),
            created_at,
            updated_at,
        };
        
        tracing::info!("Returning existing incomplete client: {}", client_id);
        res.render(Json(client_response));
        return Ok(());
    }
    
    let client_id = Uuid::new_v4();
    let install_path = format!(".clients/{}", client_id);
    let now = Utc::now();
    
    // Create default client config with registration enabled by default for new clients
    let default_config = ClientConfig {
        registration_enabled: true,
        ..Default::default()
    };
    let config_json = serde_json::to_value(default_config)
        .map_err(|e| AppError::InternalServerError(format!("Failed to serialize config: {}", e)))?;
    
    // Insert into database
    sqlx::query(
        r#"
        INSERT INTO clients (id, name, description, status, install_path, config, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(client_id)
    .bind(&create_request.name)
    .bind(&create_request.description)
    .bind("pending")
    .bind(&install_path)
    .bind(&config_json)
    .bind(now)
    .bind(now)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to create client: {}", e)))?;

    let client_response = ClientResponse {
        id: client_id.to_string(),
        name: create_request.name,
        description: create_request.description,
        status: ClientStatus::Pending,
        install_path: install_path.clone(),
        created_at: now,
        updated_at: now,
    };

    tracing::info!("Created client: {} with registration enabled", client_id);
    
    // Start Claude setup automatically in the background
    tokio::spawn({
        let state = state.clone();
        let client_uuid = client_id;
        async move {
            tracing::info!("Starting automatic Claude setup for client {}", client_uuid);
            
            // Create a dummy channel since we're not streaming progress here
            let (tx, _rx) = mpsc::channel::<String>(100);
            
            match ClaudeManager::setup_client(client_uuid, Some(tx)).await {
                Ok(_) => {
                    tracing::info!("Claude environment ready for client {}", client_uuid);
                    
                    // Update status to pending (waiting for authentication)
                    let _ = sqlx::query("UPDATE clients SET status = $1, updated_at = $2 WHERE id = $3")
                        .bind("pending")
                        .bind(Utc::now())
                        .bind(client_uuid)
                        .execute(&state.db_pool)
                        .await;
                }
                Err(e) => {
                    tracing::error!("Failed to setup Claude for client {}: {}", client_uuid, e);
                    
                    // Update status to error
                    let _ = sqlx::query("UPDATE clients SET status = $1, updated_at = $2 WHERE id = $3")
                        .bind("error")
                        .bind(Utc::now())
                        .bind(client_uuid)
                        .execute(&state.db_pool)
                        .await;
                }
            }
        }
    });
    
    res.render(Json(client_response));
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct ClaudeSetupRequest {
    pub client_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeTokenRequest {
    pub client_id: String,
    pub claude_token: String,
}

#[handler]
pub async fn start_claude_setup(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let setup_request: ClaudeSetupRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&setup_request.client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    // Verify client exists and check if already set up
    let client_row = sqlx::query("SELECT id, name, install_path, status, claude_token FROM clients WHERE id = $1")
        .bind(client_uuid)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let client_row = client_row.ok_or_else(|| AppError::NotFound("Client not found".to_string()))?;
    
    // Check if client already has a valid Claude token
    let status: String = client_row.get("status");
    let claude_token: Option<String> = client_row.get("claude_token");
    
    if status == "active" && claude_token.is_some() {
        return Err(AppError::BadRequest("Claude Code is already set up for this client".to_string()));
    }
    
    let _install_path: String = client_row.get("install_path");
    
    // Update status to installing
    sqlx::query("UPDATE clients SET status = $1, updated_at = $2 WHERE id = $3")
        .bind("installing")
        .bind(Utc::now())
        .bind(client_uuid)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to update client status: {}", e)))?;
    
    // Start the Claude Code setup process in background
    tokio::spawn({
        let state = state.clone();
        async move {
            // First setup the environment
            match ClaudeManager::setup_client(client_uuid, None).await {
                Ok(_) => {
                    tracing::info!("Claude Code environment ready for client {}, ready for streaming setup-token", client_uuid);
                    // The actual streaming will happen via SSE endpoint
                    // Update client status to pending (ready for streaming setup)
                    let _ = sqlx::query("UPDATE clients SET status = $1, updated_at = $2 WHERE id = $3")
                        .bind("pending")
                        .bind(Utc::now())
                        .bind(client_uuid)
                        .execute(&state.db_pool)
                        .await;
                }
                Err(e) => {
                    tracing::error!("Failed to setup Claude Code environment for client {}: {}", client_uuid, e);
                    // Update client status to error
                    let _ = sqlx::query("UPDATE clients SET status = $1, updated_at = $2 WHERE id = $3")
                        .bind("error")
                        .bind(Utc::now())
                        .bind(client_uuid)
                        .execute(&state.db_pool)
                        .await;
                }
            }
        }
    });
    
    tracing::info!("Starting Claude Code setup for client {}", client_uuid);
    
    res.render(Json(serde_json::json!({
        "message": "Claude Code setup initiated. Setting up environment...",
        "client_id": setup_request.client_id,
        "status": "installing"
    })));
    Ok(())
}

#[handler]
pub async fn submit_claude_token(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let token_request: ClaudeTokenRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&token_request.client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    // Verify client exists and get install path
    let client_row = sqlx::query("SELECT id, name, install_path FROM clients WHERE id = $1")
        .bind(client_uuid)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let _client_row = client_row.ok_or_else(|| AppError::NotFound("Client not found".to_string()))?;
    
    // Submit token to Claude CLI and get OAUTH_TOKEN
    match ClaudeManager::submit_token(client_uuid, &token_request.claude_token).await {
        Ok(oauth_token) => {
            // Validate that we got a non-empty OAUTH_TOKEN
            if oauth_token.trim().is_empty() {
                return Err(AppError::InternalServerError("Received empty OAUTH_TOKEN from Claude CLI".to_string()));
            }
            
            // Store OAUTH_TOKEN in database and mark as active
            sqlx::query("UPDATE clients SET claude_token = $1, status = $2, updated_at = $3 WHERE id = $4")
                .bind(&oauth_token)
                .bind("active")
                .bind(Utc::now())
                .bind(client_uuid)
                .execute(&state.db_pool)
                .await
                .map_err(|e| AppError::InternalServerError(format!("Failed to update client token: {}", e)))?;
            
            tracing::info!("Claude Code setup completed for client {}", client_uuid);
            
            res.render(Json(serde_json::json!({
                "success": true,
                "message": "Claude Code setup completed successfully",
                "client_id": token_request.client_id,
                "status": "active"
            })));
        }
        Err(e) => {
            tracing::error!("Failed to submit token to Claude CLI for client {}: {}", client_uuid, e);
            return Err(AppError::InternalServerError(format!("Failed to complete Claude setup: {}", e)));
        }
    }
    Ok(())
}

#[handler]
pub async fn list_clients(depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    // First, fix any inconsistent states (has token but not active)
    let fix_result = sqlx::query("UPDATE clients SET status = 'active', updated_at = $1 WHERE claude_token IS NOT NULL AND status != 'active'")
        .bind(Utc::now())
        .execute(&state.db_pool)
        .await;
    
    if let Ok(result) = fix_result {
        if result.rows_affected() > 0 {
            tracing::info!("Fixed {} clients with inconsistent state (had token but not active)", result.rows_affected());
        }
    }
    
    let rows = sqlx::query("SELECT id, name, description, status, install_path, created_at, updated_at FROM clients ORDER BY created_at ASC")
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let clients: Vec<ClientResponse> = rows.iter().map(|row| {
        let id: Uuid = row.get("id");
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");
        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "pending" => ClientStatus::Pending,
            "installing" => ClientStatus::Installing,
            "active" => ClientStatus::Active,
            "error" => ClientStatus::Error,
            _ => ClientStatus::Pending,
        };
        
        ClientResponse {
            id: id.to_string(),
            name: row.get("name"),
            description: row.get("description"),
            status,
            install_path: row.get("install_path"),
            created_at,
            updated_at,
        }
    }).collect();
    
    res.render(Json(clients));
    Ok(())
}

#[handler]
pub async fn get_client_status(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let client_id = req.param::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("Missing client_id".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    let client_row = sqlx::query("SELECT id, status, updated_at FROM clients WHERE id = $1")
        .bind(client_uuid)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let client_row = client_row.ok_or_else(|| AppError::NotFound("Client not found".to_string()))?;
    
    let status_str: String = client_row.get("status");
    let updated_at: chrono::DateTime<chrono::Utc> = client_row.get("updated_at");
    
    res.render(Json(serde_json::json!({
        "client_id": client_id,
        "status": status_str,
        "updated_at": updated_at
    })));
    Ok(())
}

#[handler]
pub async fn get_setup_progress(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let client_id = req.param::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("Missing client_id".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    let client_row = sqlx::query("SELECT id, status, claude_token, install_path, updated_at FROM clients WHERE id = $1")
        .bind(client_uuid)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let client_row = client_row.ok_or_else(|| AppError::NotFound("Client not found".to_string()))?;
    
    let mut status_str: String = client_row.get("status");
    let claude_token: Option<String> = client_row.get("claude_token");
    let install_path: String = client_row.get("install_path");
    let updated_at: chrono::DateTime<chrono::Utc> = client_row.get("updated_at");
    
    // Fix inconsistent state
    if claude_token.is_some() && status_str != "active" {
        tracing::info!("Fixing inconsistent state in get_setup_progress for client {}: has token but status is {}", client_uuid, status_str);
        let _ = sqlx::query("UPDATE clients SET status = $1, updated_at = $2 WHERE id = $3")
            .bind("active")
            .bind(Utc::now())
            .bind(client_uuid)
            .execute(&state.db_pool)
            .await;
        status_str = "active".to_string();
    }
    
    // Determine progress message based on status
    let progress_message = match status_str.as_str() {
        "pending" => {
            if claude_token.is_some() {
                "Setup complete. Client is active."
            } else {
                "Waiting for Claude token submission."
            }
        },
        "installing" => {
            if claude_token.is_some() {
                "Setup complete. Client is active."
            } else {
                "Installing Claude Code environment..."
            }
        },
        "active" => "Client is active and ready.",
        "error" => "Setup failed. Please check logs.",
        _ => "Unknown status"
    };
    
    // Check if setup files exist to provide more detail
    // Use CLIENTS_DIR env var, or default to ../.clients (project root)
    let clients_base = std::env::var("CLIENTS_DIR")
        .unwrap_or_else(|_| "../.clients".to_string());
    let clients_base_path = std::path::Path::new(&clients_base);
    
    // Extract just the client ID from the install_path (e.g., ".clients/uuid" -> "uuid")
    let client_id_str = install_path.split('/').next_back().unwrap_or("");
    let client_dir = clients_base_path.join(client_id_str);
    let bun_path = clients_base_path.join("bun");
    
    let bun_exists = bun_path.join("bin/bun").exists();
    let has_packages = client_dir.join("node_modules").exists();
    
    res.render(Json(serde_json::json!({
        "client_id": client_id,
        "status": status_str,
        "progress_message": progress_message,
        "has_token": claude_token.is_some(),
        "bun_installed": bun_exists,
        "packages_installed": has_packages,
        "updated_at": updated_at
    })));
    Ok(())
}

pub fn client_routes() -> Router {
    Router::new()
        .push(Router::with_path("/clients").get(list_clients).post(create_client))
        .push(Router::with_path("/clients/cleanup").post(cleanup_invalid_clients))
        .push(Router::with_path("/clients/<client_id>/status").get(get_client_status))
        .push(Router::with_path("/clients/<client_id>/setup-progress").get(get_setup_progress))
        .push(Router::with_path("/claude/setup-token").post(start_claude_setup))
        .push(Router::with_path("/claude/token").post(submit_claude_token))
        .push(Router::with_path("/claude-sse").get(claude_setup_sse_query))
}

#[handler]
pub async fn claude_setup_sse_query(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let client_id = req.query::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("Missing client_id query parameter".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    // Get client details including token
    let client_row = sqlx::query("SELECT id, name, install_path, status, claude_token FROM clients WHERE id = $1")
        .bind(client_uuid)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let client_row = client_row.ok_or_else(|| AppError::NotFound("Client not found".to_string()))?;
    let _install_path: String = client_row.get("install_path");
    let current_status: String = client_row.get("status");
    let claude_token: Option<String> = client_row.get("claude_token");
    
    // Fix inconsistent state: if has token but status isn't active, fix it
    if claude_token.is_some() && current_status != "active" {
        tracing::info!("Fixing inconsistent state for client {}: has token but status is {}", client_uuid, current_status);
        let _ = sqlx::query("UPDATE clients SET status = $1, updated_at = $2 WHERE id = $3")
            .bind("active")
            .bind(Utc::now())
            .bind(client_uuid)
            .execute(&state.db_pool)
            .await;
        
        res.render(Json(serde_json::json!({
            "message": "Client already set up (fixed inconsistent state)",
            "status": "active",
            "has_token": true
        })));
        return Ok(());
    }
    
    // Check if already setup (active with token)
    if current_status == "active" && claude_token.is_some() {
        res.render(Json(serde_json::json!({
            "message": "Client already set up",
            "status": current_status,
            "has_token": true
        })));
        return Ok(());
    }
    
    // Check if already in progress - allow re-attempting if stuck and no token
    if current_status == "installing" && claude_token.is_none() {
        // Reset status to created to allow retry
        tracing::info!("Resetting stuck client {} from installing to created", client_uuid);
        let _ = sqlx::query("UPDATE clients SET status = $1, updated_at = $2 WHERE id = $3")
            .bind("created")
            .bind(Utc::now())
            .bind(client_uuid)
            .execute(&state.db_pool)
            .await;
    }
    
    // Update status to installing to prevent duplicate setups
    let _ = sqlx::query("UPDATE clients SET status = $1, updated_at = $2 WHERE id = $3")
        .bind("installing")
        .bind(Utc::now())
        .bind(client_uuid)
        .execute(&state.db_pool)
        .await;
    
    // Create channel for progress updates
    let (tx, mut rx) = mpsc::channel::<String>(100);
    
    // Start setup in background with progress reporting
    let state_clone = state.clone();
    let client_uuid_clone = client_uuid;
    
    tokio::spawn(async move {
        match ClaudeManager::setup_client(client_uuid_clone, Some(tx.clone())).await {
            Ok(_) => {
                // Send a message that environment is ready
                let _ = tx.send("Environment setup complete. Starting authentication flow...".to_string()).await;
                
                // Start streaming claude setup-token output
                match ClaudeManager::start_setup_token_stream(client_uuid_clone, tx.clone()).await {
                    Ok(_) => {
                        // Check if OAuth token was captured during streaming
                        let setup = ClaudeManager::get_client_setup(client_uuid_clone);
                        let oauth_token = if let Some(setup) = setup {
                            setup.get_oauth_token().await
                        } else {
                            None
                        };
                        
                        if let Some(token) = oauth_token {
                            // OAuth token was captured automatically, update status to active
                            tracing::info!("OAuth token captured automatically for client {}", client_uuid_clone);
                            
                            let _ = sqlx::query("UPDATE clients SET claude_token = $1, status = $2, updated_at = $3 WHERE id = $4")
                                .bind(&token)
                                .bind("active")
                                .bind(Utc::now())
                                .bind(client_uuid_clone)
                                .execute(&state_clone.db_pool)
                                .await;
                            
                            let _ = tx.send(format!("Token captured successfully: {}", token)).await;
                        } else {
                            // No token captured, update status to pending
                            let _ = sqlx::query("UPDATE clients SET status = $1, updated_at = $2 WHERE id = $3")
                                .bind("pending")
                                .bind(Utc::now())
                                .bind(client_uuid_clone)
                                .execute(&state_clone.db_pool)
                                .await;
                        }
                        
                        let _ = tx.send("COMPLETE".to_string()).await;
                    }
                    Err(e) => {
                        let _ = tx.send(format!("ERROR: Failed to start setup token stream: {}", e)).await;
                        
                        // Update status to error on failure
                        let _ = sqlx::query("UPDATE clients SET status = $1, updated_at = $2 WHERE id = $3")
                            .bind("error")
                            .bind(Utc::now())
                            .bind(client_uuid_clone)
                            .execute(&state_clone.db_pool)
                            .await;
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(format!("ERROR: {}", e)).await;
            }
        }
    });
    
    // Set up SSE response
    let event_stream = async_stream::stream! {
        // Send initial event
        let start_event = SseEvent::default()
            .name("start")
            .json(serde_json::json!({"message": "Starting Claude Code setup..."}));
        if let Ok(event) = start_event {
            yield Ok::<SseEvent, salvo::Error>(event);
        }
        
        // Stream progress updates
        while let Some(message) = rx.recv().await {
            if message == "COMPLETE" {
                let complete_event = SseEvent::default()
                    .name("complete")
                    .json(serde_json::json!({"message": "Setup completed successfully"}));
                if let Ok(event) = complete_event {
                    yield Ok(event);
                }
                break;
            } else if message == "INPUT_READY" {
                let input_ready_event = SseEvent::default()
                    .name("input_ready")
                    .json(serde_json::json!({"message": "Ready for token input", "input_ready": true}));
                if let Ok(event) = input_ready_event {
                    yield Ok(event);
                }
                continue;
            } else if message.starts_with("ERROR:") {
                let error_event = SseEvent::default()
                    .name("error")
                    .json(serde_json::json!({"message": message}));
                if let Ok(event) = error_event {
                    yield Ok(event);
                }
                break;
            } else {
                let progress_event = SseEvent::default()
                    .name("progress")
                    .json(serde_json::json!({"message": message}));
                if let Ok(event) = progress_event {
                    yield Ok(event);
                }
            }
        }
    };
    
    sse::stream(res, event_stream);
    Ok(())
}

#[handler]
pub async fn cleanup_invalid_clients(depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    // Reset clients that are marked as active but don't have claude_token
    let result = sqlx::query("UPDATE clients SET status = 'pending', updated_at = $1 WHERE status = 'active' AND claude_token IS NULL")
        .bind(Utc::now())
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let rows_affected = result.rows_affected();
    
    tracing::info!("Cleaned up {} clients with inconsistent state", rows_affected);
    
    res.render(Json(serde_json::json!({
        "message": format!("Cleaned up {} clients", rows_affected),
        "clients_reset": rows_affected
    })));
    Ok(())
}