use anyhow::Result;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;
use std::collections::HashMap;

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
            project_id.to_string()
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

    // Note: With Bun runtime, datasource queries are executed within the JavaScript
    // environment. The methods below are kept for backwards compatibility but
    // will need to be reimplemented if direct Rust access is required.
    //
    // For now, analysis scripts running in Bun will handle datasource queries
    // using the ctx.query() API which will be implemented to call back into Rust.
}