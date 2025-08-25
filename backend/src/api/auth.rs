use salvo::prelude::*;
use salvo::session::SessionDepotExt;
use serde::{Deserialize, Serialize};
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;
// use chrono::Utc;
use sqlx::Row;

use crate::utils::AppState;
use crate::utils::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub client_id: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String, // This is actually the hash, but named password in DB
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub client_id: String, // Client UUID as string
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub client_id: String, // Client UUID as string
    pub username: String,
    pub password: String,
    pub invite_code: Option<String>, // Optional invite code if required
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub client_id: String,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeResponse {
    pub user: UserResponse,
    pub is_setup_complete: bool,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            client_id: user.client_id,
            username: user.username,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub message: String,
}

#[handler]
pub async fn register(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let register_req: RegisterRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON".to_string()))?;

    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;

    // Parse client_id as UUID
    let client_id = Uuid::parse_str(&register_req.client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID".to_string()))?;

    // Get client and check if registration is enabled
    let client_row = sqlx::query("SELECT id, config FROM clients WHERE id = $1")
        .bind(&client_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let client_row = client_row.ok_or_else(|| AppError::BadRequest("Client not found".to_string()))?;

    // Check if this is the first user for the client
    let user_count_row = sqlx::query("SELECT COUNT(*) as count FROM users WHERE client_id = $1")
        .bind(&client_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let user_count: i64 = user_count_row.get("count");
    let is_first_user = user_count == 0;

    // Parse client config
    let config: serde_json::Value = client_row.get("config");
    let registration_enabled = config.get("registration_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Allow creating the first user even if registration is disabled
    if !is_first_user && !registration_enabled {
        return Err(AppError::BadRequest("Registration is disabled for this client".to_string()));
    }

    // Check invite code if required (skip for first user)
    if !is_first_user {
        let require_invite_code = config.get("require_invite_code")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if require_invite_code {
            let expected_code = config.get("invite_code")
                .and_then(|v| v.as_str());
            
            match (expected_code, register_req.invite_code.as_deref()) {
                (Some(expected), Some(provided)) if expected == provided => {
                    // Invite code matches, continue
                }
                _ => {
                    return Err(AppError::BadRequest("Invalid or missing invite code".to_string()));
                }
            }
        }
    }

    // Check if user already exists for this client
    let existing_user = sqlx::query("SELECT id FROM users WHERE client_id = $1 AND username = $2")
        .bind(&client_id)
        .bind(&register_req.username)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    if existing_user.is_some() {
        return Err(AppError::BadRequest("Username already exists for this client".to_string()));
    }

    // Hash password
    let password_hash = hash(&register_req.password, DEFAULT_COST)
        .map_err(|e| AppError::InternalServerError(format!("Failed to hash password: {}", e)))?;

    // Create user
    let user_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO users (id, client_id, username, password)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(&user_id)
    .bind(&client_id)
    .bind(&register_req.username)
    .bind(&password_hash)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to create user: {}", e)))?;

    // Fetch the created user
    let row = sqlx::query("SELECT id, client_id, username, password FROM users WHERE id = $1")
        .bind(&user_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to fetch user: {}", e)))?;

    let user = User {
        id: user_id.to_string(),
        client_id: client_id.to_string(),
        username: row.get("username"),
        password: row.get("password"),
    };

    // Create session
    if let Some(session) = depot.session_mut() {
        session.insert("user_id", &user.id)
            .map_err(|e| AppError::InternalServerError(format!("Failed to create session: {}", e)))?;
        session.insert("username", &user.username)
            .map_err(|e| AppError::InternalServerError(format!("Failed to create session: {}", e)))?;
    } else {
        return Err(AppError::InternalServerError("No session available".to_string()));
    }

    let auth_response = AuthResponse {
        user: UserResponse::from(user),
        message: "User registered successfully".to_string(),
    };

    res.render(Json(auth_response));
    Ok(())
}

#[handler]
pub async fn login(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let login_req: LoginRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON".to_string()))?;

    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;

    // Parse client_id as UUID
    let client_id = Uuid::parse_str(&login_req.client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID".to_string()))?;

    // Find user by username and client_id
    let row = sqlx::query(
        r#"
        SELECT id, client_id, username, password 
        FROM users 
        WHERE client_id = $1 AND username = $2
        "#,
    )
    .bind(&client_id)
    .bind(&login_req.username)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let row = row.ok_or_else(|| AppError::BadRequest("Invalid credentials".to_string()))?;

    let user = User {
        id: {
            let id: Uuid = row.get("id");
            id.to_string()
        },
        client_id: {
            let cid: Uuid = row.get("client_id");
            cid.to_string()
        },
        username: row.get("username"),
        password: row.get("password"),
    };

    // Verify password
    let is_valid = verify(&login_req.password, &user.password)
        .map_err(|e| AppError::InternalServerError(format!("Failed to verify password: {}", e)))?;

    if !is_valid {
        return Err(AppError::BadRequest("Invalid credentials".to_string()));
    }

    // Create session
    if let Some(session) = depot.session_mut() {
        session.insert("user_id", &user.id)
            .map_err(|e| AppError::InternalServerError(format!("Failed to create session: {}", e)))?;
        session.insert("username", &user.username)
            .map_err(|e| AppError::InternalServerError(format!("Failed to create session: {}", e)))?;
    } else {
        return Err(AppError::InternalServerError("No session available".to_string()));
    }

    let auth_response = AuthResponse {
        user: UserResponse::from(user),
        message: "Logged in successfully".to_string(),
    };

    res.render(Json(auth_response));
    Ok(())
}

#[handler]
pub async fn logout(depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    if let Some(session) = depot.session_mut() {
        session.remove("user_id");
        session.remove("username");
    }

    res.render(Json(serde_json::json!({
        "message": "Logged out successfully"
    })));
    Ok(())
}

#[handler]
pub async fn me(depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let user_id = if let Some(session) = depot.session_mut() {
        let user_id: Option<String> = session.get("user_id");
        tracing::info!("Session found, user_id: {:?}", user_id);
        user_id.ok_or_else(|| AppError::Unauthorized("Not authenticated".to_string()))?
    } else {
        tracing::warn!("No session found in depot");
        return Err(AppError::Unauthorized("No session found".to_string()));
    };

    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;

    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|e| {
            tracing::error!("Failed to parse user_id '{}': {}", user_id, e);
            AppError::InternalServerError("Invalid user ID in session".to_string())
        })?;

    let row = sqlx::query("SELECT id, client_id, username, password FROM users WHERE id = $1")
        .bind(&user_uuid)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let row = match row {
        Some(r) => r,
        None => {
            // User in session doesn't exist, clear the session
            tracing::warn!("User {} not found in database, clearing session", user_id);
            if let Some(session) = depot.session_mut() {
                session.remove("user_id");
                session.remove("username");
            }
            return Err(AppError::Unauthorized("Session expired - please log in again".to_string()));
        }
    };

    let user = User {
        id: {
            let id: Uuid = row.get("id");
            id.to_string()
        },
        client_id: {
            let cid: Uuid = row.get("client_id");
            cid.to_string()
        },
        username: row.get("username"),
        password: row.get("password"),
    };
    
    tracing::info!("User found: {}, checking setup status", user.username);

    // Check if setup is complete for this user:
    // 1. Client must exist and be active
    // 2. User must have at least one project
    let client_uuid = Uuid::parse_str(&user.client_id)
        .map_err(|e| {
            tracing::error!("Failed to parse client_id '{}': {}", user.client_id, e);
            AppError::InternalServerError("Invalid client ID".to_string())
        })?;
    
    tracing::info!("Checking client status for client_id: {}", client_uuid);
    
    let client_check = sqlx::query(
        "SELECT status FROM clients WHERE id = $1"
    )
    .bind(&client_uuid)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query client status: {}", e);
        AppError::InternalServerError(format!("Database error: {}", e))
    })?;
    
    let client_is_active = client_check
        .map(|row| {
            let status: String = row.get("status");
            tracing::info!("Client status: {}", status);
            status == "active"
        })
        .unwrap_or_else(|| {
            tracing::warn!("Client not found for id: {}", client_uuid);
            false
        });
    
    // Check if user has at least one project
    tracing::info!("Checking project count for client_id: {}", client_uuid);
    let project_count = sqlx::query(
        "SELECT COUNT(*) as count FROM projects WHERE client_id = $1"
    )
    .bind(&client_uuid)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query project count: {}", e);
        AppError::InternalServerError(format!("Database error: {}", e))
    })?;
    
    let has_projects: i64 = project_count.get("count");
    tracing::info!("User has {} projects", has_projects);
    let is_setup_complete = client_is_active && has_projects > 0;

    res.render(Json(MeResponse {
        user: UserResponse::from(user),
        is_setup_complete,
    }));
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct RegistrationStatusResponse {
    pub registration_enabled: bool,
    pub require_invite_code: bool,
    pub client_name: String,
}

#[handler]
pub async fn check_registration_status(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let client_id = req.query::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("client_id query parameter required".to_string()))?;

    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;

    // Parse client_id as UUID
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID".to_string()))?;

    // Get client info and config
    let client_row = sqlx::query("SELECT name, config FROM clients WHERE id = $1")
        .bind(&client_uuid)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let client_row = client_row.ok_or_else(|| AppError::BadRequest("Client not found".to_string()))?;

    let client_name: String = client_row.get("name");
    let config: serde_json::Value = client_row.get("config");

    let registration_enabled = config.get("registration_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let require_invite_code = config.get("require_invite_code")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    res.render(Json(RegistrationStatusResponse {
        registration_enabled,
        require_invite_code,
        client_name,
    }));
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct PublicClient {
    pub id: String,
    pub name: String,
}

