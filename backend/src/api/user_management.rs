use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use bcrypt::{hash, DEFAULT_COST};
use sqlx::Row;

use crate::models::user::UserRole;
use crate::utils::{AppState, AppError};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub client_id: Uuid,
    pub username: String,
    pub role: UserRole,
    pub status: UserStatus,
    pub last_active: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    Active,
    Suspended,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: Option<UserRole>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub role: Option<UserRole>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangePasswordRequest {
    pub password: String,
}

// List users for a specific client (Root-only)
#[handler]
pub async fn list_users_for_client(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let client_id = req.param::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    let rows = sqlx::query(
        r#"
        SELECT 
            u.id, u.client_id, u.username, u.role, u.created_at, u.updated_at,
            'active' as status,
            l.last_login as last_active
        FROM users u
        LEFT JOIN (
            SELECT user_id, MAX(created_at) as last_login
            FROM sessions 
            GROUP BY user_id
        ) l ON u.id = l.user_id
        WHERE u.client_id = $1
        ORDER BY u.created_at DESC
        "#
    )
    .bind(client_uuid)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let users: Vec<UserResponse> = rows.iter().map(|row| {
        let role_str: String = row.get("role");
        let role = match role_str.as_str() {
            "user" => UserRole::User,
            "admin" => UserRole::Admin,
            "root" => UserRole::Root,
            _ => UserRole::User,
        };
        
        UserResponse {
            id: row.get("id"),
            client_id: row.get("client_id"),
            username: row.get("username"),
            role,
            status: UserStatus::Active,
            last_active: row.get("last_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }).collect();
    
    res.render(Json(users));
    Ok(())
}

// Get a specific user (Root-only)
#[handler]
pub async fn get_user(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let client_id = req.param::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;
    
    let user_id = req.param::<String>("user_id")
        .ok_or_else(|| AppError::BadRequest("Missing user ID".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    let row = sqlx::query(
        r#"
        SELECT 
            u.id, u.client_id, u.username, u.role, u.created_at, u.updated_at,
            'active' as status,
            l.last_login as last_active
        FROM users u
        LEFT JOIN (
            SELECT user_id, MAX(created_at) as last_login
            FROM sessions 
            GROUP BY user_id
        ) l ON u.id = l.user_id
        WHERE u.id = $1 AND u.client_id = $2
        "#
    )
    .bind(user_uuid)
    .bind(client_uuid)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let row = row.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    
    let role_str: String = row.get("role");
    let role = match role_str.as_str() {
        "user" => UserRole::User,
        "admin" => UserRole::Admin,
        "root" => UserRole::Root,
        _ => UserRole::User,
    };
    
    let user = UserResponse {
        id: row.get("id"),
        client_id: row.get("client_id"),
        username: row.get("username"),
        role,
        status: UserStatus::Active,
        last_active: row.get("last_active"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    };
    
    res.render(Json(user));
    Ok(())
}

// Create a new user for a specific client (Root-only)
#[handler]
pub async fn create_user_for_client(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let client_id = req.param::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    let create_req: CreateUserRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    // Check if client exists
    let client_exists = sqlx::query("SELECT id FROM clients WHERE id = $1 AND deleted_at IS NULL")
        .bind(client_uuid)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    if client_exists.is_none() {
        return Err(AppError::NotFound("Client not found".to_string()));
    }
    
    // Check if username already exists for this client
    let existing_user = sqlx::query("SELECT id FROM users WHERE username = $1 AND client_id = $2")
        .bind(&create_req.username)
        .bind(client_uuid)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    if existing_user.is_some() {
        return Err(AppError::BadRequest("Username already exists for this client".to_string()));
    }
    
    // Hash password
    let hashed_password = hash(&create_req.password, DEFAULT_COST)
        .map_err(|e| AppError::InternalServerError(format!("Failed to hash password: {}", e)))?;
    
    let user_id = Uuid::new_v4();
    let role = create_req.role.unwrap_or(UserRole::User);
    let role_str = match role {
        UserRole::User => "user",
        UserRole::Admin => "admin",
        UserRole::Root => "root",
    };
    
    let now = Utc::now();
    
    // Create user
    sqlx::query(
        "INSERT INTO users (id, client_id, username, password, role, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind(user_id)
    .bind(client_uuid)
    .bind(&create_req.username)
    .bind(&hashed_password)
    .bind(role_str)
    .bind(now)
    .bind(now)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to create user: {}", e)))?;
    
    let user = UserResponse {
        id: user_id,
        client_id: client_uuid,
        username: create_req.username,
        role,
        status: UserStatus::Active,
        last_active: None,
        created_at: now,
        updated_at: now,
    };
    
    res.render(Json(user));
    Ok(())
}

// Update user (Root-only)
#[handler]
pub async fn update_user(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let client_id = req.param::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;
    
    let user_id = req.param::<String>("user_id")
        .ok_or_else(|| AppError::BadRequest("Missing user ID".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".to_string()))?;
    
    let update_req: UpdateUserRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    // Build dynamic update query
    let mut updates = Vec::new();
    let mut param_count = 2; // client_id and user_id are $1 and $2
    
    if let Some(ref _username) = update_req.username {
        param_count += 1;
        updates.push(format!("username = ${}", param_count));
    }
    
    if let Some(ref _role) = update_req.role {
        param_count += 1;
        updates.push(format!("role = ${}", param_count));
    }
    
    if updates.is_empty() {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }
    
    param_count += 1;
    updates.push(format!("updated_at = ${}", param_count));
    
    let query_str = format!(
        "UPDATE users SET {} WHERE id = $1 AND client_id = $2 RETURNING id",
        updates.join(", ")
    );
    
    let mut query = sqlx::query(&query_str);
    query = query.bind(user_uuid).bind(client_uuid);
    
    // Bind other parameters based on what was provided
    if let Some(username) = update_req.username {
        query = query.bind(username);
    }
    if let Some(role) = update_req.role {
        let role_str = match role {
            UserRole::User => "user",
            UserRole::Admin => "admin",
            UserRole::Root => "root",
        };
        query = query.bind(role_str);
    }
    query = query.bind(Utc::now());
    
    let result = query
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    if result.is_none() {
        return Err(AppError::NotFound("User not found".to_string()));
    }
    
    res.render(Json(serde_json::json!({
        "message": "User updated successfully",
        "id": user_id
    })));
    Ok(())
}

// Delete user (Root-only)
#[handler]
pub async fn delete_user(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let client_id = req.param::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;
    
    let user_id = req.param::<String>("user_id")
        .ok_or_else(|| AppError::BadRequest("Missing user ID".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    // Check if user exists and is not the only admin
    let user_row = sqlx::query("SELECT role FROM users WHERE id = $1 AND client_id = $2")
        .bind(user_uuid)
        .bind(client_uuid)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let user_row = user_row.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    let user_role: String = user_row.get("role");
    
    // If trying to delete an admin, check if there are other admins
    if user_role == "admin" || user_role == "root" {
        let admin_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE client_id = $1 AND role IN ('admin', 'root')"
        )
        .bind(client_uuid)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
        
        if admin_count <= 1 {
            return Err(AppError::BadRequest("Cannot delete the last admin user".to_string()));
        }
    }
    
    // Delete user sessions first
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_uuid)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    // Delete user
    sqlx::query("DELETE FROM users WHERE id = $1 AND client_id = $2")
        .bind(user_uuid)
        .bind(client_uuid)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    res.render(Json(serde_json::json!({
        "message": "User deleted successfully",
        "id": user_id
    })));
    Ok(())
}

// Root routes
pub fn root_routes() -> Router {
    Router::new()
        .push(Router::with_path("{client_id}/users")
            .get(list_users_for_client)
            .post(create_user_for_client))
        .push(Router::with_path("{client_id}/users/{user_id}")
            .get(get_user)
            .put(update_user)
            .delete(delete_user))
}