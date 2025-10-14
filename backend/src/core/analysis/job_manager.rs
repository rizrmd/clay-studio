use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

use crate::models::analysis::*;
use super::sandbox::AnalysisSandbox;

pub struct JobManager {
    db: PgPool,
    sandbox: Arc<AnalysisSandbox>,
    running_jobs: Arc<RwLock<HashMap<Uuid, JobHandle>>>,
}

pub struct JobHandle {
    pub job_id: Uuid,
    pub analysis_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub cancel_sender: broadcast::Sender<()>,
}

impl JobManager {
    pub async fn new(db: PgPool, sandbox: Arc<AnalysisSandbox>) -> Self {
        Self {
            db,
            sandbox,
            running_jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn execute_analysis(
        &self,
        analysis_id: Uuid,
        parameters: Value,
        triggered_by: String,
    ) -> Result<Uuid> {
        // Get analysis details
        let analysis = self.get_analysis(analysis_id).await?;
        
        // Create job record
        let job_id = self.create_job(analysis_id, parameters.clone(), triggered_by).await?;
        
        // Start execution in background
        let manager = Arc::new(self.clone());
        let job_analysis = analysis.clone();
        let job_params = parameters.clone();
        
        tokio::spawn(async move {
            if let Err(e) = manager.execute_job_internal(job_id, job_analysis, job_params).await {
                tracing::error!("Job {} failed: {}", job_id, e);
                let _ = manager.mark_job_failed(job_id, &e.to_string()).await;
            }
        });

        Ok(job_id)
    }

    async fn execute_job_internal(
        &self,
        job_id: Uuid,
        analysis: Analysis,
        parameters: Value,
    ) -> Result<()> {
        // Mark job as running
        self.mark_job_running(job_id).await?;

        // Create cancellation channel
        let (cancel_sender, mut cancel_receiver) = broadcast::channel(1);
        
        // Register running job
        {
            let mut running_jobs = self.running_jobs.write().await;
            running_jobs.insert(job_id, JobHandle {
                job_id,
                analysis_id: analysis.id,
                started_at: Utc::now(),
                cancel_sender,
            });
        }

        // Get datasources for the project
        let datasources = self.get_project_datasources(analysis.project_id).await?;

        // Execute analysis in sandbox
        let result = tokio::select! {
            result = self.sandbox.execute_analysis(&analysis, parameters, job_id, datasources) => {
                result
            }
            _ = cancel_receiver.recv() => {
                Err(anyhow!("Job was cancelled"))
            }
        };

        // Clean up running job
        {
            let mut running_jobs = self.running_jobs.write().await;
            running_jobs.remove(&job_id);
        }

        match result {
            Ok(result_value) => {
                self.mark_job_completed(job_id, result_value).await?;
            }
            Err(e) => {
                self.mark_job_failed(job_id, &e.to_string()).await?;
                return Err(e);
            }
        }

        Ok(())
    }

    async fn create_job(
        &self,
        analysis_id: Uuid,
        parameters: Value,
        triggered_by: String,
    ) -> Result<Uuid> {
        let job_id = Uuid::new_v4();
        
        sqlx::query!(
            r#"
            INSERT INTO analysis_jobs (id, analysis_id, status, parameters, triggered_by)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            job_id,
            analysis_id,
            "pending",
            parameters,
            triggered_by
        )
        .execute(&self.db)
        .await?;

        Ok(job_id)
    }

    async fn mark_job_running(&self, job_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE analysis_jobs 
            SET status = $1, started_at = NOW()
            WHERE id = $2
            "#,
            "running",
            job_id
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn mark_job_completed(&self, job_id: Uuid, result: Value) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE analysis_jobs 
            SET status = $1, result = $2, completed_at = NOW(),
                execution_time_ms = EXTRACT(EPOCH FROM (NOW() - started_at)) * 1000
            WHERE id = $3
            "#,
            "completed",
            result,
            job_id
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn mark_job_failed(&self, job_id: Uuid, error_message: &str) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE analysis_jobs 
            SET status = $1, error_message = $2, completed_at = NOW(),
                execution_time_ms = EXTRACT(EPOCH FROM (NOW() - started_at)) * 1000
            WHERE id = $3
            "#,
            "failed",
            error_message,
            job_id
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn get_job_status(&self, job_id: Uuid) -> Result<AnalysisJob> {
        let row = sqlx::query!(
            r#"
            SELECT id, analysis_id, status, parameters, result, error_message, 
                   logs, started_at, completed_at, created_at, execution_time_ms, triggered_by
            FROM analysis_jobs
            WHERE id = $1
            "#,
            job_id
        )
        .fetch_one(&self.db)
        .await?;

        let status = match row.status.as_str() {
            "pending" => JobStatus::Pending,
            "running" => JobStatus::Running,
            "completed" => JobStatus::Completed,
            "failed" => JobStatus::Failed,
            "cancelled" => JobStatus::Cancelled,
            _ => JobStatus::Failed,
        };

        Ok(AnalysisJob {
            id: row.id,
            analysis_id: row.analysis_id,
            status,
            parameters: row.parameters.unwrap_or_default(),
            result: row.result,
            error_message: row.error_message,
            logs: row.logs.unwrap_or_default(),
            started_at: row.started_at.map(|dt| chrono::DateTime::<chrono::Utc>::from_timestamp(dt.unix_timestamp(), 0).unwrap_or_else(|| chrono::Utc::now())),
            completed_at: row.completed_at.map(|dt| chrono::DateTime::<chrono::Utc>::from_timestamp(dt.unix_timestamp(), 0).unwrap_or_else(|| chrono::Utc::now())),
            created_at: row.created_at.map(|dt| chrono::DateTime::<chrono::Utc>::from_timestamp(dt.unix_timestamp(), 0).unwrap_or_else(|| chrono::Utc::now())).unwrap_or_else(|| chrono::Utc::now()),
            execution_time_ms: row.execution_time_ms,
            triggered_by: row.triggered_by.unwrap_or_else(|| "system".to_string()),
        })
    }

    pub async fn cancel_job(&self, job_id: Uuid) -> Result<()> {
        // Cancel running job if it exists
        {
            let running_jobs = self.running_jobs.read().await;
            if let Some(handle) = running_jobs.get(&job_id) {
                let _ = handle.cancel_sender.send(());
            }
        }

        // Update database status
        sqlx::query!(
            r#"
            UPDATE analysis_jobs 
            SET status = $1, completed_at = NOW()
            WHERE id = $2 AND status IN ('pending', 'running')
            "#,
            "cancelled",
            job_id
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn get_running_jobs(&self) -> Vec<Uuid> {
        let running_jobs = self.running_jobs.read().await;
        running_jobs.keys().cloned().collect()
    }

    async fn get_analysis(&self, analysis_id: Uuid) -> Result<Analysis> {
        let row = sqlx::query!(
            r#"
            SELECT id, title, script_content, project_id, created_at, updated_at, 
                   created_by, version, is_active, metadata
            FROM analyses
            WHERE id = $1 AND is_active = true
            "#,
            analysis_id
        )
        .fetch_one(&self.db)
        .await?;

        Ok(Analysis {
            id: row.id,
            title: row.title,
            script_content: row.script_content,
            project_id: uuid::Uuid::parse_str(&row.project_id).unwrap_or_default(),
            created_at: row.created_at.map(|dt| chrono::DateTime::<chrono::Utc>::from_timestamp(dt.unix_timestamp(), 0).unwrap_or_else(|| chrono::Utc::now())).unwrap_or_else(|| chrono::Utc::now()),
            updated_at: row.updated_at.and_then(|dt| chrono::DateTime::<chrono::Utc>::from_timestamp(dt.unix_timestamp(), 0)).unwrap_or_else(|| chrono::Utc::now()),
            created_by: row.created_by.or(Some(uuid::Uuid::nil())),
            version: row.version.unwrap_or(1),
            is_active: row.is_active.unwrap_or(true),
            metadata: row.metadata.unwrap_or_default(),
        })
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
        .fetch_all(&self.db)
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
}

// Make JobManager cloneable for use in async tasks
impl Clone for JobManager {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            sandbox: Arc::clone(&self.sandbox),
            running_jobs: Arc::clone(&self.running_jobs),
        }
    }
}