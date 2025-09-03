use crate::core::mcp::types::*;
use crate::core::projects::manager::ProjectManager;
use crate::utils::claude_md_template;
use serde_json::json;
use sqlx::{PgPool, Row};
use chrono::Utc;
use uuid;

#[derive(Clone)]
pub struct McpHandlers {
    pub project_id: String,
    #[allow(dead_code)]
    pub client_id: String,
    #[allow(dead_code)]
    pub server_type: String,
    pub db_pool: PgPool,
}

impl McpHandlers {
    /// Refresh CLAUDE.md with current datasource information
    pub async fn refresh_claude_md(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get all datasources for this project
        let data_sources = sqlx::query(
            "SELECT id, name, source_type, schema_info FROM data_sources WHERE project_id = $1 AND deleted_at IS NULL"
        )
        .bind(&self.project_id)
        .fetch_all(&self.db_pool)
        .await?;

        if !data_sources.is_empty() {
            // Get project name
            let project_name = sqlx::query_scalar::<_, String>(
                "SELECT name FROM projects WHERE id = $1"
            )
            .bind(&self.project_id)
            .fetch_one(&self.db_pool)
            .await?;

            // Convert datasources to the format expected by the template
            let datasource_values: Vec<serde_json::Value> = data_sources.iter().map(|ds| {
                json!({
                    "id": ds.get::<String, _>("id"),
                    "name": ds.get::<String, _>("name"),
                    "source_type": ds.get::<String, _>("source_type"),
                    "schema_info": ds.get::<Option<String>, _>("schema_info"),
                })
            }).collect();

            // Generate enhanced CLAUDE.md with datasource information
            let claude_md_content = claude_md_template::generate_claude_md_with_datasources(
                &self.project_id,
                &project_name,
                datasource_values
            ).await;

            // Write to project's CLAUDE.md
            let pm = ProjectManager::new();
            let client_id = uuid::Uuid::parse_str(&self.client_id)
                .map_err(|e| format!("Invalid client ID: {}", e))?;
            pm.save_claude_md_content(client_id, &self.project_id, &claude_md_content)
                .map_err(|e| format!("Failed to save CLAUDE.md: {}", e))?;
        }

        Ok(())
    }

    pub async fn execute_db_operation<F, T>(&self, operation: &str, f: F) -> Result<T, JsonRpcError>
    where
        F: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
    {
        let start_time = std::time::Instant::now();
        
        match f.await {
            Ok(result) => {
                let duration = start_time.elapsed();
                eprintln!(
                    "[{}] [DEBUG] MCP operation '{}' completed successfully in {}ms", 
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                    operation,
                    duration.as_millis()
                );
                Ok(result)
            }
            Err(error) => {
                let duration = start_time.elapsed();
                eprintln!(
                    "[{}] [ERROR] MCP operation '{}' failed after {}ms", 
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                    operation,
                    duration.as_millis()
                );
                Err(self.handle_mcp_error(operation, error))
            }
        }
    }

    pub fn handle_mcp_error(&self, operation: &str, error: Box<dyn std::error::Error + Send + Sync>) -> JsonRpcError {
        eprintln!(
            "[{}] [ERROR] MCP operation '{}' failed: {}", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            operation,
            error
        );
        JsonRpcError {
            code: INTERNAL_ERROR,
            message: error.to_string(),
            data: None,
        }
    }

    pub async fn get_datasource_connector(&self, datasource_id: &str) -> Result<DataSourceInfo, JsonRpcError> {
        let source = sqlx::query(
            "SELECT name, source_type, connection_config 
             FROM data_sources 
             WHERE id = $1 AND project_id = $2"
        )
        .bind(datasource_id)
        .bind(&self.project_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Database error: {}", e),
            data: None,
        })?
        .ok_or_else(|| JsonRpcError {
            code: INVALID_PARAMS,
            message: "Data source not found. The datasource_id does not exist or has been deleted. Use datasource_list to see available data sources.".to_string(),
            data: None,
        })?;
        
        let connection_config_str: String = source.get("connection_config");
        let connection_config = serde_json::from_str(&connection_config_str)
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to parse connection config: {}", e),
                data: None,
            })?;

        Ok(DataSourceInfo {
            name: source.get("name"),
            source_type: source.get("source_type"),
            connection_config,
        })
    }
}