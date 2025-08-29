use salvo::prelude::*;
use crate::models::*;
use crate::utils::AppState;
use crate::utils::AppError;
use crate::utils::claude_md_template;
use crate::utils::middleware::{get_current_client_id, is_current_user_root};
use crate::core::projects::{ProjectManager, ProjectInfo, ProjectInfoWithStats};
use crate::core::tools::ToolApplicabilityChecker;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::Row;

#[handler]
pub async fn get_project_context(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let project_id = req.param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;

    // Fetch data sources from database
    let data_source_rows = sqlx::query(
        "SELECT id, name, source_type, connection_config, schema_info, preview_data, table_list, last_tested_at, is_active
         FROM data_sources
         WHERE project_id = $1 AND is_active = true"
    )
    .bind(&project_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let mut data_sources = Vec::new();
    for row in data_source_rows {
        let table_list_json: Option<serde_json::Value> = row.get("table_list");
        let table_list = table_list_json.and_then(|v| {
            if v.is_array() {
                v.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|item| item.as_str().map(String::from))
                        .collect()
                })
            } else {
                None
            }
        });
        
        data_sources.push(DataSourceContext {
            id: row.get("id"),
            name: row.get("name"),
            source_type: row.get("source_type"),
            connection_config: row.get("connection_config"),
            schema_info: row.get("schema_info"),
            preview_data: row.get("preview_data"),
            table_list,
            last_tested_at: row.get::<Option<chrono::DateTime<Utc>>, _>("last_tested_at")
                .map(|dt| dt.to_rfc3339()),
            is_active: row.get("is_active"),
        });
    }
    
    // If no data sources found, return empty list
    if data_sources.is_empty() {
        data_sources = vec![];
    }

    // Determine applicable tools based on data sources
    let available_tools = ToolApplicabilityChecker::determine_applicable_tools(&data_sources);
    
    // Fetch project details from database
    let project_row = sqlx::query(
        "SELECT name, settings, organization_settings, created_at, updated_at 
         FROM projects 
         WHERE id = $1"
    )
    .bind(&project_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let (project_name, project_settings_json, org_settings_json) = if let Some(row) = project_row {
        (
            row.get::<String, _>("name"),
            row.get::<Option<serde_json::Value>, _>("settings").unwrap_or_else(|| serde_json::json!({})),
            row.get::<Option<serde_json::Value>, _>("organization_settings").unwrap_or_else(|| serde_json::json!({}))
        )
    } else {
        // Default values if project not found
        (
            "Unknown Project".to_string(),
            serde_json::json!({}),
            serde_json::json!({})
        )
    };
    
    // Get total conversation count for this project
    let total_conversations = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM conversations WHERE project_id = $1"
    )
    .bind(&project_id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(0) as usize;
    
    // Fetch recent activity (recent messages and data source changes)
    let recent_messages = sqlx::query(
        "SELECT m.id, m.content, m.created_at, c.id as conversation_id, c.title 
         FROM messages m 
         JOIN conversations c ON m.conversation_id = c.id 
         WHERE c.project_id = $1 
         ORDER BY m.created_at DESC 
         LIMIT 5"
    )
    .bind(&project_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let mut recent_activity = Vec::new();
    for msg_row in recent_messages {
        let content: String = msg_row.get("content");
        let description = if content.len() > 50 {
            format!("{}...", &content[..50])
        } else {
            content
        };
        
        recent_activity.push(RecentActivity {
            activity_type: "message".to_string(),
            description,
            timestamp: msg_row.get::<chrono::DateTime<Utc>, _>("created_at").to_rfc3339(),
            conversation_id: Some(msg_row.get("conversation_id")),
        });
    }
    
    // Add recent data source updates
    let recent_datasource_updates = sqlx::query(
        "SELECT name, source_type, updated_at 
         FROM data_sources 
         WHERE project_id = $1 
         ORDER BY updated_at DESC 
         LIMIT 3"
    )
    .bind(&project_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    for ds_row in recent_datasource_updates {
        let name: String = ds_row.get("name");
        let source_type: String = ds_row.get("source_type");
        
        recent_activity.push(RecentActivity {
            activity_type: "data_source".to_string(),
            description: format!("Updated {} data source: {}", source_type, name),
            timestamp: ds_row.get::<chrono::DateTime<Utc>, _>("updated_at").to_rfc3339(),
            conversation_id: None,
        });
    }
    
    // Sort recent activity by timestamp
    recent_activity.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    recent_activity.truncate(10); // Keep only the 10 most recent activities

    // Update CLAUDE.md with current datasource information if client is available
    if let Ok(Some(client_row)) = sqlx::query(
        "SELECT id FROM clients WHERE status = 'active' LIMIT 1"
    )
    .fetch_optional(&state.db_pool)
    .await {
        let client_id: Uuid = client_row.get("id");
        
        // Get datasources for CLAUDE.md generation
        let datasource_values: Vec<serde_json::Value> = data_sources.iter().map(|ds| {
            serde_json::json!({
                "id": ds.id,
                "name": ds.name,
                "source_type": ds.source_type,
                "schema_info": ds.schema_info,
            })
        }).collect();
        
        // Generate enhanced CLAUDE.md with datasource information
        let claude_md_content = claude_md_template::generate_claude_md_with_datasources(
            &project_id,
            &project_name,
            datasource_values
        ).await;
        
        // Save the updated CLAUDE.md
        let project_manager = ProjectManager::new();
        let _ = project_manager.save_claude_md_content(client_id, &project_id, &claude_md_content);
    }

    let project_context = ProjectContextResponse {
        project_id: project_id.clone(),
        project_settings: ProjectSettings {
            project_id: project_id.clone(),
            name: project_name,
            settings: project_settings_json,
            organization_settings: org_settings_json,
            default_analysis_preferences: AnalysisPreferences::default(),
        },
        data_sources,
        available_tools,
        total_conversations: total_conversations as i32,
        recent_activity,
    };

    res.render(Json(project_context));
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
}

#[handler]
pub async fn create_project(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let create_req: CreateProjectRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;
    
    // Get current user's client_id
    let client_id = get_current_client_id(depot)?;
    
    // Insert project into database - PostgreSQL will generate the UUID as string
    let project_id = Uuid::new_v4().to_string();
    let project_row = sqlx::query(
        "INSERT INTO projects (id, name, client_id) VALUES ($1, $2, $3) RETURNING id, name, created_at, updated_at"
    )
    .bind(&project_id)
    .bind(&create_req.name)
    .bind(client_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to create project: {}", e)))?;
    
    let project_id: String = project_row.get("id");
    let project_name: String = project_row.get("name");
    let created_at: chrono::DateTime<chrono::Utc> = project_row.get("created_at");
    let updated_at: chrono::DateTime<chrono::Utc> = project_row.get("updated_at");
    
    // Create project directory using the UUID string
    let project_manager = ProjectManager::new();
    project_manager.ensure_project_directory(client_id, &project_id)?;
    
    // Generate and save initial CLAUDE.md
    let claude_md_content = claude_md_template::generate_claude_md(&project_id, &project_name);
    project_manager.save_claude_md_content(client_id, &project_id, &claude_md_content)?;
    
    // Create project info
    let project_info = ProjectInfo {
        id: project_id,
        name: project_name,
        created_at: created_at.to_rfc3339(),
        updated_at: updated_at.to_rfc3339(),
        client_id,
    };
    
    res.render(Json(project_info));
    Ok(())
}

#[handler]
pub async fn list_projects(
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    
    // Get current user's client_id for filtering
    let client_id = get_current_client_id(depot)?;
    
    // Get projects filtered by client_id (unless user is root), excluding soft-deleted projects
    let project_rows = if is_current_user_root(depot) {
        sqlx::query(
            "SELECT id, name, created_at, updated_at FROM projects WHERE deleted_at IS NULL ORDER BY created_at DESC"
        )
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to fetch projects: {}", e)))?
    } else {
        sqlx::query(
            "SELECT id, name, created_at, updated_at FROM projects WHERE client_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC"
        )
        .bind(client_id)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to fetch projects: {}", e)))?
    };
    
    // Get the first active client for directory info (if needed)
    let client_row = sqlx::query(
        "SELECT id FROM clients WHERE status = 'active' LIMIT 1"
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let client_id = if let Some(row) = client_row {
        let id: Uuid = row.get("id");
        id
    } else {
        // Return empty array if no client
        res.render(Json(Vec::<ProjectInfoWithStats>::new()));
        return Ok(());
    };
    
    let mut projects = Vec::new();
    for row in project_rows {
        let project_id: String = row.get("id");
        let project_name: String = row.get("name");
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");
        
        // Get conversation count for this project
        let conversation_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM conversations WHERE project_id = $1"
        )
        .bind(&project_id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(0);
        
        // Get datasource count for this project
        let datasource_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM data_sources WHERE project_id = $1 AND is_active = true"
        )
        .bind(&project_id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(0);
        
        projects.push(ProjectInfoWithStats {
            id: project_id,
            name: project_name,
            created_at: created_at.to_rfc3339(),
            updated_at: updated_at.to_rfc3339(),
            client_id,
            conversation_count: Some(conversation_count as i32),
            datasource_count: Some(datasource_count as i32),
        });
    }
    
    res.render(Json(projects));
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct SaveQueryRequest {
    pub query_name: String,
    pub content: String,
}

#[handler]
pub async fn save_query(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let project_id = req.param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;
    let save_req: SaveQueryRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;
    
    // Get the first active client
    let client_row = sqlx::query(
        "SELECT id FROM clients WHERE status = 'active' LIMIT 1"
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let client_id = if let Some(row) = client_row {
        let id: Uuid = row.get("id");
        id
    } else {
        return Err(AppError::ServiceUnavailable(
            "No active client available. Please set up a client first.".to_string()
        ));
    };
    
    let project_manager = ProjectManager::new();
    let query_path = project_manager.save_query(
        client_id,
        &project_id,
        &save_req.query_name,
        &save_req.content
    )?;
    
    #[derive(Serialize)]
    struct SaveQueryResponse {
        message: String,
        path: String,
    }
    
    res.render(Json(SaveQueryResponse {
        message: "Query saved successfully".to_string(),
        path: query_path.to_string_lossy().to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn list_queries(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let project_id = req.param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;
    
    // Get the first active client
    let client_row = sqlx::query(
        "SELECT id FROM clients WHERE status = 'active' LIMIT 1"
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let client_id = if let Some(row) = client_row {
        let id: Uuid = row.get("id");
        id
    } else {
        return Err(AppError::ServiceUnavailable(
            "No active client available. Please set up a client first.".to_string()
        ));
    };
    
    let project_manager = ProjectManager::new();
    let queries = project_manager.list_queries(client_id, &project_id)?;
    
    res.render(Json(queries));
    Ok(())
}

#[handler]
pub async fn get_claude_md(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let project_id = req.param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;
    
    // Get the first active client
    let client_row = sqlx::query(
        "SELECT id FROM clients WHERE status = 'active' LIMIT 1"
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let client_id = if let Some(row) = client_row {
        let id: Uuid = row.get("id");
        id
    } else {
        return Err(AppError::ServiceUnavailable(
            "No active client available. Please set up a client first.".to_string()
        ));
    };
    
    let project_manager = ProjectManager::new();
    let content = project_manager.get_claude_md_content(client_id, &project_id)?;
    
    #[derive(Serialize)]
    struct ClaudeMdResponse {
        content: String,
    }
    
    res.render(Json(ClaudeMdResponse { content }));
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct SaveClaudeMdRequest {
    pub content: String,
}

#[handler]
pub async fn save_claude_md(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let project_id = req.param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;
    let save_req: SaveClaudeMdRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;
    
    // Get the first active client
    let client_row = sqlx::query(
        "SELECT id FROM clients WHERE status = 'active' LIMIT 1"
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
    
    let client_id = if let Some(row) = client_row {
        let id: Uuid = row.get("id");
        id
    } else {
        return Err(AppError::ServiceUnavailable(
            "No active client available. Please set up a client first.".to_string()
        ));
    };
    
    let project_manager = ProjectManager::new();
    project_manager.save_claude_md_content(client_id, &project_id, &save_req.content)?;
    
    #[derive(Serialize)]
    struct SaveClaudeMdResponse {
        message: String,
    }
    
    res.render(Json(SaveClaudeMdResponse {
        message: "CLAUDE.md saved successfully".to_string(),
    }));
    Ok(())
}

#[handler]
pub async fn delete_project(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let project_id = req.param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;
    
    // Get current user's client_id
    let client_id = get_current_client_id(depot)?;
    
    // Verify the project belongs to the current user (unless they're root)
    let project_exists = if is_current_user_root(depot) {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = $1 AND deleted_at IS NULL)"
        )
        .bind(&project_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    } else {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = $1 AND client_id = $2 AND deleted_at IS NULL)"
        )
        .bind(&project_id)
        .bind(client_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?
    };
    
    if !project_exists {
        return Err(AppError::NotFound("Project not found or already deleted".to_string()));
    }
    
    // Soft delete the project by setting deleted_at timestamp
    sqlx::query(
        "UPDATE projects SET deleted_at = NOW() WHERE id = $1"
    )
    .bind(&project_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to delete project: {}", e)))?;
    
    #[derive(Serialize)]
    struct DeleteProjectResponse {
        message: String,
        project_id: String,
    }
    
    res.render(Json(DeleteProjectResponse {
        message: "Project deleted successfully".to_string(),
        project_id,
    }));
    Ok(())
}