#[handler]
pub async fn list_public_clients(depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;

    // Get all active clients with valid tokens
    let rows = sqlx::query("SELECT id, name FROM clients WHERE status = 'active' AND claude_token IS NOT NULL")
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let clients: Vec<PublicClient> = rows.iter().map(|row| {
        let id: Uuid = row.get("id");
        PublicClient {
            id: id.to_string(),
            name: row.get("name"),
        }
    }).collect();

    res.render(Json(clients));
    Ok(())
}

pub fn auth_routes() -> Router {
    Router::new()
        .push(Router::with_path("/register").post(register))
        .push(Router::with_path("/login").post(login))
        .push(Router::with_path("/logout").post(logout))
        .push(Router::with_path("/me").get(me))
        .push(Router::with_path("/registration-status").get(check_registration_status))
        .push(Router::with_path("/clients").get(list_public_clients))
        .push(Router::with_path("/clients/all").get(list_all_clients))
        .push(Router::with_path("/users/exists").get(check_users_exist))
}

#[handler]
pub async fn check_users_exist(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    // Get client_id from query params
    let client_id = req.query::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("client_id parameter required".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    // Check if any users exist for this client
    let row = sqlx::query("SELECT COUNT(*) as count FROM users WHERE client_id = $1")
        .bind(&client_uuid)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let user_count: i64 = row.get("count");
    let users_exist = user_count > 0;
    
    res.render(Json(serde_json::json!({
        "users_exist": users_exist,
        "user_count": user_count
    })));
    Ok(())
}

#[handler]
pub async fn list_all_clients(depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;

    // Get all clients (including incomplete ones) for setup flow
    let rows = sqlx::query("SELECT id, name, status, claude_token FROM clients ORDER BY created_at ASC")
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let clients: Vec<serde_json::Value> = rows.iter().map(|row| {
        let id: Uuid = row.get("id");
        let claude_token: Option<String> = row.get("claude_token");
        serde_json::json!({
            "id": id.to_string(),
            "name": row.get::<String, _>("name"),
            "status": row.get::<String, _>("status"),
            "has_token": claude_token.is_some()
        })
    }).collect();

    res.render(Json(clients));
    Ok(())
}