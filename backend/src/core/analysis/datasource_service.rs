use anyhow::Result;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;
use std::collections::HashMap;

use crate::utils::datasource::{get_pool_manager, create_connector};
use crate::models::data_source::DataSource;

/// Service that provides datasource access for analysis scripts
/// using the existing connection pooling infrastructure
#[derive(Clone)]
pub struct AnalysisDatasourceService {
    db_pool: PgPool,
}

impl AnalysisDatasourceService {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Get datasource configs for a project
    pub async fn get_project_datasources(&self, project_id: Uuid) -> Result<HashMap<String, Value>> {
        let rows = sqlx::query!(
            r#"
            SELECT name, source_type, connection_config as config
            FROM data_sources
            WHERE project_id = $1 AND deleted_at IS NULL
            "#,
            project_id
        )
        .fetch_all(&self.db_pool)
        .await?;

        let mut datasources = HashMap::new();
        for row in rows {
            let datasource_info = serde_json::json!({
                "name": row.name,
                "type": row.source_type,
                "config": row.config
            });
            datasources.insert(row.name, datasource_info);
        }

        Ok(datasources)
    }

    /// Execute a query on a datasource using connection pooling
    pub async fn execute_datasource_query(
        &self,
        datasource_name: &str,
        project_id: Uuid,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Value> {
        // Get datasource config
        let datasource = sqlx::query!(
            r#"
            SELECT source_type, connection_config
            FROM data_sources
            WHERE project_id = $1 AND name = $2 AND deleted_at IS NULL
            "#,
            project_id,
            datasource_name
        )
        .fetch_one(&self.db_pool)
        .await?;

        // Create connector using factory
        let connector = create_connector(&datasource.source_type, &datasource.connection_config)
            .await?;

        // Execute query with limit
        let limit = limit.unwrap_or(10000);
        let result = connector.execute_query(query, limit).await?;

        Ok(result)
    }

    /// Get schema information for a datasource
    pub async fn get_datasource_schema(
        &self,
        datasource_name: &str,
        project_id: Uuid,
    ) -> Result<Value> {
        // Get datasource config
        let datasource = sqlx::query!(
            r#"
            SELECT source_type, connection_config
            FROM data_sources
            WHERE project_id = $1 AND name = $2 AND deleted_at IS NULL
            "#,
            project_id,
            datasource_name
        )
        .fetch_one(&self.db_pool)
        .await?;

        // Create connector
        let connector = create_connector(&datasource.source_type, &datasource.connection_config)
            .await?;

        // Get schema
        let schema = connector.get_schema().await?;

        Ok(schema)
    }

    /// Test a datasource connection
    pub async fn test_datasource_connection(
        &self,
        datasource_name: &str,
        project_id: Uuid,
    ) -> Result<bool> {
        // Get datasource config
        let datasource = sqlx::query!(
            r#"
            SELECT source_type, connection_config
            FROM data_sources
            WHERE project_id = $1 AND name = $2 AND deleted_at IS NULL
            "#,
            project_id,
            datasource_name
        )
        .fetch_one(&self.db_pool)
        .await?;

        // Create connector
        let connector = create_connector(&datasource.source_type, &datasource.connection_config)
            .await?;

        // Test connection
        connector.test_connection().await
    }
}