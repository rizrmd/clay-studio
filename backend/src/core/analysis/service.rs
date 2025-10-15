use anyhow::Result;
use serde_json::Value;
use sqlx::types::Uuid;
use sqlx::PgPool;
use tokio::task;
use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;

use super::bun_runtime::BunRuntime;

#[derive(Clone)]
pub struct AnalysisService {
    db_pool: PgPool,
    bun_runtime: Arc<BunRuntime>,
}

impl AnalysisService {
    pub fn new(db_pool: PgPool, clients_dir: PathBuf) -> Result<Self> {
        let bun_runtime = Arc::new(
            BunRuntime::new(clients_dir)?
                .with_db_pool(db_pool.clone())
        );
        Ok(Self {
            db_pool,
            bun_runtime,
        })
    }

    pub async fn submit_analysis_job(&self, analysis_id: Uuid, parameters: Value, _trigger_type: String) -> Result<Uuid> {
        let job_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO analysis_jobs (id, analysis_id, parameters, status, created_at)
            VALUES ($1, $2, $3, 'pending', NOW())
            "#,
            job_id,
            analysis_id,
            parameters
        )
        .execute(&self.db_pool)
        .await?;

        // Spawn async task to execute the job
        let service = self.clone();
        task::spawn(async move {
            if let Err(e) = service.execute_job(job_id, analysis_id, parameters).await {
                eprintln!("Error executing job {}: {:?}", job_id, e);
            }
        });

        Ok(job_id)
    }

    async fn execute_job(&self, job_id: Uuid, analysis_id: Uuid, parameters: Value) -> Result<()> {
        // Update status to running
        sqlx::query!(
            "UPDATE analysis_jobs SET status = 'running', started_at = NOW() WHERE id = $1",
            job_id
        )
        .execute(&self.db_pool)
        .await?;

        // Get the analysis script and project_id
        let analysis = sqlx::query!(
            "SELECT script_content, project_id FROM analyses WHERE id = $1",
            analysis_id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        if analysis.is_none() {
            self.update_job_status(job_id, "failed", None, Some("Analysis not found".to_string())).await?;
            return Ok(());
        }

        let analysis_row = analysis.unwrap();
        let script_content = analysis_row.script_content;
        let project_id = Uuid::parse_str(&analysis_row.project_id)?;

        // Get datasources for the project
        let datasources = self.get_project_datasources(project_id).await?;

        // Build context
        let context = serde_json::json!({
            "datasources": datasources,
            "metadata": {},
        });

        // Get backend URL from environment
        let backend_url = std::env::var("BACKEND_URL").ok();

        // Generate auth token for this job
        let auth_token = Some(format!("analysis-job-{}", job_id));

        // Execute using Bun runtime
        let config = crate::core::analysis::bun_runtime::AnalysisConfig {
            script_content,
            parameters,
            context,
            backend_url,
            auth_token,
        };

        match self.bun_runtime.execute_analysis(
            project_id,
            job_id,
            config,
        ).await {
            Ok(result) => {
                // Update job with completed status
                sqlx::query!(
                    r#"
                    UPDATE analysis_jobs
                    SET status = 'completed',
                        result = $1,
                        completed_at = NOW(),
                        execution_time_ms = EXTRACT(EPOCH FROM (NOW() - started_at)) * 1000
                    WHERE id = $2
                    "#,
                    result,
                    job_id
                )
                .execute(&self.db_pool)
                .await?;
            }
            Err(e) => {
                let error_msg = format!("Script execution failed: {}", e);
                tracing::error!("Job {} failed: {}", job_id, error_msg);
                self.update_job_status(job_id, "failed", None, Some(error_msg)).await?;
            }
        }

        Ok(())
    }

    async fn get_project_datasources(&self, project_id: Uuid) -> Result<HashMap<String, Value>> {
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
                "name": row.name.clone(),
                "type": row.source_type,
                "config": row.config
            });
            datasources.insert(row.name, datasource_info);
        }

        Ok(datasources)
    }

    pub async fn create_job(&self, analysis_id: Uuid, parameters: Value) -> Result<Uuid> {
        let job_id = Uuid::new_v4();
        
        sqlx::query!(
            r#"
            INSERT INTO analysis_jobs (id, analysis_id, parameters, status, created_at)
            VALUES ($1, $2, $3, 'pending', NOW())
            "#,
            job_id,
            analysis_id,
            parameters
        )
        .execute(&self.db_pool)
        .await?;
        
        Ok(job_id)
    }

    pub async fn update_job_status(&self, job_id: Uuid, status: &str, result: Option<Value>, error_message: Option<String>) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE analysis_jobs 
            SET status = $1, result = $2, error_message = $3
            WHERE id = $4
            "#,
            status,
            result,
            error_message,
            job_id
        )
        .execute(&self.db_pool)
        .await?;
        
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn get_job_result(&self, job_id: Uuid) -> Result<Option<Value>> {
        let row = sqlx::query!(
            "SELECT status, result, error_message FROM analysis_jobs WHERE id = $1",
            job_id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        match row {
            Some(row) => match row.status.as_str() {
                "completed" => Ok(row.result),
                "failed" => Ok(Some(serde_json::json!({
                    "error": row.error_message.unwrap_or("Unknown error".to_string())
                }))),
                _ => Ok(Some(serde_json::json!({
                    "status": row.status
                })))
            },
            None => Ok(None)
        }
    }
}