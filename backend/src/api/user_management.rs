use bcrypt::{hash, DEFAULT_COST};
use chrono::{DateTime, Utc};
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::models::user::UserRole;
use crate::utils::{AppError, AppState};

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
pub async fn list_users_for_client(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let client_id = req
        .param::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;

    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;

    let state = depot
        .obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;

    let rows = sqlx::query(
        r#"
        SELECT 
            u.id, u.client_id, u.username, u.role,
            'active' as status,
            NULL as last_active
        FROM users u
        WHERE u.client_id = $1
        ORDER BY u.id DESC
        "#,
    )
    .bind(client_uuid)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let users: Vec<UserResponse> = rows
        .iter()
        .map(|row| {
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
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        })
        .collect();

    res.render(Json(users));
    Ok(())
}

// Get a specific user (Root-only)
#[handler]
pub async fn get_user(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let client_id = req
        .param::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;

    let user_id = req
        .param::<String>("user_id")
        .ok_or_else(|| AppError::BadRequest("Missing user ID".to_string()))?;

    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;

    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".to_string()))?;

    let state = depot
        .obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;

    let row = sqlx::query(
        r#"
        SELECT 
            u.id, u.client_id, u.username, u.role,
            'active' as status,
            NULL as last_active
        FROM users u
        WHERE u.id = $1 AND u.client_id = $2
        "#,
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
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    res.render(Json(user));
    Ok(())
}

// Create a new user for a specific client (Root-only)
#[handler]
pub async fn create_user_for_client(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let client_id = req
        .param::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;

    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;

    let create_req: CreateUserRequest = req
        .parse_json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid JSON".to_string()))?;

    let state = depot
        .obtain::<AppState>()
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
        return Err(AppError::BadRequest(
            "Username already exists for this client".to_string(),
        ));
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
pub async fn update_user(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let client_id = req
        .param::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;

    let user_id = req
        .param::<String>("user_id")
        .ok_or_else(|| AppError::BadRequest("Missing user ID".to_string()))?;

    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;

    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".to_string()))?;

    let update_req: UpdateUserRequest = req
        .parse_json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid JSON".to_string()))?;

    let state = depot
        .obtain::<AppState>()
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
pub async fn delete_user(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let client_id = req
        .param::<String>("client_id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;

    let user_id = req
        .param::<String>("user_id")
        .ok_or_else(|| AppError::BadRequest("Missing user ID".to_string()))?;

    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;

    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".to_string()))?;

    let state = depot
        .obtain::<AppState>()
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
            "SELECT COUNT(*) FROM users WHERE client_id = $1 AND role IN ('admin', 'root')",
        )
        .bind(client_uuid)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

        if admin_count <= 1 {
            return Err(AppError::BadRequest(
                "Cannot delete the last admin user".to_string(),
            ));
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

// Admin routes (for managing users within their own client)
#[handler]
pub async fn list_users_admin(depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = depot
        .obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;

    let user_client_id = depot.get::<String>("current_user_client_id").map_err(|_| {
        AppError::InternalServerError("Failed to get client ID from depot".to_string())
    })?;
    let client_id = Uuid::parse_str(user_client_id)
        .map_err(|_| AppError::InternalServerError("Invalid client ID format".to_string()))?;

    let rows = sqlx::query(
        r#"
        SELECT 
            u.id, u.client_id, u.username, u.role,
            'active' as status,
            NULL as last_active
        FROM users u
        WHERE u.client_id = $1
        ORDER BY u.id DESC
        "#,
    )
    .bind(client_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let users: Vec<UserResponse> = rows
        .iter()
        .map(|row| {
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
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        })
        .collect();

    res.render(Json(users));
    Ok(())
}

#[handler]
pub async fn create_user_admin(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let create_req: CreateUserRequest = req
        .parse_json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid JSON".to_string()))?;

    let state = depot
        .obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;

    let user_client_id = depot.get::<String>("current_user_client_id").map_err(|_| {
        AppError::InternalServerError("Failed to get client ID from depot".to_string())
    })?;
    let client_id = Uuid::parse_str(user_client_id)
        .map_err(|_| AppError::InternalServerError("Invalid client ID format".to_string()))?;

    let user_role = depot
        .get::<String>("current_user_role")
        .map_err(|_| AppError::InternalServerError("Failed to get role from depot".to_string()))?;

    // Check if username already exists for this client
    let existing_user = sqlx::query("SELECT id FROM users WHERE username = $1 AND client_id = $2")
        .bind(&create_req.username)
        .bind(client_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    if existing_user.is_some() {
        return Err(AppError::BadRequest(
            "Username already exists for this client".to_string(),
        ));
    }

    // Hash password
    let hashed_password = hash(&create_req.password, DEFAULT_COST)
        .map_err(|e| AppError::InternalServerError(format!("Failed to hash password: {}", e)))?;

    let user_id = Uuid::new_v4();
    let role = create_req.role.unwrap_or(UserRole::User);

    // Admins can only create users with 'user' role, not other admins
    let role_str = if user_role == "admin" && role != UserRole::User {
        "user"
    } else {
        match role {
            UserRole::User => "user",
            UserRole::Admin => "admin",
            UserRole::Root => "root",
        }
    };

    let now = Utc::now();

    // Create user
    sqlx::query(
        "INSERT INTO users (id, client_id, username, password, role, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind(user_id)
    .bind(client_id)
    .bind(&create_req.username)
    .bind(&hashed_password)
    .bind(role_str)
    .bind(now)
    .bind(now)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to create user: {}", e)))?;

    let user_response = UserResponse {
        id: user_id,
        client_id,
        username: create_req.username,
        role: if role_str == "user" {
            UserRole::User
        } else {
            role
        },
        status: UserStatus::Active,
        last_active: None,
        created_at: now,
        updated_at: now,
    };

    res.render(Json(user_response));
    Ok(())
}

#[handler]
pub async fn update_user_admin(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let user_id = req
        .param::<String>("user_id")
        .ok_or_else(|| AppError::BadRequest("Missing user ID".to_string()))?;

    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".to_string()))?;

    let update_req: UpdateUserRequest = req
        .parse_json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid JSON".to_string()))?;

    let state = depot
        .obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;

    let user_client_id = depot.get::<String>("current_user_client_id").map_err(|_| {
        AppError::InternalServerError("Failed to get client ID from depot".to_string())
    })?;
    let client_id = Uuid::parse_str(user_client_id)
        .map_err(|_| AppError::InternalServerError("Invalid client ID format".to_string()))?;

    let user_role = depot
        .get::<String>("current_user_role")
        .map_err(|_| AppError::InternalServerError("Failed to get role from depot".to_string()))?;

    let current_user_id = depot.get::<String>("current_user_id").map_err(|_| {
        AppError::InternalServerError("Failed to get user ID from depot".to_string())
    })?;
    let current_user_uuid = Uuid::parse_str(current_user_id)
        .map_err(|_| AppError::InternalServerError("Invalid user ID format".to_string()))?;

    // Verify the user being updated belongs to the same client
    let target_user = sqlx::query("SELECT client_id, role FROM users WHERE id = $1")
        .bind(user_uuid)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let target_user =
        target_user.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    let target_client_id: Uuid = target_user.get("client_id");
    let target_role: String = target_user.get("role");

    if target_client_id != client_id {
        return Err(AppError::Forbidden(
            "Cannot update users from other clients".to_string(),
        ));
    }

    // Admins cannot modify other admins or root users
    if user_role == "admin"
        && (target_role == "admin" || target_role == "root")
        && user_uuid != current_user_uuid
    {
        return Err(AppError::Forbidden(
            "Cannot modify other administrators".to_string(),
        ));
    }

    // Build dynamic update query
    let mut updates = Vec::new();
    let mut param_count = 1; // user_id is $1

    if let Some(ref _username) = update_req.username {
        param_count += 1;
        updates.push(format!("username = ${}", param_count));
    }

    if let Some(ref role) = update_req.role {
        // Admins can only set role to 'user', not promote to admin
        if user_role == "admin" && *role != UserRole::User {
            return Err(AppError::Forbidden(
                "Cannot promote users to admin role".to_string(),
            ));
        }
        param_count += 1;
        updates.push(format!("role = ${}", param_count));
    }

    if updates.is_empty() {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    param_count += 1;
    updates.push(format!("updated_at = ${}", param_count));

    let query_str = format!(
        "UPDATE users SET {} WHERE id = $1 RETURNING id",
        updates.join(", ")
    );

    let mut query = sqlx::query(&query_str);
    query = query.bind(user_uuid);

    // Bind other parameters based on what was provided
    if let Some(username) = update_req.username {
        query = query.bind(username);
    }
    if let Some(role) = update_req.role {
        let role_str = match role {
            UserRole::User => "user",
            UserRole::Admin => {
                if user_role == "admin" {
                    "user"
                } else {
                    "admin"
                }
            }
            UserRole::Root => "root",
        };
        query = query.bind(role_str);
    }
    query = query.bind(Utc::now());

    query
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    res.render(Json(serde_json::json!({
        "message": "User updated successfully",
        "id": user_id
    })));
    Ok(())
}

#[handler]
pub async fn delete_user_admin(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let user_id = req
        .param::<String>("user_id")
        .ok_or_else(|| AppError::BadRequest("Missing user ID".to_string()))?;

    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID format".to_string()))?;

    let state = depot
        .obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;

    let user_client_id = depot.get::<String>("current_user_client_id").map_err(|_| {
        AppError::InternalServerError("Failed to get client ID from depot".to_string())
    })?;
    let client_id = Uuid::parse_str(user_client_id)
        .map_err(|_| AppError::InternalServerError("Invalid client ID format".to_string()))?;

    let user_role = depot
        .get::<String>("current_user_role")
        .map_err(|_| AppError::InternalServerError("Failed to get role from depot".to_string()))?;

    let current_user_id = depot.get::<String>("current_user_id").map_err(|_| {
        AppError::InternalServerError("Failed to get user ID from depot".to_string())
    })?;
    let current_user_uuid = Uuid::parse_str(current_user_id)
        .map_err(|_| AppError::InternalServerError("Invalid user ID format".to_string()))?;

    // Check if user exists and belongs to the same client
    let user_row = sqlx::query("SELECT role, client_id FROM users WHERE id = $1")
        .bind(user_uuid)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let user_row = user_row.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    let user_role_str: String = user_row.get("role");
    let user_client_id: Uuid = user_row.get("client_id");

    if user_client_id != client_id {
        return Err(AppError::Forbidden(
            "Cannot delete users from other clients".to_string(),
        ));
    }

    // Admins cannot delete other admins or root users
    if user_role == "admin" && (user_role_str == "admin" || user_role_str == "root") {
        return Err(AppError::Forbidden(
            "Cannot delete administrator accounts".to_string(),
        ));
    }

    // Prevent deleting self
    if user_uuid == current_user_uuid {
        return Err(AppError::BadRequest(
            "Cannot delete your own account".to_string(),
        ));
    }

    // If trying to delete an admin, check if there are other admins
    if user_role_str == "admin" || user_role_str == "root" {
        let admin_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE client_id = $1 AND role IN ('admin', 'root')",
        )
        .bind(client_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

        if admin_count <= 1 {
            return Err(AppError::BadRequest(
                "Cannot delete the last admin user".to_string(),
            ));
        }
    }

    // Delete user sessions first
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_uuid)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    // Delete user
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_uuid)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    res.render(Json(serde_json::json!({
        "message": "User deleted successfully",
        "id": user_id
    })));
    Ok(())
}

pub fn admin_routes() -> Router {
    Router::new()
        .push(
            Router::with_path("users")
                .get(list_users_admin)
                .post(create_user_admin),
        )
        .push(
            Router::with_path("users/{user_id}")
                .put(update_user_admin)
                .delete(delete_user_admin),
        )
}

// Root routes
pub fn root_routes() -> Router {
    Router::new()
        .push(
            Router::with_path("{client_id}/users")
                .get(list_users_for_client)
                .post(create_user_for_client),
        )
        .push(
            Router::with_path("{client_id}/users/{user_id}")
                .get(get_user)
                .put(update_user)
                .delete(delete_user),
        )
}
