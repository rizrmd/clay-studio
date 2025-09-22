use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::types::time;

use crate::utils::error::AppError;
use crate::utils::get_app_state;

// Helper function to convert OffsetDateTime to chrono DateTime
fn convert_time_to_chrono(dt: time::OffsetDateTime) -> DateTime<Utc> {
    DateTime::from_timestamp(dt.unix_timestamp(), dt.nanosecond()).unwrap_or_else(chrono::Utc::now)
}

// Helper function to convert chrono DateTime to OffsetDateTime
fn convert_chrono_to_time(dt: DateTime<Utc>) -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp(dt.timestamp()).unwrap_or_else(|_| time::OffsetDateTime::now_utc())
}

// Request/Response DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAnalysisRequest {
    pub title: String,
    pub script_content: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAnalysisRequest {
    pub title: Option<String>,
    pub script_content: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, Value>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisResponse {
    pub id: Uuid,
    pub title: String,
    pub script_content: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, Value>,
    pub project_id: String,
    pub is_active: bool,
    pub version: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteAnalysisRequest {
    pub parameters: Option<HashMap<String, Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobResponse {
    pub job_id: Uuid,
    pub analysis_id: Uuid,
    pub status: String,
    pub parameters: HashMap<String, Value>,
    pub result: Option<Value>,
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationRequest {
    pub script_content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResponse {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub metadata: Option<HashMap<String, Value>>,
}

// Job Management DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct JobListResponse {
    pub jobs: Vec<JobResponse>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobStatusResponse {
    pub job_id: Uuid,
    pub analysis_id: Uuid,
    pub status: String,
    pub progress: Option<f32>,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub execution_time_ms: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CancelJobRequest {
    pub reason: Option<String>,
}

// Scheduling DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateScheduleRequest {
    pub cron_expression: String,
    pub timezone: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateScheduleRequest {
    pub cron_expression: Option<String>,
    pub timezone: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleResponse {
    pub id: Uuid,
    pub analysis_id: Uuid,
    pub cron_expression: String,
    pub timezone: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
}

// Results DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisResultSummary {
    pub analysis_id: Uuid,
    pub analysis_title: String,
    pub total_jobs: i64,
    pub successful_jobs: i64,
    pub failed_jobs: i64,
    pub average_execution_time_ms: Option<f64>,
    pub last_success: Option<DateTime<Utc>>,
    pub last_failure: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisResultsList {
    pub results: Vec<JobResponse>,
    pub summary: AnalysisResultSummary,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResultExportRequest {
    pub format: String, // csv, json, xlsx
    pub job_ids: Option<Vec<Uuid>>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResultExportResponse {
    pub export_id: Uuid,
    pub format: String,
    pub status: String, // generating, ready, failed
    pub download_url: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

// Analysis CRUD Handlers

#[handler]
pub async fn list_analysis(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let project_id = req.param::<String>("project_id")
        .ok_or_else(|| AppError::BadRequest("project_id is required".to_string()))?;
    
    let active_only = req.query::<bool>("active_only").unwrap_or(true);
    let limit = req.query::<i64>("limit").unwrap_or(50).min(100);

    let query = if active_only {
        format!(
            "SELECT id, title, script_content, metadata, project_id, is_active, version, created_at, updated_at FROM analyses WHERE project_id = '{}' AND is_active = true ORDER BY updated_at DESC LIMIT {}",
            project_id, limit
        )
    } else {
        format!(
            "SELECT id, title, script_content, metadata, project_id, is_active, version, created_at, updated_at FROM analyses WHERE project_id = '{}' ORDER BY updated_at DESC LIMIT {}",
            project_id, limit
        )
    };

    let rows = sqlx::query(&query)
        .fetch_all(&state.db_pool)
        .await
        .map_err(AppError::SqlxError)?;

    let response: Vec<AnalysisResponse> = rows.into_iter().map(|row| {
        let metadata_val: Option<Value> = row.try_get("metadata").ok();
        let metadata_map: HashMap<String, Value> = metadata_val.as_ref()
            .and_then(|m| m.as_object())
            .map(|obj| {
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            })
            .unwrap_or_default();

        let description = metadata_map.get("description")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());

        let tags = metadata_map.get("tags")
            .and_then(|t| t.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        let created_at: Option<time::OffsetDateTime> = row.try_get("created_at").ok();
        let updated_at: Option<time::OffsetDateTime> = row.try_get("updated_at").ok();

        AnalysisResponse {
            id: row.get("id"),
            title: row.get("title"),
            script_content: row.get("script_content"),
            description,
            tags,
            metadata: metadata_map,
            project_id: row.get("project_id"),
            is_active: row.try_get("is_active").unwrap_or(Some(true)).unwrap_or(true),
            version: row.try_get("version").unwrap_or(Some(1)).unwrap_or(1),
            created_at: created_at.map(convert_time_to_chrono).unwrap_or_else(chrono::Utc::now),
            updated_at: updated_at.map(convert_time_to_chrono).unwrap_or_else(chrono::Utc::now),
        }
    }).collect();

    res.render(Json(response));
    Ok(())
}

#[handler]
pub async fn get_analysis(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let analysis_id = req.param::<Uuid>("analysis_id")
        .ok_or_else(|| AppError::BadRequest("analysis_id is required".to_string()))?;

    let analysis = sqlx::query!(
        r#"
        SELECT id, title, script_content, metadata, project_id, is_active, version, created_at, updated_at
        FROM analyses 
        WHERE id = $1
        "#,
        analysis_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    match analysis {
        Some(row) => {
            let metadata_map: HashMap<String, Value> = row.metadata.as_ref()
                .and_then(|m| m.as_object())
                .map(|obj| {
                    obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                })
                .unwrap_or_default();

            let description = metadata_map.get("description")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string());

            let tags = metadata_map.get("tags")
                .and_then(|t| t.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default();

            let response = AnalysisResponse {
                id: row.id,
                title: row.title,
                script_content: row.script_content,
                description,
                tags,
                metadata: metadata_map,
                project_id: row.project_id,
                is_active: row.is_active.unwrap_or(true),
                version: row.version.unwrap_or(1),
                created_at: row.created_at.map(convert_time_to_chrono).unwrap_or_else(chrono::Utc::now),
                updated_at: row.updated_at.map(convert_time_to_chrono).unwrap_or_else(chrono::Utc::now),
            };

            res.render(Json(response));
        }
        None => {
            return Err(AppError::NotFound(format!("Analysis {} not found", analysis_id)));
        }
    }

    Ok(())
}

#[handler]
pub async fn create_analysis(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let project_id = req.param::<String>("project_id")
        .ok_or_else(|| AppError::BadRequest("project_id is required".to_string()))?;
    
    let create_req: CreateAnalysisRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;

    // Build metadata
    let mut metadata = create_req.metadata.unwrap_or_default();
    if let Some(description) = &create_req.description {
        metadata.insert("description".to_string(), Value::String(description.clone()));
    }
    if let Some(tags) = &create_req.tags {
        metadata.insert("tags".to_string(), Value::Array(
            tags.iter().map(|t| Value::String(t.clone())).collect()
        ));
    }

    let analysis_id = Uuid::new_v4();
    let metadata_json = serde_json::to_value(&metadata)
        .map_err(|_| AppError::BadRequest("Invalid metadata".to_string()))?;

    sqlx::query!(
        r#"
        INSERT INTO analyses (id, title, script_content, metadata, project_id, is_active, version, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
        "#,
        analysis_id,
        create_req.title,
        create_req.script_content,
        metadata_json,
        project_id,
        true,
        1i32
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    // Return created analysis
    let response = AnalysisResponse {
        id: analysis_id,
        title: create_req.title,
        script_content: create_req.script_content,
        description: create_req.description,
        tags: create_req.tags.unwrap_or_default(),
        metadata,
        project_id,
        is_active: true,
        version: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    res.status_code(StatusCode::CREATED);
    res.render(Json(response));
    Ok(())
}

#[handler]
pub async fn update_analysis(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let analysis_id = req.param::<Uuid>("analysis_id")
        .ok_or_else(|| AppError::BadRequest("analysis_id is required".to_string()))?;
    
    let update_req: UpdateAnalysisRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;

    // Get current analysis
    let current = sqlx::query!(
        "SELECT metadata FROM analyses WHERE id = $1",
        analysis_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?
    .ok_or_else(|| AppError::NotFound(format!("Analysis {} not found", analysis_id)))?;

    // Update metadata
    let mut metadata: HashMap<String, Value> = current.metadata.as_ref()
        .and_then(|m| m.as_object())
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();

    if let Some(new_metadata) = update_req.metadata {
        metadata.extend(new_metadata);
    }
    if let Some(description) = &update_req.description {
        metadata.insert("description".to_string(), Value::String(description.clone()));
    }
    if let Some(tags) = &update_req.tags {
        metadata.insert("tags".to_string(), Value::Array(
            tags.iter().map(|t| Value::String(t.clone())).collect()
        ));
    }

    let metadata_json = serde_json::to_value(&metadata)
        .map_err(|_| AppError::BadRequest("Invalid metadata".to_string()))?;

    // Build update query dynamically
    let mut query = "UPDATE analyses SET updated_at = NOW()".to_string();
    let mut param_count = 1;

    if update_req.title.is_some() {
        param_count += 1;
        query.push_str(&format!(", title = ${}", param_count));
    }
    if update_req.script_content.is_some() {
        param_count += 1;
        query.push_str(&format!(", script_content = ${}", param_count));
    }
    if update_req.is_active.is_some() {
        param_count += 1;
        query.push_str(&format!(", is_active = ${}", param_count));
    }
    param_count += 1;
    query.push_str(&format!(", metadata = ${}", param_count));
    query.push_str(" WHERE id = $1");

    let mut db_query = sqlx::query(&query).bind(analysis_id);
    
    if let Some(title) = &update_req.title {
        db_query = db_query.bind(title);
    }
    if let Some(script_content) = &update_req.script_content {
        db_query = db_query.bind(script_content);
    }
    if let Some(is_active) = update_req.is_active {
        db_query = db_query.bind(is_active);
    }
    db_query = db_query.bind(&metadata_json);

    db_query.execute(&state.db_pool)
        .await
        .map_err(AppError::SqlxError)?;

    res.render(Json(serde_json::json!({"message": "Analysis updated successfully"})));
    Ok(())
}

#[handler]
pub async fn delete_analysis(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let analysis_id = req.param::<Uuid>("analysis_id")
        .ok_or_else(|| AppError::BadRequest("analysis_id is required".to_string()))?;

    let result = sqlx::query!(
        "UPDATE analyses SET is_active = false, updated_at = NOW() WHERE id = $1",
        analysis_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Analysis {} not found", analysis_id)));
    }

    res.render(Json(serde_json::json!({"message": "Analysis deleted successfully"})));
    Ok(())
}

// Analysis Execution Handlers

#[handler]
pub async fn execute_analysis(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let analysis_id = req.param::<Uuid>("analysis_id")
        .ok_or_else(|| AppError::BadRequest("analysis_id is required".to_string()))?;
    
    let execute_req: ExecuteAnalysisRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;

    // Check if analysis exists and is active
    let analysis = sqlx::query!(
        "SELECT id FROM analyses WHERE id = $1 AND is_active = true",
        analysis_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    if analysis.is_none() {
        return Err(AppError::NotFound(format!("Active analysis {} not found", analysis_id)));
    }

    // Submit job to analysis system
    let job_id = state.analysis_service
        .submit_analysis_job(
            analysis_id,
            serde_json::to_value(execute_req.parameters.unwrap_or_default())
                .unwrap_or(serde_json::json!({})),
            "api".to_string()
        )
        .await
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    res.status_code(StatusCode::ACCEPTED);
    res.render(Json(serde_json::json!({
        "job_id": job_id,
        "analysis_id": analysis_id,
        "status": "submitted",
        "message": "Analysis job submitted for execution"
    })));
    Ok(())
}

// Analysis Validation Handler

#[handler]
pub async fn validate_analysis(req: &mut Request, res: &mut Response) -> Result<(), AppError> {
    let validation_req: ValidationRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;

    // Basic JavaScript validation
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Check for export default
    if !validation_req.script_content.contains("export default") {
        errors.push("Script must contain 'export default' function".to_string());
    }

    // Check for common syntax issues
    if validation_req.script_content.contains("var ") {
        warnings.push("Consider using 'let' or 'const' instead of 'var'".to_string());
    }

    // More sophisticated validation could be added here

    let response = ValidationResponse {
        valid: errors.is_empty(),
        errors,
        warnings,
        metadata: None,
    };

    res.render(Json(response));
    Ok(())
}

// Job Management Handlers

#[handler]
pub async fn list_jobs(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    
    // Query parameters
    let analysis_id = req.query::<Uuid>("analysis_id");
    let status = req.query::<String>("status");
    let page = req.query::<i32>("page").unwrap_or(1).max(1);
    let per_page = req.query::<i32>("per_page").unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    // Build base query
    let mut where_conditions = Vec::new();
    let mut param_count = 0;

    if analysis_id.is_some() {
        param_count += 1;
        where_conditions.push(format!("analysis_id = ${}", param_count));
    }
    if status.is_some() {
        param_count += 1;
        where_conditions.push(format!("status = ${}", param_count));
    }

    let where_clause = if where_conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_conditions.join(" AND "))
    };

    // Get total count
    let count_query = format!(
        "SELECT COUNT(*) as count FROM analysis_jobs {}",
        where_clause
    );
    let mut count_db_query = sqlx::query(&count_query);
    if let Some(aid) = analysis_id {
        count_db_query = count_db_query.bind(aid);
    }
    if let Some(s) = &status {
        count_db_query = count_db_query.bind(s);
    }
    
    let total: i64 = count_db_query
        .fetch_one(&state.db_pool)
        .await
        .map_err(AppError::SqlxError)?
        .get("count");

    // Get jobs
    param_count += 1;
    let limit_param = param_count;
    param_count += 1;
    let offset_param = param_count;

    let jobs_query = format!(
        r#"
        SELECT id, analysis_id, status, parameters, result, error_message, 
               created_at, started_at, completed_at, execution_time_ms, triggered_by
        FROM analysis_jobs 
        {} 
        ORDER BY created_at DESC 
        LIMIT ${} OFFSET ${}
        "#,
        where_clause, limit_param, offset_param
    );

    let mut jobs_db_query = sqlx::query(&jobs_query);
    if let Some(aid) = analysis_id {
        jobs_db_query = jobs_db_query.bind(aid);
    }
    if let Some(s) = &status {
        jobs_db_query = jobs_db_query.bind(s);
    }
    jobs_db_query = jobs_db_query.bind(per_page as i64).bind(offset as i64);

    let jobs = jobs_db_query
        .fetch_all(&state.db_pool)
        .await
        .map_err(AppError::SqlxError)?;

    let job_responses: Vec<JobResponse> = jobs.into_iter().map(|row| {
        let parameters: HashMap<String, Value> = row.try_get::<Option<serde_json::Value>, _>("parameters")
            .unwrap_or_default()
            .and_then(|v| v.as_object().cloned())
            .map(|obj| obj.into_iter().collect())
            .unwrap_or_default();

        JobResponse {
            job_id: row.get("id"),
            analysis_id: row.get("analysis_id"),
            status: row.get("status"),
            parameters,
            result: row.try_get("result").ok().flatten(),
            error_message: row.try_get("error_message").ok().flatten(),
            created_at: convert_time_to_chrono(row.get("created_at")),
            started_at: row.try_get("started_at").ok().flatten().map(convert_time_to_chrono),
            completed_at: row.try_get("completed_at").ok().flatten().map(convert_time_to_chrono),
        }
    }).collect();

    let response = JobListResponse {
        jobs: job_responses,
        total,
        page,
        per_page,
    };

    res.render(Json(response));
    Ok(())
}

#[handler]
pub async fn get_job_status(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let job_id = req.param::<Uuid>("job_id")
        .ok_or_else(|| AppError::BadRequest("job_id is required".to_string()))?;

    let job = sqlx::query!(
        r#"
        SELECT id, analysis_id, status, created_at, started_at, completed_at, execution_time_ms
        FROM analysis_jobs 
        WHERE id = $1
        "#,
        job_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    match job {
        Some(row) => {
            let response = JobStatusResponse {
                job_id: row.id,
                analysis_id: row.analysis_id,
                status: row.status,
                progress: None, // Could be enhanced to track actual progress
                message: None,  // Could be enhanced with status messages
                created_at: row.created_at.map(convert_time_to_chrono).unwrap_or_else(Utc::now),
                started_at: row.started_at.map(convert_time_to_chrono),
                completed_at: row.completed_at.map(convert_time_to_chrono),
                execution_time_ms: row.execution_time_ms,
            };

            res.render(Json(response));
        }
        None => {
            return Err(AppError::NotFound(format!("Job {} not found", job_id)));
        }
    }

    Ok(())
}

#[handler]
pub async fn get_job_result(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let job_id = req.param::<Uuid>("job_id")
        .ok_or_else(|| AppError::BadRequest("job_id is required".to_string()))?;

    let job = sqlx::query!(
        r#"
        SELECT id, status, result, error_message
        FROM analysis_jobs 
        WHERE id = $1
        "#,
        job_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    match job {
        Some(row) => {
            match row.status.as_str() {
                "completed" => {
                    res.render(Json(serde_json::json!({
                        "job_id": job_id,
                        "status": "completed",
                        "result": row.result
                    })));
                }
                "failed" => {
                    res.render(Json(serde_json::json!({
                        "job_id": job_id,
                        "status": "failed",
                        "error": row.error_message
                    })));
                }
                "pending" | "running" => {
                    res.render(Json(serde_json::json!({
                        "job_id": job_id,
                        "status": row.status,
                        "message": "Job is still processing"
                    })));
                }
                _ => {
                    res.render(Json(serde_json::json!({
                        "job_id": job_id,
                        "status": row.status
                    })));
                }
            }
        }
        None => {
            return Err(AppError::NotFound(format!("Job {} not found", job_id)));
        }
    }

    Ok(())
}

#[handler]
pub async fn cancel_job(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let job_id = req.param::<Uuid>("job_id")
        .ok_or_else(|| AppError::BadRequest("job_id is required".to_string()))?;

    let cancel_req: CancelJobRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;

    // Check if job exists and can be cancelled
    let job = sqlx::query!(
        "SELECT status FROM analysis_jobs WHERE id = $1",
        job_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    match job {
        Some(row) => {
            match row.status.as_str() {
                "pending" | "running" => {
                    // Cancel the job
                    let error_message = cancel_req.reason
                        .unwrap_or_else(|| "Job cancelled by user".to_string());

                    sqlx::query!(
                        r#"
                        UPDATE analysis_jobs 
                        SET status = 'cancelled', 
                            error_message = $1, 
                            completed_at = NOW()
                        WHERE id = $2
                        "#,
                        error_message,
                        job_id
                    )
                    .execute(&state.db_pool)
                    .await
                    .map_err(AppError::SqlxError)?;

                    res.render(Json(serde_json::json!({
                        "job_id": job_id,
                        "status": "cancelled",
                        "message": "Job cancelled successfully"
                    })));
                }
                "completed" | "failed" | "cancelled" => {
                    return Err(AppError::BadRequest(format!("Job is already {}", row.status)));
                }
                _ => {
                    return Err(AppError::BadRequest(format!("Cannot cancel job with status: {}", row.status)));
                }
            }
        }
        None => {
            return Err(AppError::NotFound(format!("Job {} not found", job_id)));
        }
    }

    Ok(())
}

#[handler] 
pub async fn get_job_logs(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let job_id = req.param::<Uuid>("job_id")
        .ok_or_else(|| AppError::BadRequest("job_id is required".to_string()))?;

    let job = sqlx::query!(
        "SELECT logs FROM analysis_jobs WHERE id = $1",
        job_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    match job {
        Some(row) => {
            let logs = row.logs.unwrap_or_default();
            res.render(Json(serde_json::json!({
                "job_id": job_id,
                "logs": logs
            })));
        }
        None => {
            return Err(AppError::NotFound(format!("Job {} not found", job_id)));
        }
    }

    Ok(())
}

#[handler]
pub async fn retry_job(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let job_id = req.param::<Uuid>("job_id")
        .ok_or_else(|| AppError::BadRequest("job_id is required".to_string()))?;

    // Get failed job details
    let job = sqlx::query!(
        r#"
        SELECT analysis_id, parameters 
        FROM analysis_jobs 
        WHERE id = $1 AND status = 'failed'
        "#,
        job_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    match job {
        Some(row) => {
            // Create new job with same parameters
            let new_job_id = state.analysis_service
                .submit_analysis_job(
                    row.analysis_id,
                    row.parameters.unwrap_or_else(|| serde_json::json!({})),
                    "retry".to_string()
                )
                .await
                .map_err(|e| AppError::InternalServerError(e.to_string()))?;

            res.render(Json(serde_json::json!({
                "original_job_id": job_id,
                "new_job_id": new_job_id,
                "analysis_id": row.analysis_id,
                "status": "submitted",
                "message": "Job retry submitted successfully"
            })));
        }
        None => {
            return Err(AppError::NotFound(format!("Failed job {} not found", job_id)));
        }
    }

    Ok(())
}

// Analysis Scheduling Handlers

#[handler]
pub async fn create_schedule(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let analysis_id = req.param::<Uuid>("analysis_id")
        .ok_or_else(|| AppError::BadRequest("analysis_id is required".to_string()))?;
    
    let schedule_req: CreateScheduleRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;

    // Validate cron expression (basic validation)
    if !is_valid_cron(&schedule_req.cron_expression) {
        return Err(AppError::BadRequest("Invalid cron expression".to_string()));
    }

    // Check if analysis exists
    let analysis = sqlx::query!(
        "SELECT id FROM analyses WHERE id = $1",
        analysis_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    if analysis.is_none() {
        return Err(AppError::NotFound(format!("Analysis {} not found", analysis_id)));
    }

    let schedule_id = Uuid::new_v4();
    let timezone = schedule_req.timezone.unwrap_or_else(|| "UTC".to_string());
    let enabled = schedule_req.enabled.unwrap_or(true);

    // Calculate next run time (simplified - would use proper cron library in production)
    let next_run_at = calculate_next_run(&schedule_req.cron_expression, &timezone)
        .unwrap_or_else(|| Utc::now() + chrono::Duration::hours(1));

    sqlx::query!(
        r#"
        INSERT INTO analysis_schedules (id, analysis_id, cron_expression, timezone, enabled, created_at, updated_at, next_run_at)
        VALUES ($1, $2, $3, $4, $5, NOW(), NOW(), $6)
        "#,
        schedule_id,
        analysis_id,
        schedule_req.cron_expression,
        timezone,
        enabled,
        convert_chrono_to_time(next_run_at)
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    let response = ScheduleResponse {
        id: schedule_id,
        analysis_id,
        cron_expression: schedule_req.cron_expression,
        timezone,
        enabled,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_run_at: None,
        next_run_at: Some(next_run_at),
    };

    res.status_code(StatusCode::CREATED);
    res.render(Json(response));
    Ok(())
}

#[handler]
pub async fn list_schedules(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let analysis_id = req.param::<Uuid>("analysis_id")
        .ok_or_else(|| AppError::BadRequest("analysis_id is required".to_string()))?;

    let enabled_only = req.query::<bool>("enabled_only").unwrap_or(false);

    let schedules = sqlx::query!(
        r#"
        SELECT id, analysis_id, cron_expression, timezone, enabled, created_at, updated_at, last_run_at, next_run_at
        FROM analysis_schedules 
        WHERE analysis_id = $1 AND ($2 = false OR enabled = true)
        ORDER BY created_at DESC
        "#,
        analysis_id,
        enabled_only
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    let response: Vec<ScheduleResponse> = schedules.into_iter().map(|row| {
        ScheduleResponse {
            id: row.id,
            analysis_id: row.analysis_id,
            cron_expression: row.cron_expression,
            timezone: row.timezone.unwrap_or_else(|| "UTC".to_string()),
            enabled: row.enabled.unwrap_or(true),
            created_at: row.created_at.map(convert_time_to_chrono).unwrap_or_else(Utc::now),
            updated_at: row.updated_at.map(convert_time_to_chrono).unwrap_or_else(Utc::now),
            last_run_at: row.last_run_at.map(convert_time_to_chrono),
            next_run_at: row.next_run_at.map(convert_time_to_chrono),
        }
    }).collect();

    res.render(Json(response));
    Ok(())
}

#[handler]
pub async fn get_schedule(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let schedule_id = req.param::<Uuid>("schedule_id")
        .ok_or_else(|| AppError::BadRequest("schedule_id is required".to_string()))?;

    let schedule = sqlx::query!(
        r#"
        SELECT id, analysis_id, cron_expression, timezone, enabled, created_at, updated_at, last_run_at, next_run_at
        FROM analysis_schedules 
        WHERE id = $1
        "#,
        schedule_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    match schedule {
        Some(row) => {
            let response = ScheduleResponse {
                id: row.id,
                analysis_id: row.analysis_id,
                cron_expression: row.cron_expression,
                timezone: row.timezone.unwrap_or_else(|| "UTC".to_string()),
                enabled: row.enabled.unwrap_or(true),
                created_at: row.created_at.map(convert_time_to_chrono).unwrap_or_else(Utc::now),
                updated_at: row.updated_at.map(convert_time_to_chrono).unwrap_or_else(Utc::now),
                last_run_at: row.last_run_at.map(convert_time_to_chrono),
                next_run_at: row.next_run_at.map(convert_time_to_chrono),
            };

            res.render(Json(response));
        }
        None => {
            return Err(AppError::NotFound(format!("Schedule {} not found", schedule_id)));
        }
    }

    Ok(())
}

#[handler]
pub async fn update_schedule(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let schedule_id = req.param::<Uuid>("schedule_id")
        .ok_or_else(|| AppError::BadRequest("schedule_id is required".to_string()))?;
    
    let update_req: UpdateScheduleRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;

    // Validate cron expression if provided
    if let Some(ref cron) = update_req.cron_expression {
        if !is_valid_cron(cron) {
            return Err(AppError::BadRequest("Invalid cron expression".to_string()));
        }
    }

    // Get current schedule
    let current = sqlx::query!(
        "SELECT cron_expression, timezone FROM analysis_schedules WHERE id = $1",
        schedule_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?
    .ok_or_else(|| AppError::NotFound(format!("Schedule {} not found", schedule_id)))?;

    // Calculate new next_run_at if cron or timezone changed
    let cron_expr = update_req.cron_expression.as_ref().unwrap_or(&current.cron_expression);
    let default_timezone = current.timezone.unwrap_or_else(|| "UTC".to_string());
    let timezone = update_req.timezone.as_ref().unwrap_or(&default_timezone);

    let next_run_at = if update_req.cron_expression.is_some() || update_req.timezone.is_some() {
        Some(calculate_next_run(cron_expr, timezone)
            .unwrap_or_else(|| Utc::now() + chrono::Duration::hours(1)))
    } else {
        None
    };

    // Build update query dynamically
    let mut query = "UPDATE analysis_schedules SET updated_at = NOW()".to_string();
    let mut param_count = 1;

    if update_req.cron_expression.is_some() {
        param_count += 1;
        query.push_str(&format!(", cron_expression = ${}", param_count));
    }
    if update_req.timezone.is_some() {
        param_count += 1;
        query.push_str(&format!(", timezone = ${}", param_count));
    }
    if update_req.enabled.is_some() {
        param_count += 1;
        query.push_str(&format!(", enabled = ${}", param_count));
    }
    if next_run_at.is_some() {
        param_count += 1;
        query.push_str(&format!(", next_run_at = ${}", param_count));
    }
    query.push_str(" WHERE id = $1");

    let mut db_query = sqlx::query(&query).bind(schedule_id);
    
    if let Some(cron) = &update_req.cron_expression {
        db_query = db_query.bind(cron);
    }
    if let Some(tz) = &update_req.timezone {
        db_query = db_query.bind(tz);
    }
    if let Some(enabled) = update_req.enabled {
        db_query = db_query.bind(enabled);
    }
    if let Some(next_run) = next_run_at {
        db_query = db_query.bind(next_run);
    }

    db_query.execute(&state.db_pool)
        .await
        .map_err(AppError::SqlxError)?;

    res.render(Json(serde_json::json!({"message": "Schedule updated successfully"})));
    Ok(())
}

#[handler]
pub async fn delete_schedule(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let schedule_id = req.param::<Uuid>("schedule_id")
        .ok_or_else(|| AppError::BadRequest("schedule_id is required".to_string()))?;

    let result = sqlx::query!(
        "DELETE FROM analysis_schedules WHERE id = $1",
        schedule_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Schedule {} not found", schedule_id)));
    }

    res.render(Json(serde_json::json!({"message": "Schedule deleted successfully"})));
    Ok(())
}

#[handler]
pub async fn trigger_scheduled_analysis(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let schedule_id = req.param::<Uuid>("schedule_id")
        .ok_or_else(|| AppError::BadRequest("schedule_id is required".to_string()))?;

    // Get schedule details
    let schedule = sqlx::query!(
        "SELECT analysis_id, enabled FROM analysis_schedules WHERE id = $1",
        schedule_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    match schedule {
        Some(row) => {
            if !row.enabled.unwrap_or(false) {
                return Err(AppError::BadRequest("Schedule is disabled".to_string()));
            }

            // Trigger the analysis
            let job_id = state.analysis_service
                .submit_analysis_job(
                    row.analysis_id,
                    serde_json::json!({}), // No parameters for scheduled runs
                    "schedule".to_string()
                )
                .await
                .map_err(|e| AppError::InternalServerError(e.to_string()))?;

            // Update last_run_at
            sqlx::query!(
                "UPDATE analysis_schedules SET last_run_at = NOW() WHERE id = $1",
                schedule_id
            )
            .execute(&state.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

            res.render(Json(serde_json::json!({
                "job_id": job_id,
                "analysis_id": row.analysis_id,
                "schedule_id": schedule_id,
                "status": "triggered",
                "message": "Scheduled analysis triggered successfully"
            })));
        }
        None => {
            return Err(AppError::NotFound(format!("Schedule {} not found", schedule_id)));
        }
    }

    Ok(())
}

// Helper functions for scheduling

fn is_valid_cron(cron: &str) -> bool {
    // Basic cron validation - in production use proper cron library
    let parts: Vec<&str> = cron.split_whitespace().collect();
    parts.len() == 5 || parts.len() == 6 // Standard cron (5) or with seconds (6)
}

fn calculate_next_run(_cron: &str, _timezone: &str) -> Option<DateTime<Utc>> {
    // Simplified next run calculation - in production use proper cron library
    // For now, just return 1 hour from now
    Some(Utc::now() + chrono::Duration::hours(1))
}

// Analysis Results Handlers

#[handler]
pub async fn get_analysis_results(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let analysis_id = req.param::<Uuid>("analysis_id")
        .ok_or_else(|| AppError::BadRequest("analysis_id is required".to_string()))?;

    let page = req.query::<i32>("page").unwrap_or(1).max(1);
    let per_page = req.query::<i32>("per_page").unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let status_filter = req.query::<String>("status");
    let date_from = req.query::<DateTime<Utc>>("date_from");
    let date_to = req.query::<DateTime<Utc>>("date_to");

    // Build where conditions
    let mut where_conditions = vec!["analysis_id = $1".to_string()];
    let mut param_count = 1;

    if status_filter.is_some() {
        param_count += 1;
        where_conditions.push(format!("status = ${}", param_count));
    }
    if date_from.is_some() {
        param_count += 1;
        where_conditions.push(format!("created_at >= ${}", param_count));
    }
    if date_to.is_some() {
        param_count += 1;
        where_conditions.push(format!("created_at <= ${}", param_count));
    }

    let where_clause = where_conditions.join(" AND ");

    // Get analysis title and summary stats
    let analysis_info = sqlx::query!(
        "SELECT title FROM analyses WHERE id = $1",
        analysis_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?
    .ok_or_else(|| AppError::NotFound(format!("Analysis {} not found", analysis_id)))?;

    // Get summary statistics
    let stats_query = format!(
        r#"
        SELECT 
            COUNT(*) as total_jobs,
            COUNT(CASE WHEN status = 'completed' THEN 1 END) as successful_jobs,
            COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed_jobs,
            AVG(CASE WHEN execution_time_ms IS NOT NULL THEN execution_time_ms::float END) as avg_execution_time,
            MAX(CASE WHEN status = 'completed' THEN completed_at END) as last_success,
            MAX(CASE WHEN status = 'failed' THEN completed_at END) as last_failure
        FROM analysis_jobs
        WHERE {}
        "#,
        where_clause
    );

    let mut stats_db_query = sqlx::query(&stats_query).bind(analysis_id);
    
    if let Some(s) = &status_filter {
        stats_db_query = stats_db_query.bind(s);
    }
    if let Some(from) = date_from {
        stats_db_query = stats_db_query.bind(from);
    }
    if let Some(to) = date_to {
        stats_db_query = stats_db_query.bind(to);
    }

    let stats = stats_db_query
        .fetch_one(&state.db_pool)
        .await
        .map_err(AppError::SqlxError)?;

    // Get paginated results
    param_count += 1;
    let limit_param = param_count;
    param_count += 1;
    let offset_param = param_count;

    let jobs_query = format!(
        r#"
        SELECT id, analysis_id, status, parameters, result, error_message,
               created_at, started_at, completed_at, execution_time_ms, triggered_by
        FROM analysis_jobs
        WHERE {}
        ORDER BY created_at DESC
        LIMIT ${} OFFSET ${}
        "#,
        where_clause, limit_param, offset_param
    );

    let mut jobs_db_query = sqlx::query(&jobs_query).bind(analysis_id);
    
    if let Some(s) = &status_filter {
        jobs_db_query = jobs_db_query.bind(s);
    }
    if let Some(from) = date_from {
        jobs_db_query = jobs_db_query.bind(from);
    }
    if let Some(to) = date_to {
        jobs_db_query = jobs_db_query.bind(to);
    }
    jobs_db_query = jobs_db_query.bind(per_page as i64).bind(offset as i64);

    let jobs = jobs_db_query
        .fetch_all(&state.db_pool)
        .await
        .map_err(AppError::SqlxError)?;

    let job_responses: Vec<JobResponse> = jobs.into_iter().map(|row| {
        let parameters: HashMap<String, Value> = row.try_get::<Option<serde_json::Value>, _>("parameters")
            .unwrap_or_default()
            .and_then(|v| v.as_object().cloned())
            .map(|obj| obj.into_iter().collect())
            .unwrap_or_default();

        JobResponse {
            job_id: row.get("id"),
            analysis_id: row.get("analysis_id"),
            status: row.get("status"),
            parameters,
            result: row.try_get("result").ok().flatten(),
            error_message: row.try_get("error_message").ok().flatten(),
            created_at: convert_time_to_chrono(row.get("created_at")),
            started_at: row.try_get("started_at").ok().flatten().map(convert_time_to_chrono),
            completed_at: row.try_get("completed_at").ok().flatten().map(convert_time_to_chrono),
        }
    }).collect();

    let summary = AnalysisResultSummary {
        analysis_id,
        analysis_title: analysis_info.title,
        total_jobs: stats.get("total_jobs"),
        successful_jobs: stats.get("successful_jobs"),
        failed_jobs: stats.get("failed_jobs"),
        average_execution_time_ms: stats.try_get("avg_execution_time").ok().flatten(),
        last_success: stats.try_get("last_success").ok().flatten(),
        last_failure: stats.try_get("last_failure").ok().flatten(),
    };

    let response = AnalysisResultsList {
        results: job_responses,
        summary,
        total: stats.get("total_jobs"),
        page,
        per_page,
    };

    res.render(Json(response));
    Ok(())
}

#[handler]
pub async fn get_result_analytics(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let analysis_id = req.param::<Uuid>("analysis_id")
        .ok_or_else(|| AppError::BadRequest("analysis_id is required".to_string()))?;

    let days = req.query::<i32>("days").unwrap_or(30).clamp(1, 365);

    // Get analytics data
    let analytics = sqlx::query!(
        r#"
        SELECT 
            DATE(created_at) as date,
            COUNT(*) as total_runs,
            COUNT(CASE WHEN status = 'completed' THEN 1 END) as successful_runs,
            COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed_runs,
            AVG(CASE WHEN execution_time_ms IS NOT NULL THEN execution_time_ms::float END) as avg_execution_time
        FROM analysis_jobs
        WHERE analysis_id = $1 AND created_at >= NOW() - INTERVAL '1 day' * $2
        GROUP BY DATE(created_at)
        ORDER BY date DESC
        "#,
        analysis_id,
        days as f64
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    // Get performance trends
    let performance = sqlx::query!(
        r#"
        SELECT 
            AVG(CASE WHEN execution_time_ms IS NOT NULL THEN execution_time_ms::float END) as avg_time,
            MIN(CASE WHEN execution_time_ms IS NOT NULL THEN execution_time_ms END) as min_time,
            MAX(CASE WHEN execution_time_ms IS NOT NULL THEN execution_time_ms END) as max_time,
            COUNT(*) as total_completed
        FROM analysis_jobs
        WHERE analysis_id = $1 AND status = 'completed' AND created_at >= NOW() - INTERVAL '1 day' * $2
        "#,
        analysis_id,
        days as f64
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    // Get error patterns
    let errors = sqlx::query!(
        r#"
        SELECT 
            error_message,
            COUNT(*) as occurrence_count
        FROM analysis_jobs
        WHERE analysis_id = $1 AND status = 'failed' AND created_at >= NOW() - INTERVAL '1 day' * $2
        GROUP BY error_message
        ORDER BY occurrence_count DESC
        LIMIT 10
        "#,
        analysis_id,
        days as f64
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    let response = serde_json::json!({
        "analysis_id": analysis_id,
        "period_days": days,
        "daily_stats": analytics.into_iter().map(|row| {
            serde_json::json!({
                "date": row.date,
                "total_runs": row.total_runs,
                "successful_runs": row.successful_runs,
                "failed_runs": row.failed_runs,
                "success_rate": if row.total_runs.unwrap_or(0) > 0 { 
                    (row.successful_runs.unwrap_or(0) as f64 / row.total_runs.unwrap_or(1) as f64) * 100.0 
                } else { 0.0 },
                "avg_execution_time_ms": row.avg_execution_time
            })
        }).collect::<Vec<_>>(),
        "performance": {
            "avg_execution_time_ms": performance.avg_time,
            "min_execution_time_ms": performance.min_time,
            "max_execution_time_ms": performance.max_time,
            "total_completed": performance.total_completed
        },
        "top_errors": errors.into_iter().map(|row| {
            serde_json::json!({
                "error_message": row.error_message,
                "occurrence_count": row.occurrence_count
            })
        }).collect::<Vec<_>>()
    });

    res.render(Json(response));
    Ok(())
}

#[handler]
pub async fn export_results(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let _state = get_app_state(depot)?;
    let _analysis_id = req.param::<Uuid>("analysis_id")
        .ok_or_else(|| AppError::BadRequest("analysis_id is required".to_string()))?;

    let export_req: ResultExportRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;

    // Validate format
    if !matches!(export_req.format.as_str(), "csv" | "json" | "xlsx") {
        return Err(AppError::BadRequest("Invalid export format. Supported: csv, json, xlsx".to_string()));
    }

    // For now, return a simple export response (would be enhanced with actual export generation)
    let export_id = Uuid::new_v4();
    let expires_at = Utc::now() + chrono::Duration::hours(24);

    // In a real implementation, this would:
    // 1. Queue an export job
    // 2. Generate the file asynchronously
    // 3. Store it temporarily
    // 4. Return a download URL

    let response = ResultExportResponse {
        export_id,
        format: export_req.format,
        status: "generating".to_string(),
        download_url: None,
        expires_at: Some(expires_at),
    };

    res.status_code(StatusCode::ACCEPTED);
    res.render(Json(response));
    Ok(())
}

#[handler]
pub async fn get_export_status(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let _state = get_app_state(depot)?;
    let export_id = req.param::<Uuid>("export_id")
        .ok_or_else(|| AppError::BadRequest("export_id is required".to_string()))?;

    // For now, return a mock status (would query actual export status in real implementation)
    let response = ResultExportResponse {
        export_id,
        format: "csv".to_string(),
        status: "ready".to_string(),
        download_url: Some(format!("/api/exports/{}/download", export_id)),
        expires_at: Some(Utc::now() + chrono::Duration::hours(23)),
    };

    res.render(Json(response));
    Ok(())
}

#[handler]
pub async fn compare_results(req: &mut Request, depot: &mut Depot, res: &mut Response) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    
    let job_id_1 = req.query::<Uuid>("job1")
        .ok_or_else(|| AppError::BadRequest("job1 parameter is required".to_string()))?;
    let job_id_2 = req.query::<Uuid>("job2") 
        .ok_or_else(|| AppError::BadRequest("job2 parameter is required".to_string()))?;

    // Get both job results
    let jobs = sqlx::query!(
        r#"
        SELECT id, analysis_id, status, result, parameters, execution_time_ms, created_at
        FROM analysis_jobs
        WHERE id = ANY($1) AND status = 'completed'
        "#,
        &[job_id_1, job_id_2]
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::SqlxError)?;

    if jobs.len() != 2 {
        return Err(AppError::BadRequest("Both jobs must exist and be completed".to_string()));
    }

    let job1 = &jobs[0];
    let job2 = &jobs[1];

    // Basic comparison
    let response = serde_json::json!({
        "comparison": {
            "job1": {
                "id": job1.id,
                "analysis_id": job1.analysis_id,
                "result": job1.result,
                "parameters": job1.parameters,
                "execution_time_ms": job1.execution_time_ms,
                "created_at": job1.created_at
            },
            "job2": {
                "id": job2.id,
                "analysis_id": job2.analysis_id,
                "result": job2.result,
                "parameters": job2.parameters,
                "execution_time_ms": job2.execution_time_ms,
                "created_at": job2.created_at
            },
            "differences": {
                "same_analysis": job1.analysis_id == job2.analysis_id,
                "execution_time_diff_ms": job2.execution_time_ms.unwrap_or(0) - job1.execution_time_ms.unwrap_or(0),
                "results_identical": job1.result == job2.result,
                "parameters_identical": job1.parameters == job2.parameters
            }
        }
    });

    res.render(Json(response));
    Ok(())
}

// Configure API routes
pub fn configure_analysis_routes() -> Router {
    Router::new()
        // Analysis CRUD
        .push(
            Router::with_path("/projects/{project_id}/analysis")
                .get(list_analysis)
                .post(create_analysis)
        )
        .push(
            Router::with_path("/analysis/{analysis_id}")
                .get(get_analysis)
                .put(update_analysis)
                .delete(delete_analysis)
        )
        .push(
            Router::with_path("/analysis/{analysis_id}/execute")
                .post(execute_analysis)
        )
        // Validation
        .push(
            Router::with_path("/analysis/validate")
                .post(validate_analysis)
        )
        // Job Management
        .push(
            Router::with_path("/jobs")
                .get(list_jobs)
        )
        .push(
            Router::with_path("/jobs/{job_id}")
                .get(get_job_status)
        )
        .push(
            Router::with_path("/jobs/{job_id}/result")
                .get(get_job_result)
        )
        .push(
            Router::with_path("/jobs/{job_id}/cancel")
                .post(cancel_job)
        )
        .push(
            Router::with_path("/jobs/{job_id}/logs")
                .get(get_job_logs)
        )
        .push(
            Router::with_path("/jobs/{job_id}/retry")
                .post(retry_job)
        )
        // Analysis Scheduling
        .push(
            Router::with_path("/analysis/{analysis_id}/schedules")
                .get(list_schedules)
                .post(create_schedule)
        )
        .push(
            Router::with_path("/schedules/{schedule_id}")
                .get(get_schedule)
                .put(update_schedule)
                .delete(delete_schedule)
        )
        .push(
            Router::with_path("/schedules/{schedule_id}/trigger")
                .post(trigger_scheduled_analysis)
        )
        // Analysis Results
        .push(
            Router::with_path("/analysis/{analysis_id}/results")
                .get(get_analysis_results)
        )
        .push(
            Router::with_path("/analysis/{analysis_id}/analytics")
                .get(get_result_analytics)
        )
        .push(
            Router::with_path("/analysis/{analysis_id}/export")
                .post(export_results)
        )
        .push(
            Router::with_path("/exports/{export_id}/status")
                .get(get_export_status)
        )
        .push(
            Router::with_path("/results/compare")
                .get(compare_results)
        )
}