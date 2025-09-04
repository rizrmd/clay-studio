use crate::utils::AppError;
use salvo::prelude::*;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Extract domain from the request headers
/// First tries X-Frontend-Host (for proxied requests), then falls back to Host header
pub fn extract_domain_from_request(req: &Request) -> Option<String> {
    // First try to get the real frontend host from custom header
    if let Some(frontend_host) = req
        .headers()
        .get("x-frontend-host")
        .and_then(|h| h.to_str().ok())
    {
        return Some(frontend_host.to_string());
    }

    // Fall back to regular Host header
    req.headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .map(|host| {
            // Keep the full host including port for domain matching
            host.to_string()
        })
}

/// Check if a client is allowed to serve a specific domain
pub async fn is_client_allowed_for_domain(
    pool: &PgPool,
    client_id: Uuid,
    request_domain: &str,
) -> Result<bool, AppError> {
    // Fetch client's domains from database
    let row = sqlx::query("SELECT domains FROM clients WHERE id = $1")
        .bind(client_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let row = row.ok_or_else(|| AppError::NotFound("Client not found".to_string()))?;

    // Get domains array from the row
    let domains: Option<Vec<String>> = row.get("domains");

    // If domains is NULL or empty, client can serve all domains
    match domains {
        None => Ok(true),
        Some(domains) if domains.is_empty() => Ok(true),
        Some(domains) => {
            // Check if the request domain matches any of the client's domains
            Ok(domains.iter().any(|d| d == request_domain))
        }
    }
}

/// Validate that a client can be accessed from the current domain
pub async fn validate_client_domain(
    pool: &PgPool,
    client_id: Uuid,
    req: &Request,
) -> Result<(), AppError> {
    // Extract domain from request
    let request_domain = extract_domain_from_request(req)
        .ok_or_else(|| AppError::BadRequest("Missing Host header".to_string()))?;

    // Check if client is allowed for this domain
    let is_allowed = is_client_allowed_for_domain(pool, client_id, &request_domain).await?;

    if !is_allowed {
        return Err(AppError::Forbidden(format!(
            "Client {} is not authorized for domain {}",
            client_id, request_domain
        )));
    }

    Ok(())
}
