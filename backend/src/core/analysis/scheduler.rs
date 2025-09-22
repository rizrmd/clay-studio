use anyhow::Result;
use chrono::{DateTime, Utc};
use cron::Schedule;
use serde_json::Value;
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use super::job_manager::JobManager;

pub struct AnalysisScheduler {
    db: PgPool,
    job_manager: Arc<JobManager>,
    is_running: Arc<tokio::sync::RwLock<bool>>,
}

impl AnalysisScheduler {
    pub async fn new(db: PgPool, job_manager: Arc<JobManager>) -> Self {
        Self {
            db,
            job_manager,
            is_running: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        {
            let mut is_running = self.is_running.write().await;
            if *is_running {
                return Ok(());
            }
            *is_running = true;
        }

        let db = self.db.clone();
        let job_manager = Arc::clone(&self.job_manager);
        let is_running = Arc::clone(&self.is_running);

        tokio::spawn(async move {
            let mut interval_timer = interval(Duration::from_secs(60)); // Check every minute

            loop {
                interval_timer.tick().await;
                
                {
                    let running = is_running.read().await;
                    if !*running {
                        break;
                    }
                }

                if let Err(e) = Self::check_and_run_scheduled_analyses(&db, &job_manager).await {
                    tracing::error!("Error checking scheduled analyses: {}", e);
                }
            }

            tracing::info!("Analysis scheduler stopped");
        });

        tracing::info!("Analysis scheduler started");
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        Ok(())
    }

    async fn check_and_run_scheduled_analyses(
        db: &PgPool,
        job_manager: &Arc<JobManager>,
    ) -> Result<()> {
        let now = Utc::now();

        // Get all enabled schedules that should run
        let schedules = sqlx::query!(
            r#"
            SELECT s.id, s.analysis_id, s.cron_expression, s.timezone, s.last_run_at,
                   a.title, a.script_content
            FROM analysis_schedules s
            JOIN analyses a ON s.analysis_id = a.id
            WHERE s.enabled = true 
              AND a.is_active = true
              AND (s.next_run_at IS NULL OR s.next_run_at <= $1)
            "#,
            now
        )
        .fetch_all(db)
        .await?;

        for schedule in schedules {
            match Self::should_run_now(&schedule.cron_expression, &schedule.timezone, schedule.last_run_at) {
                Ok(should_run) => {
                    if should_run {
                        tracing::info!("Running scheduled analysis: {}", schedule.title);
                        
                        // Get default parameters for scheduled run
                        let default_params = Self::get_default_parameters(&schedule.script_content).await;
                        
                        // Execute the analysis
                        match job_manager.execute_analysis(
                            schedule.analysis_id,
                            default_params,
                            "schedule".to_string(),
                        ).await {
                            Ok(job_id) => {
                                tracing::info!("Started scheduled job {} for analysis {}", job_id, schedule.analysis_id);
                                
                                // Update last run time and calculate next run
                                if let Ok(next_run) = Self::calculate_next_run(&schedule.cron_expression, &schedule.timezone) {
                                    let _ = sqlx::query!(
                                        r#"
                                        UPDATE analysis_schedules 
                                        SET last_run_at = $1, next_run_at = $2
                                        WHERE id = $3
                                        "#,
                                        now,
                                        next_run,
                                        schedule.id
                                    )
                                    .execute(db)
                                    .await;
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to start scheduled analysis {}: {}", schedule.analysis_id, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error evaluating schedule for analysis {}: {}", schedule.analysis_id, e);
                }
            }
        }

        Ok(())
    }

    fn should_run_now(
        cron_expression: &str,
        timezone: &str,
        last_run_at: Option<DateTime<Utc>>,
    ) -> Result<bool> {
        let schedule = Schedule::from_str(cron_expression)?;
        let now = Utc::now();

        // Convert to the specified timezone if needed
        let _tz_now = if timezone != "UTC" {
            // For simplicity, we'll assume UTC for now
            // In a full implementation, you'd use chrono-tz
            now
        } else {
            now
        };

        // Find the next scheduled time from the last run (or from an hour ago if no last run)
        let from_time = last_run_at.unwrap_or_else(|| now - chrono::Duration::hours(1));
        
        if let Some(next_run) = schedule.after(&from_time).next() {
            // Should run if the next scheduled time has passed and we haven't run since then
            Ok(next_run <= now && (last_run_at.map_or(true, |last| last < next_run)))
        } else {
            Ok(false)
        }
    }

    fn calculate_next_run(cron_expression: &str, _timezone: &str) -> Result<DateTime<Utc>> {
        let schedule = Schedule::from_str(cron_expression)?;
        let now = Utc::now();

        if let Some(next_run) = schedule.after(&now).next() {
            Ok(next_run)
        } else {
            Err(anyhow::anyhow!("Could not calculate next run time"))
        }
    }

    async fn get_default_parameters(_script_content: &str) -> Value {
        // Parse the script to extract default parameter values
        // For now, return empty object
        // In a full implementation, this would parse the JS to extract defaults
        
        // Handle special default values like "yesterday", "current_month", etc.
        let now = Utc::now();
        let yesterday = (now - chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
        
        serde_json::json!({
            "date": yesterday,
            "mode": "incremental"
        })
    }

    pub async fn update_schedule_next_runs(&self) -> Result<()> {
        let schedules = sqlx::query!(
            r#"
            SELECT id, cron_expression, timezone
            FROM analysis_schedules
            WHERE enabled = true
            "#
        )
        .fetch_all(&self.db)
        .await?;

        for schedule in schedules {
            if let Ok(next_run) = Self::calculate_next_run(&schedule.cron_expression, &schedule.timezone) {
                let _ = sqlx::query!(
                    "UPDATE analysis_schedules SET next_run_at = $1 WHERE id = $2",
                    next_run,
                    schedule.id
                )
                .execute(&self.db)
                .await;
            }
        }

        Ok(())
    }
}