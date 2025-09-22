use anyhow::Result;
use serde_json::Value;
use sqlx::types::Uuid;
use sqlx::PgPool;

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
        
        Ok(job_id)
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