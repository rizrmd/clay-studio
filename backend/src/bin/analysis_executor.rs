// Analysis Executor - Dedicated service for running analysis jobs
// Works with all database types through the existing datasource infrastructure

use anyhow::Result;
use salvo::prelude::*;
use salvo::conn::tcp::TcpListener;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::env;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, error};
use uuid::Uuid;

// Simple error type for this binary
#[derive(Debug)]
pub struct AppError(String);

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AppError {}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError(s)
    }
}

#[async_trait::async_trait]
impl Writer for AppError {
    async fn write(mut self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        res.render(Text::Plain(self.0));
    }
}

// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub jobs_processed: u64,
}

// Analysis job structure
#[derive(Debug, Clone)]
pub struct AnalysisJobData {
    pub id: Uuid,
    pub analysis_id: Uuid,
    pub script_content: String,
    pub parameters: serde_json::Value,
    pub project_id: Uuid,
}

#[derive(Clone)]
pub struct AnalysisExecutor {
    db_pool: PgPool,
    jobs_processed: Arc<std::sync::atomic::AtomicU64>,
}

impl AnalysisExecutor {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool,
            jobs_processed: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    // Start the job polling loop
    pub async fn start_job_poller(&self) {
        let executor = self.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = executor.poll_and_execute_jobs().await {
                    error!("Error in job poller: {}", e);
                }
                sleep(Duration::from_millis(1000)).await;
            }
        });
    }

    // Poll for pending jobs and execute them
    async fn poll_and_execute_jobs(&self) -> Result<()> {
        // Get pending jobs with their analysis scripts
        let jobs = sqlx::query!(
            r#"
            SELECT aj.id, aj.analysis_id, aj.parameters, a.script_content, a.project_id
            FROM analysis_jobs aj
            JOIN analyses a ON aj.analysis_id = a.id
            WHERE aj.status = 'pending'
            ORDER BY aj.created_at ASC
            LIMIT 5
            "#,
        )
        .fetch_all(&self.db_pool)
        .await?;

        if jobs.is_empty() {
            return Ok(());
        }

        info!("Found {} pending jobs", jobs.len());

        for job_row in jobs {
            let job_id = job_row.id;
            
            // Mark job as running first
            sqlx::query!(
                "UPDATE analysis_jobs SET status = $1, started_at = NOW() WHERE id = $2",
                "running",
                job_id
            )
            .execute(&self.db_pool)
            .await?;

            // Convert to our job structure
            let job = AnalysisJobData {
                id: job_row.id,
                analysis_id: job_row.analysis_id,
                script_content: job_row.script_content,
                parameters: job_row.parameters.unwrap_or_else(|| serde_json::json!({})),
                project_id: uuid::Uuid::parse_str(&job_row.project_id)
                    .unwrap_or_else(|_| uuid::Uuid::new_v4()),
            };

            // Execute job in background
            let executor = self.clone();
            tokio::spawn(async move {
                match executor.execute_job(job).await {
                    Ok(_) => {
                        executor.jobs_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        info!("Job {} completed successfully", job_id);
                    }
                    Err(e) => {
                        error!("Job {} failed: {}", job_id, e);
                        let _ = executor.mark_job_failed(job_id, &e.to_string()).await;
                    }
                }
            });
        }

        Ok(())
    }

    async fn execute_job(&self, job: AnalysisJobData) -> Result<()> {
        info!("Executing job {} for analysis {}", job.id, job.analysis_id);

        // TODO: In a full implementation, this would:
        // 1. Get datasource connections for the project using the datasource service
        // 2. Create a QuickJS sandbox with access to those datasources
        // 3. Execute the JavaScript analysis script
        // 4. Return structured results
        
        // For now, just validate the script format and return mock result
        let result = if job.script_content.contains("export default") {
            serde_json::json!({
                "status": "success",
                "data": {
                    "message": "Analysis executed successfully (mock)",
                    "script_length": job.script_content.len(),
                    "parameters_count": job.parameters.as_object().map_or(0, |o| o.len()),
                    "execution_time_ms": 150
                }
            })
        } else {
            return Err(anyhow::anyhow!("Invalid script format: must export default"));
        };

        // Mark job as completed
        sqlx::query!(
            "UPDATE analysis_jobs SET status = $1, result = $2, completed_at = NOW() WHERE id = $3",
            "completed",
            result,
            job.id
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    async fn mark_job_failed(&self, job_id: Uuid, error: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE analysis_jobs SET status = $1, error_message = $2, completed_at = NOW() WHERE id = $3",
            "failed",
            error,
            job_id
        )
        .execute(&self.db_pool)
        .await?;
        Ok(())
    }

    pub fn get_jobs_processed(&self) -> u64 {
        self.jobs_processed.load(std::sync::atomic::Ordering::Relaxed)
    }
}

// HTTP Handlers
#[handler]
async fn health_check_simple() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        jobs_processed: 0, // Simplified for now
    })
}

#[handler]
async fn health_check(depot: &mut Depot) -> Json<HealthResponse> {
    let executor = match depot.obtain::<Arc<AnalysisExecutor>>() {
        Ok(exec) => exec,
        Err(_) => {
            return Json(HealthResponse {
                status: "unhealthy".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                jobs_processed: 0,
            });
        }
    };
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        jobs_processed: executor.get_jobs_processed(),
    })
}

#[handler]
async fn submit_job(_req: &mut Request) -> Json<serde_json::Value> {
    // For future use when implementing direct job submission
    Json(serde_json::json!({
        "message": "Job submission via database polling only",
        "status": "use_database_polling"
    }))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Get configuration
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost:5432/clay_studio".to_string());
    let port: u16 = env::var("ANALYSIS_EXECUTOR_PORT")
        .unwrap_or_else(|_| "8002".to_string())
        .parse()
        .unwrap_or(8002);

    // Connect to database
    info!("Analysis Executor starting...");
    info!("Connecting to database: {}", database_url);
    let db_pool = PgPool::connect(&database_url).await?;

    // Test database connection
    sqlx::query!("SELECT 1 as test")
        .fetch_one(&db_pool)
        .await?;
    info!("Database connection successful");

    // Create executor
    let executor = Arc::new(AnalysisExecutor::new(db_pool));

    // Start job poller
    executor.start_job_poller().await;
    info!("Job poller started");

    // Configure HTTP server (simplified without middleware for now)
    let app = Router::new()
        .push(Router::with_path("/health").get(health_check_simple))
        .push(Router::with_path("/jobs").post(submit_job));

    // Start server
    info!("Starting HTTP server on port {}", port);
    let listener = TcpListener::new(format!("0.0.0.0:{}", port)).bind().await;
    Server::new(listener).serve(app).await;

    Ok(())
}