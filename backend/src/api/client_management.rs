use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use sqlx::Row;

use crate::models::client::{ClientStatus, ClientAdminResponse, ClientRootResponse, ClientUpdateRequest};
use crate::utils::{AppState, AppError};

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateConfigRequest {
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateDomainsRequest {
    pub domains: Vec<String>,
}

// Admin endpoints (read-only, accessible to admin and root)

#[handler]
pub async fn list_clients_admin(depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    let role = depot.get::<String>("current_user_role")
        .map_err(|_| AppError::InternalServerError("Failed to get role from depot".to_string()))?;
    
    let user_client_id = depot.get::<String>("current_user_client_id")
        .ok();  // Convert Result to Option
    
    // Build query based on role
    let query = if role.as_str() == "admin" {
        // Admin can only see their own client
        let client_id_str = user_client_id.ok_or_else(|| AppError::InternalServerError("Client ID not found for admin".to_string()))?;
        let client_id = Uuid::parse_str(client_id_str.as_str()).map_err(|_| AppError::InternalServerError("Invalid client ID".to_string()))?;
        
        sqlx::query(
            r#"
            SELECT 
                c.id, c.name, c.description, c.status, c.install_path, c.domains,
                c.created_at, c.updated_at,
                (SELECT COUNT(*) FROM users WHERE client_id = c.id) as user_count,
                (SELECT COUNT(*) FROM projects WHERE client_id = c.id) as project_count
            FROM clients c
            WHERE c.id = $1 AND c.deleted_at IS NULL
            "#
        ).bind(client_id)
    } else {
        // Root can see all clients
        sqlx::query(
            r#"
            SELECT 
                c.id, c.name, c.description, c.status, c.install_path, c.domains,
                c.created_at, c.updated_at,
                (SELECT COUNT(*) FROM users WHERE client_id = c.id) as user_count,
                (SELECT COUNT(*) FROM projects WHERE client_id = c.id) as project_count
            FROM clients c
            WHERE c.deleted_at IS NULL
            ORDER BY c.created_at DESC
            "#
        )
    };
    
    let rows = query
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let clients: Vec<ClientAdminResponse> = rows.iter().map(|row| {
        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "pending" => ClientStatus::Pending,
            "installing" => ClientStatus::Installing,
            "active" => ClientStatus::Active,
            "error" => ClientStatus::Error,
            _ => ClientStatus::Pending,
        };
        
        ClientAdminResponse {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            status,
            install_path: row.get("install_path"),
            domains: row.get("domains"),
            user_count: row.get("user_count"),
            project_count: row.get("project_count"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }).collect();
    
    res.render(Json(clients));
    Ok(())
}

#[handler]
pub async fn get_client_admin(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let client_id = req.param::<String>("id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    let role = depot.get::<String>("current_user_role")
        .map_err(|_| AppError::InternalServerError("Failed to get role from depot".to_string()))?;
    
    // If admin, check if they're accessing their own client
    if role.as_str() == "admin" {
        let user_client_id = depot.get::<String>("current_user_client_id")
            .map_err(|_| AppError::InternalServerError("Failed to get client ID from depot".to_string()))?;
        
        let user_client_uuid = Uuid::parse_str(user_client_id.as_str())
            .map_err(|_| AppError::InternalServerError("Invalid user client ID".to_string()))?;
        
        if client_uuid != user_client_uuid {
            return Err(AppError::Forbidden("You can only view your own client".to_string()));
        }
    }
    
    let row = sqlx::query(
        r#"
        SELECT 
            c.id, c.name, c.description, c.status, c.install_path, c.domains,
            c.created_at, c.updated_at,
            (SELECT COUNT(*) FROM users WHERE client_id = c.id) as user_count,
            (SELECT COUNT(*) FROM projects WHERE client_id = c.id) as project_count
        FROM clients c
        WHERE c.id = $1 AND c.deleted_at IS NULL
        "#
    )
    .bind(client_uuid)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let row = row.ok_or_else(|| AppError::NotFound("Client not found".to_string()))?;
    
    let status_str: String = row.get("status");
    let status = match status_str.as_str() {
        "pending" => ClientStatus::Pending,
        "installing" => ClientStatus::Installing,
        "active" => ClientStatus::Active,
        "error" => ClientStatus::Error,
        _ => ClientStatus::Pending,
    };
    
    let client = ClientAdminResponse {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        status,
        install_path: row.get("install_path"),
        domains: row.get("domains"),
        user_count: row.get("user_count"),
        project_count: row.get("project_count"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    };
    
    res.render(Json(client));
    Ok(())
}

// Root-only endpoints (full access)

#[handler]
pub async fn list_clients_root(depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    let rows = sqlx::query(
        r#"
        SELECT 
            c.id, c.name, c.description, c.status, c.install_path, c.domains,
            c.config, c.claude_token, c.created_at, c.updated_at, c.deleted_at,
            (SELECT COUNT(*) FROM users WHERE client_id = c.id) as user_count,
            (SELECT COUNT(*) FROM projects WHERE client_id = c.id) as project_count,
            (SELECT COUNT(*) FROM conversations WHERE project_id IN 
                (SELECT id FROM projects WHERE client_id = c.id)) as conversation_count
        FROM clients c
        ORDER BY c.created_at DESC
        "#
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let clients: Vec<ClientRootResponse> = rows.iter().map(|row| {
        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "pending" => ClientStatus::Pending,
            "installing" => ClientStatus::Installing,
            "active" => ClientStatus::Active,
            "error" => ClientStatus::Error,
            _ => ClientStatus::Pending,
        };
        
        let claude_token: Option<String> = row.get("claude_token");
        
        ClientRootResponse {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            status,
            install_path: row.get("install_path"),
            domains: row.get("domains"),
            config: row.get("config"),
            has_claude_token: claude_token.is_some(),
            user_count: row.get("user_count"),
            project_count: row.get("project_count"),
            conversation_count: row.get("conversation_count"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            deleted_at: row.get("deleted_at"),
        }
    }).collect();
    
    res.render(Json(clients));
    Ok(())
}

#[handler]
pub async fn get_client_root(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let client_id = req.param::<String>("id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    let row = sqlx::query(
        r#"
        SELECT 
            c.id, c.name, c.description, c.status, c.install_path, c.domains,
            c.config, c.claude_token, c.created_at, c.updated_at, c.deleted_at,
            (SELECT COUNT(*) FROM users WHERE client_id = c.id) as user_count,
            (SELECT COUNT(*) FROM projects WHERE client_id = c.id) as project_count,
            (SELECT COUNT(*) FROM conversations WHERE project_id IN 
                (SELECT id FROM projects WHERE client_id = c.id)) as conversation_count
        FROM clients c
        WHERE c.id = $1
        "#
    )
    .bind(client_uuid)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let row = row.ok_or_else(|| AppError::NotFound("Client not found".to_string()))?;
    
    let status_str: String = row.get("status");
    let status = match status_str.as_str() {
        "pending" => ClientStatus::Pending,
        "installing" => ClientStatus::Installing,
        "active" => ClientStatus::Active,
        "error" => ClientStatus::Error,
        _ => ClientStatus::Pending,
    };
    
    let claude_token: Option<String> = row.get("claude_token");
    
    let client = ClientRootResponse {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        status,
        install_path: row.get("install_path"),
        domains: row.get("domains"),
        config: row.get("config"),
        has_claude_token: claude_token.is_some(),
        user_count: row.get("user_count"),
        project_count: row.get("project_count"),
        conversation_count: row.get("conversation_count"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        deleted_at: row.get("deleted_at"),
    };
    
    res.render(Json(client));
    Ok(())
}

#[handler]
pub async fn update_client(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    tracing::info!("update_client endpoint hit");
    
    let client_id = req.param::<String>("id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;
    
    tracing::info!("Client ID from params: {}", client_id);
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    let update_req: ClientUpdateRequest = req.parse_json().await
        .map_err(|e| {
            tracing::error!("Failed to parse JSON: {:?}", e);
            AppError::BadRequest("Invalid JSON".to_string())
        })?;
    
    tracing::info!("Update request parsed: {:?}", update_req);
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    // Build dynamic update query
    let mut updates = Vec::new();
    let mut param_count = 1;
    
    if let Some(ref _name) = update_req.name {
        param_count += 1;
        updates.push(format!("name = ${}", param_count));
    }
    
    if let Some(ref _description) = update_req.description {
        param_count += 1;
        updates.push(format!("description = ${}", param_count));
    }
    
    if let Some(ref _domains) = update_req.domains {
        param_count += 1;
        updates.push(format!("domains = ${}", param_count));
    }
    
    if updates.is_empty() {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }
    
    param_count += 1;
    updates.push(format!("updated_at = ${}", param_count));
    
    let query_str = format!(
        "UPDATE clients SET {} WHERE id = $1 AND deleted_at IS NULL RETURNING id",
        updates.join(", ")
    );
    
    let mut query = sqlx::query(&query_str);
    query = query.bind(client_uuid);
    
    // Bind other parameters based on what was provided
    if let Some(name) = update_req.name {
        query = query.bind(name);
    }
    if let Some(description) = update_req.description {
        query = query.bind(description);
    }
    if let Some(domains) = update_req.domains {
        query = query.bind(serde_json::to_value(domains).unwrap());
    }
    query = query.bind(Utc::now());
    
    let result = query
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    if result.is_none() {
        return Err(AppError::NotFound("Client not found or already deleted".to_string()));
    }
    
    res.render(Json(serde_json::json!({
        "message": "Client updated successfully",
        "id": client_id
    })));
    Ok(())
}

#[handler]
pub async fn delete_client(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let client_id = req.param::<String>("id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    // Check if client has any users
    let user_count_row = sqlx::query("SELECT COUNT(*) as count FROM users WHERE client_id = $1")
        .bind(client_uuid)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let user_count: i64 = user_count_row.get("count");
    if user_count > 0 {
        return Err(AppError::BadRequest(format!("Cannot delete client with {} active users", user_count)));
    }
    
    // Soft delete the client
    sqlx::query("UPDATE clients SET deleted_at = $1, updated_at = $1 WHERE id = $2 AND deleted_at IS NULL")
        .bind(Utc::now())
        .bind(client_uuid)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    res.render(Json(serde_json::json!({
        "message": "Client deleted successfully",
        "id": client_id
    })));
    Ok(())
}

#[handler]
pub async fn enable_client(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let client_id = req.param::<String>("id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    sqlx::query("UPDATE clients SET status = 'active', updated_at = $1 WHERE id = $2 AND deleted_at IS NULL")
        .bind(Utc::now())
        .bind(client_uuid)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    res.render(Json(serde_json::json!({
        "message": "Client enabled successfully",
        "id": client_id
    })));
    Ok(())
}

#[handler]
pub async fn disable_client(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let client_id = req.param::<String>("id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    sqlx::query("UPDATE clients SET status = 'error', updated_at = $1 WHERE id = $2 AND deleted_at IS NULL")
        .bind(Utc::now())
        .bind(client_uuid)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    res.render(Json(serde_json::json!({
        "message": "Client disabled successfully",
        "id": client_id
    })));
    Ok(())
}

#[handler]
pub async fn update_client_config(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let client_id = req.param::<String>("id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    let config_req: UpdateConfigRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    sqlx::query("UPDATE clients SET config = $1, updated_at = $2 WHERE id = $3 AND deleted_at IS NULL")
        .bind(config_req.config)
        .bind(Utc::now())
        .bind(client_uuid)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    res.render(Json(serde_json::json!({
        "message": "Client configuration updated successfully",
        "id": client_id
    })));
    Ok(())
}

#[handler]
pub async fn update_client_domains(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let client_id = req.param::<String>("id")
        .ok_or_else(|| AppError::BadRequest("Missing client ID".to_string()))?;
    
    let client_uuid = Uuid::parse_str(&client_id)
        .map_err(|_| AppError::BadRequest("Invalid client ID format".to_string()))?;
    
    let domains_req: UpdateDomainsRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON".to_string()))?;
    
    let state = depot.obtain::<AppState>()
        .map_err(|_| AppError::InternalServerError("Failed to get app state".to_string()))?;
    
    let domains_json = serde_json::to_value(domains_req.domains)
        .map_err(|e| AppError::InternalServerError(format!("Failed to serialize domains: {}", e)))?;
    
    sqlx::query("UPDATE clients SET domains = $1, updated_at = $2 WHERE id = $3 AND deleted_at IS NULL")
        .bind(domains_json)
        .bind(Utc::now())
        .bind(client_uuid)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    res.render(Json(serde_json::json!({
        "message": "Client domains updated successfully",
        "id": client_id
    })));
    Ok(())
}

// Route builders
pub fn admin_routes() -> Router {
    Router::new()
        .push(Router::with_path("clients").get(list_clients_admin))
        .push(Router::with_path("clients/{id}").get(get_client_admin))
}

pub fn root_routes() -> Router {
    Router::new()
        .get(list_clients_root)
        .push(Router::with_path("{id}")
            .get(get_client_root)
            .put(update_client)
            .delete(delete_client))
        .push(Router::with_path("{id}/enable").post(enable_client))
        .push(Router::with_path("{id}/disable").post(disable_client))
        .push(Router::with_path("{id}/config").put(update_client_config))
        .push(Router::with_path("{id}/domains").put(update_client_domains))
}