use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::analysis::*;
use super::{bun_runtime::BunRuntime, duckdb_manager::DuckDBManager, datasource_service::AnalysisDatasourceService};

pub struct AnalysisSandbox {
    bun_runtime: Arc<BunRuntime>,
    duckdb: Arc<DuckDBManager>,
    datasource_service: AnalysisDatasourceService,
    db_pool: sqlx::PgPool,
}

impl AnalysisSandbox {
    pub async fn new(data_dir: PathBuf, db_pool: sqlx::PgPool) -> Result<Self> {
        let duckdb = Arc::new(DuckDBManager::new(data_dir.clone()).await?);
        duckdb.ensure_database_dir().await?;

        // Initialize BunRuntime with .clients directory and db_pool for MCP
        let clients_dir = data_dir.parent()
            .ok_or_else(|| anyhow!("Invalid data directory"))?
            .join(".clients");
        let bun_runtime = Arc::new(
            BunRuntime::new(clients_dir)?
                .with_db_pool(db_pool.clone())
        );

        let datasource_service = AnalysisDatasourceService::new(db_pool.clone());

        Ok(Self {
            bun_runtime,
            duckdb,
            datasource_service,
            db_pool
        })
    }

    pub async fn execute_analysis(
        &self,
        analysis: &Analysis,
        parameters: Value,
        job_id: Uuid,
        datasources: HashMap<String, Value>,
    ) -> Result<Value> {
        // Build context for the analysis
        let context = serde_json::json!({
            "datasources": datasources,
            "metadata": {},
        });

        // Get backend URL from environment or use default
        let backend_url = std::env::var("BACKEND_URL").ok();

        // Generate auth token for this job
        // TODO: Use proper JWT or session token
        let auth_token = Some(format!("analysis-job-{}", job_id));

        // Execute using Bun runtime
        let config = crate::core::analysis::bun_runtime::AnalysisConfig {
            script_content: analysis.script_content.clone(),
            parameters,
            context,
            backend_url,
            auth_token,
        };

        self.bun_runtime.execute_analysis(
            analysis.project_id,
            job_id,
            config,
        ).await
    }

    pub async fn validate_analysis(&self, script_content: &str) -> Result<ValidationResult> {
        // Use a temporary project ID for validation
        let temp_project_id = Uuid::new_v4();

        let validation_errors = self.bun_runtime
            .validate_script(temp_project_id, script_content)
            .await?;

        if validation_errors.is_empty() {
            Ok(ValidationResult {
                valid: true,
                errors: Vec::new(),
                metadata: None,
            })
        } else {
            Ok(ValidationResult {
                valid: false,
                errors: validation_errors,
                metadata: None,
            })
        }
    }

    pub async fn get_parameter_options(
        &self,
        _analysis: &Analysis,
        _parameter_name: &str,
        _current_params: Value,
    ) -> Result<Vec<ParameterOption>> {
        // This would execute the parameter's options function in the sandbox
        // For now, return empty options
        Ok(vec![])
    }

    /// Ensure project analysis directory exists and dependencies are installed
    pub async fn ensure_project_setup(&self, project_id: Uuid) -> Result<()> {
        self.bun_runtime.get_project_analysis_dir(project_id).await?;
        self.bun_runtime.install_dependencies(project_id).await?;
        Ok(())
    }
}