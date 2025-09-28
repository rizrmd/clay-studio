use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use crate::utils::{context_compiler::ContextCompiler, AppError, get_app_state};

#[derive(Debug, Deserialize)]
pub struct UpdateContextRequest {
    pub context: String,
}

#[derive(Debug, Serialize)]
pub struct ContextResponse {
    pub context: Option<String>,
    pub context_compiled: Option<String>,
    pub context_compiled_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[handler]
pub async fn get_project_context(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let project_id = req
        .param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;
    
    let result = sqlx::query!(
        r#"
        SELECT 
            context,
            context_compiled,
            context_compiled_at
        FROM projects 
        WHERE id = $1
        "#,
        project_id
    )
    .fetch_optional(&state.db_pool)
    .await?;

    match result {
        Some(row) => {
            res.render(Json(ContextResponse {
                context: row.context,
                context_compiled: row.context_compiled,
                context_compiled_at: row.context_compiled_at.map(|ts| {
                    chrono::DateTime::<chrono::Utc>::from_timestamp(
                        ts.unix_timestamp(),
                        ts.nanosecond()
                    ).unwrap_or_else(|| chrono::Utc::now())
                }),
            }));
            Ok(())
        }
        None => Err(AppError::NotFound("Project not found".to_string())),
    }
}

#[handler]
pub async fn update_project_context(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let project_id = req
        .param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;
    let payload = req.parse_json::<UpdateContextRequest>().await?;
    
    // Update the context and clear compiled cache
    sqlx::query!(
        r#"
        UPDATE projects 
        SET 
            context = $1,
            context_compiled = NULL,
            context_compiled_at = NULL,
            updated_at = NOW()
        WHERE id = $2
        "#,
        payload.context,
        project_id
    )
    .execute(&state.db_pool)
    .await?;

    res.status_code(StatusCode::OK);
    Ok(())
}

#[handler]
pub async fn compile_project_context(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let project_id = req
        .param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;
    
    let compiler = ContextCompiler::new(state.db_pool.clone());
    
    let compiled = compiler.compile_context(&project_id).await
        .map_err(|e| AppError::InternalServerError(format!("Failed to compile context: {}", e)))?;

    res.render(Json(serde_json::json!({
        "success": true,
        "compiled": compiled
    })));
    Ok(())
}

#[handler]
pub async fn preview_project_context(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let project_id = req
        .param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;
    
    let compiler = ContextCompiler::new(state.db_pool.clone());
    
    let compiled = compiler.get_compiled_context(&project_id).await
        .map_err(|e| AppError::InternalServerError(format!("Failed to get compiled context: {}", e)))?;

    res.render(Json(serde_json::json!({
        "success": true,
        "compiled": compiled,
        "is_empty": compiled.is_empty()
    })));
    Ok(())
}

#[handler]
pub async fn clear_context_cache(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let project_id = req
        .param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;
    
    sqlx::query!(
        r#"
        UPDATE projects 
        SET 
            context_compiled = NULL,
            context_compiled_at = NULL
        WHERE id = $1
        "#,
        project_id
    )
    .execute(&state.db_pool)
    .await?;

    res.status_code(StatusCode::OK);
    Ok(())
}