use sqlx::PgPool;
use uuid::Uuid;
use crate::core::mcp::types::JsonRpcError;
use super::error_responses::ErrorResponses;

/// Common database operation helpers to reduce code duplication
#[allow(dead_code)]
pub struct DatabaseHelpers;

#[allow(dead_code)]
impl DatabaseHelpers {
    /// Verify that a client exists and is not deleted
    pub async fn verify_client_exists(
        pool: &PgPool, 
        client_id: &str
    ) -> Result<(), JsonRpcError> {
        let client_uuid = Uuid::parse_str(client_id)
            .map_err(|e| ErrorResponses::invalid_params(format!("Invalid client ID format: {}", e)))?;
        
        let exists = sqlx::query("SELECT 1 FROM clients WHERE id = $1 AND deleted_at IS NULL")
            .bind(client_uuid)
            .fetch_optional(pool)
            .await
            .map_err(ErrorResponses::database_error)?;

        if exists.is_none() {
            return Err(ErrorResponses::resource_not_found("Client", client_id));
        }

        Ok(())
    }

    /// Verify that a project exists and is not deleted
    pub async fn verify_project_exists(
        pool: &PgPool, 
        project_id: &str
    ) -> Result<(), JsonRpcError> {
        let exists = sqlx::query("SELECT 1 FROM projects WHERE id = $1 AND deleted_at IS NULL")
            .bind(project_id)
            .fetch_optional(pool)
            .await
            .map_err(ErrorResponses::database_error)?;

        if exists.is_none() {
            return Err(ErrorResponses::resource_not_found("Project", project_id));
        }

        Ok(())
    }

    /// Verify that a datasource exists and belongs to the project
    pub async fn verify_datasource_access(
        pool: &PgPool, 
        datasource_id: &str, 
        project_id: &str
    ) -> Result<(), JsonRpcError> {
        let exists = sqlx::query(
            "SELECT 1 FROM data_sources WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL"
        )
        .bind(datasource_id)
        .bind(project_id)
        .fetch_optional(pool)
        .await
        .map_err(ErrorResponses::database_error)?;

        if exists.is_none() {
            return Err(ErrorResponses::resource_not_found("Datasource", datasource_id));
        }

        Ok(())
    }

    /// Get project name by ID
    pub async fn get_project_name(
        pool: &PgPool, 
        project_id: &str
    ) -> Result<String, JsonRpcError> {
        sqlx::query_scalar::<_, String>("SELECT name FROM projects WHERE id = $1")
            .bind(project_id)
            .fetch_one(pool)
            .await
            .map_err(ErrorResponses::database_error)
    }

    /// Check if user has access to project
    pub async fn verify_user_project_access(
        pool: &PgPool,
        user_id: &Uuid,
        project_id: &str
    ) -> Result<(), JsonRpcError> {
        let has_access = sqlx::query(
            r#"
            SELECT 1 FROM projects p 
            WHERE p.id = $1 AND p.user_id = $2 AND p.deleted_at IS NULL
            "#
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(ErrorResponses::database_error)?;

        if has_access.is_none() {
            return Err(ErrorResponses::invalid_params("Access denied to project"));
        }

        Ok(())
    }

    /// Get datasources for a project
    pub async fn get_project_datasources(
        pool: &PgPool,
        project_id: &str
    ) -> Result<Vec<sqlx::postgres::PgRow>, JsonRpcError> {
        sqlx::query(
            "SELECT id, name, source_type, schema_info FROM data_sources WHERE project_id = $1 AND deleted_at IS NULL"
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
        .map_err(ErrorResponses::database_error)
    }

    /// Count records in a table with optional WHERE clause (simplified version)
    pub async fn count_records_simple(
        pool: &PgPool,
        table: &str,
        where_clause: Option<&str>
    ) -> Result<i64, JsonRpcError> {
        let query_str = if let Some(where_clause) = where_clause {
            format!("SELECT COUNT(*) FROM {} WHERE {}", table, where_clause)
        } else {
            format!("SELECT COUNT(*) FROM {}", table)
        };

        sqlx::query_scalar::<_, i64>(&query_str)
            .fetch_one(pool)
            .await
            .map_err(ErrorResponses::database_error)
    }
}