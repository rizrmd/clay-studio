use anyhow::{Result, Context};
use serde_json::Value;
use sqlx::types::Uuid;
use sqlx::PgPool;
use tokio::task;
use quickjs_runtime::builder::QuickJsRuntimeBuilder;
use quickjs_runtime::jsutils::Script;

#[derive(Clone)]
pub struct AnalysisService {
    db_pool: PgPool,
}

impl AnalysisService {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
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

        // Get the analysis script
        let analysis = sqlx::query!(
            "SELECT script_content FROM analyses WHERE id = $1",
            analysis_id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        if analysis.is_none() {
            self.update_job_status(job_id, "failed", None, Some("Analysis not found".to_string())).await?;
            return Ok(());
        }

        let script_content = analysis.unwrap().script_content;

        // Execute the JavaScript
        match self.execute_javascript(&script_content, &parameters).await {
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
                let error_msg = format!("Script execution failed: {:?}", e);
                eprintln!("Job {} failed: {}", job_id, error_msg);
                self.update_job_status(job_id, "failed", None, Some(error_msg.clone())).await?;
            }
        }

        Ok(())
    }

    async fn execute_javascript(&self, script: &str, parameters: &Value) -> Result<Value> {
        // Execute in a separate blocking task to avoid blocking the async runtime
        let script_owned = script.to_string();
        let parameters_owned = parameters.clone();

        task::spawn_blocking(move || {
            // Create QuickJS runtime
            let runtime = QuickJsRuntimeBuilder::new()
                .build();

            // Remove ES6 export syntax and async keywords since QuickJS makes this complicated
            let processed_script = script_owned
                .replace("export default async function", "function main")
                .replace("export default function", "function main")
                .replace("export default", "const main =")
                .replace("async run(", "run(")  // Remove async from method definitions
                .replace("async function(", "function(");

            // Prepare the script with parameters injected (synchronous execution)
            let wrapped_script = format!(
                r#"
                (function() {{
                    const parameters = {};

                    // Execute the script
                    {};

                    // Get the result
                    let result;
                    if (typeof main === 'function') {{
                        // main is a function, call it directly
                        result = main(parameters);
                    }} else if (typeof main === 'object' && typeof main.run === 'function') {{
                        // main is an object with a run method
                        result = main.run(parameters);
                    }} else if (typeof main !== 'undefined') {{
                        // main exists but isn't callable
                        result = main;
                    }} else {{
                        result = {{ __executed: true }};
                    }}

                    // Convert to JSON string for Rust to parse
                    return JSON.stringify(result);
                }})()
                "#,
                serde_json::to_string(&parameters_owned).unwrap_or_else(|_| "{}".to_string()),
                processed_script
            );

            // Execute the script synchronously
            let script_obj = Script::new("analysis.js", &wrapped_script);

            let result = runtime.eval_sync(None, script_obj)
                .context("Failed to evaluate script")?;

            // The result is now a JSON string, parse it
            let json_result = if result.is_null_or_undefined() {
                serde_json::json!({
                    "success": true,
                    "data": null,
                    "message": "Script executed successfully but returned no data"
                })
            } else if result.is_string() {
                // Parse the JSON string returned by the script
                let json_str = result.get_str();
                match serde_json::from_str::<Value>(json_str) {
                    Ok(parsed_data) => {
                        serde_json::json!({
                            "success": true,
                            "data": parsed_data,
                            "message": "Script executed successfully"
                        })
                    }
                    Err(e) => {
                        serde_json::json!({
                            "success": false,
                            "data": null,
                            "message": format!("Failed to parse result as JSON: {}", e)
                        })
                    }
                }
            } else {
                serde_json::json!({
                    "success": false,
                    "data": null,
                    "message": "Script did not return a JSON string"
                })
            };

            Ok(json_result)
        }).await?
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