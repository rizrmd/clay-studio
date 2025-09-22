use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::analysis::*;
use super::{job_manager::JobManager, sandbox::AnalysisSandbox};

pub struct AnalysisManager {
    db: PgPool,
    job_manager: Arc<JobManager>,
    sandbox: Arc<AnalysisSandbox>,
}

impl AnalysisManager {
    pub async fn new(db: PgPool, sandbox: Arc<AnalysisSandbox>) -> Self {
        let job_manager = Arc::new(JobManager::new(db.clone(), Arc::clone(&sandbox)).await);
        
        Self {
            db,
            job_manager,
            sandbox,
        }
    }

    // Analysis CRUD operations
    pub async fn create_analysis(&self, request: CreateAnalysisRequest) -> Result<Analysis> {
        // Validate the script first
        let validation = self.sandbox.validate_analysis(&request.script_content).await?;
        if !validation.valid {
            return Err(anyhow!("Script validation failed: {:?}", validation.errors));
        }

        let analysis_id = Uuid::new_v4();
        let now = Utc::now();

        // Create analysis
        sqlx::query(
            r#"
            INSERT INTO analyses (id, title, script_content, project_id, created_at, updated_at, version, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#
        )
        .bind(analysis_id)
        .bind(&request.title)
        .bind(&request.script_content)
        .bind(request.project_id)
        .bind(now)
        .bind(now)
        .bind(1)
        .bind(serde_json::json!({}))
        .execute(&self.db)
        .await?;

        // Create first version entry
        self.create_version(analysis_id, 1, &request.script_content, Some("Initial version")).await?;

        // Extract and save dependencies
        self.update_dependencies(analysis_id, &request.script_content).await?;

        self.get_analysis(analysis_id).await
    }

    pub async fn get_analysis(&self, analysis_id: Uuid) -> Result<Analysis> {
        let row = sqlx::query(
            r#"
            SELECT id, title, script_content, project_id, created_at, updated_at, 
                   created_by, version, is_active, metadata
            FROM analyses
            WHERE id = $1 AND is_active = true
            "#
        )
        .bind(analysis_id)
        .fetch_one(&self.db)
        .await?;

        Ok(Analysis {
            id: row.get("id"),
            title: row.get("title"),
            script_content: row.get("script_content"),
            project_id: row.get("project_id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            created_by: row.get("created_by"),
            version: row.get("version"),
            is_active: row.get("is_active"),
            metadata: row.get("metadata"),
        })
    }

    pub async fn list_analyses(&self, project_id: Option<Uuid>) -> Result<Vec<AnalysisMetadata>> {
        let mut query_str = r#"
            SELECT a.id, a.title, a.created_at, a.metadata,
                   j.completed_at as last_run
            FROM analyses a
            LEFT JOIN analysis_jobs j ON a.id = j.analysis_id 
                AND j.status = 'completed'
                AND j.completed_at = (
                    SELECT MAX(completed_at) 
                    FROM analysis_jobs 
                    WHERE analysis_id = a.id AND status = 'completed'
                )
            WHERE a.is_active = true
        "#.to_string();

        let rows = if let Some(pid) = project_id {
            query_str.push_str(" AND a.project_id = $1 ORDER BY a.updated_at DESC");
            sqlx::query(&query_str)
                .bind(pid)
                .fetch_all(&self.db)
                .await?
        } else {
            query_str.push_str(" ORDER BY a.updated_at DESC");
            sqlx::query(&query_str)
                .fetch_all(&self.db)
                .await?
        };

        let mut analyses = Vec::new();
        for row in rows {
            let id: Uuid = row.get("id");
            let title: String = row.get("title");
            let created_at: DateTime<Utc> = row.get("created_at");
            let last_run: Option<DateTime<Utc>> = row.get("last_run");

            // Get dependencies
            let dependencies = self.get_analysis_dependencies(id).await?;
            
            // TODO: Parse parameters from script metadata
            let parameters = HashMap::new();

            // Get schedule if exists
            let schedule = self.get_analysis_schedule(id).await.ok();

            analyses.push(AnalysisMetadata {
                id: id.to_string(),
                title,
                parameters,
                dependencies,
                schedule,
                created_at,
                last_run,
            });
        }

        Ok(analyses)
    }

    pub async fn update_analysis(&self, analysis_id: Uuid, request: UpdateAnalysisRequest) -> Result<Analysis> {
        let mut current_analysis = self.get_analysis(analysis_id).await?;
        
        // Apply updates
        if let Some(title) = &request.title {
            current_analysis.title = title.clone();
        }
        
        if let Some(script_content) = &request.script_content {
            // Validate new script
            let validation = self.sandbox.validate_analysis(script_content).await?;
            if !validation.valid {
                return Err(anyhow!("Script validation failed: {:?}", validation.errors));
            }
            
            current_analysis.script_content = script_content.clone();
            current_analysis.version += 1;
            
            // Create new version
            self.create_version(
                analysis_id, 
                current_analysis.version, 
                script_content,
                request.change_description.as_deref()
            ).await?;
            
            // Update dependencies
            self.update_dependencies(analysis_id, script_content).await?;
        }

        // Update database
        sqlx::query!(
            r#"
            UPDATE analyses 
            SET title = $1, script_content = $2, version = $3, updated_at = NOW()
            WHERE id = $4
            "#,
            current_analysis.title,
            current_analysis.script_content,
            current_analysis.version,
            analysis_id
        )
        .execute(&self.db)
        .await?;

        self.get_analysis(analysis_id).await
    }

    pub async fn delete_analysis(&self, analysis_id: Uuid) -> Result<()> {
        // Cancel any running jobs
        let running_jobs = sqlx::query!(
            "SELECT id FROM analysis_jobs WHERE analysis_id = $1 AND status = 'running'",
            analysis_id
        )
        .fetch_all(&self.db)
        .await?;

        for job in running_jobs {
            let _ = self.job_manager.cancel_job(job.id).await;
        }

        // Soft delete the analysis
        sqlx::query!(
            "UPDATE analyses SET is_active = false WHERE id = $1",
            analysis_id
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    // Execution operations
    pub async fn execute_analysis(&self, analysis_id: Uuid, parameters: Value) -> Result<Uuid> {
        self.job_manager.execute_analysis(analysis_id, parameters, "manual".to_string()).await
    }

    pub async fn get_job_status(&self, job_id: Uuid) -> Result<AnalysisJob> {
        self.job_manager.get_job_status(job_id).await
    }

    pub async fn cancel_job(&self, job_id: Uuid) -> Result<()> {
        self.job_manager.cancel_job(job_id).await
    }

    // Parameter operations
    pub async fn get_parameter_options(
        &self,
        analysis_id: Uuid,
        parameter_name: String,
        current_params: Value,
    ) -> Result<Vec<ParameterOption>> {
        let analysis = self.get_analysis(analysis_id).await?;
        self.sandbox.get_parameter_options(&analysis, &parameter_name, current_params).await
    }

    // Version operations
    pub async fn get_analysis_versions(&self, analysis_id: Uuid) -> Result<Vec<AnalysisVersion>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, analysis_id, version_number, script_content, change_description,
                   created_at, created_by, metadata
            FROM analysis_versions
            WHERE analysis_id = $1
            ORDER BY version_number DESC
            "#,
            analysis_id
        )
        .fetch_all(&self.db)
        .await?;

        let versions = rows.into_iter().map(|row| AnalysisVersion {
            id: row.id,
            analysis_id: row.analysis_id,
            version_number: row.version_number,
            script_content: row.script_content,
            change_description: row.change_description,
            created_at: row.created_at,
            created_by: row.created_by,
            metadata: row.metadata,
        }).collect();

        Ok(versions)
    }

    pub async fn get_analysis_version(&self, analysis_id: Uuid, version_number: i32) -> Result<AnalysisVersion> {
        let row = sqlx::query!(
            r#"
            SELECT id, analysis_id, version_number, script_content, change_description,
                   created_at, created_by, metadata
            FROM analysis_versions
            WHERE analysis_id = $1 AND version_number = $2
            "#,
            analysis_id,
            version_number
        )
        .fetch_one(&self.db)
        .await?;

        Ok(AnalysisVersion {
            id: row.id,
            analysis_id: row.analysis_id,
            version_number: row.version_number,
            script_content: row.script_content,
            change_description: row.change_description,
            created_at: row.created_at,
            created_by: row.created_by,
            metadata: row.metadata,
        })
    }

    // Schedule operations
    pub async fn set_schedule(&self, analysis_id: Uuid, schedule: ScheduleConfig) -> Result<()> {
        // Delete existing schedule
        sqlx::query!(
            "DELETE FROM analysis_schedules WHERE analysis_id = $1",
            analysis_id
        )
        .execute(&self.db)
        .await?;

        if schedule.enabled {
            // Create new schedule
            sqlx::query!(
                r#"
                INSERT INTO analysis_schedules (id, analysis_id, cron_expression, timezone, enabled)
                VALUES ($1, $2, $3, $4, $5)
                "#,
                Uuid::new_v4(),
                analysis_id,
                schedule.cron,
                schedule.timezone.unwrap_or_else(|| "UTC".to_string()),
                schedule.enabled
            )
            .execute(&self.db)
            .await?;
        }

        Ok(())
    }

    // Private helper methods
    async fn create_version(
        &self,
        analysis_id: Uuid,
        version_number: i32,
        script_content: &str,
        change_description: Option<&str>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO analysis_versions (id, analysis_id, version_number, script_content, change_description)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            Uuid::new_v4(),
            analysis_id,
            version_number,
            script_content,
            change_description
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn update_dependencies(&self, analysis_id: Uuid, _script_content: &str) -> Result<()> {
        // Delete existing dependencies
        sqlx::query!(
            "DELETE FROM analysis_dependencies WHERE analysis_id = $1",
            analysis_id
        )
        .execute(&self.db)
        .await?;

        // TODO: Parse script to extract dependencies and insert them
        // This would require proper JS parsing to extract the dependencies object

        Ok(())
    }

    async fn get_analysis_dependencies(&self, analysis_id: Uuid) -> Result<AnalysisDependencies> {
        let rows = sqlx::query!(
            r#"
            SELECT dependency_type, dependency_name
            FROM analysis_dependencies
            WHERE analysis_id = $1
            "#,
            analysis_id
        )
        .fetch_all(&self.db)
        .await?;

        let mut datasources = Vec::new();
        let mut analyses = Vec::new();

        for row in rows {
            match row.dependency_type.as_str() {
                "datasource" => datasources.push(row.dependency_name),
                "analysis" => analyses.push(row.dependency_name),
                _ => {}
            }
        }

        Ok(AnalysisDependencies {
            datasources,
            analyses,
        })
    }

    async fn get_analysis_schedule(&self, analysis_id: Uuid) -> Result<ScheduleConfig> {
        let row = sqlx::query!(
            r#"
            SELECT cron_expression, timezone, enabled
            FROM analysis_schedules
            WHERE analysis_id = $1
            "#,
            analysis_id
        )
        .fetch_one(&self.db)
        .await?;

        Ok(ScheduleConfig {
            cron: row.cron_expression,
            timezone: Some(row.timezone),
            enabled: row.enabled,
        })
    }

    pub async fn validate_analysis(&self, script_content: &str) -> Result<ValidationResult> {
        self.sandbox.validate_analysis(script_content).await
    }
}