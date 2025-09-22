use crate::models::project_share::{
    CreateShareRequest, ProjectShare, ShareResponse, ShareSettings, ShareType, UpdateShareRequest,
};
use crate::utils::AppState;
use salvo::prelude::*;
use chrono::{DateTime, Utc, Datelike, Timelike};
use serde::Deserialize;
use uuid::Uuid;
use sqlx::types::time::{PrimitiveDateTime, Date, Time};

// Safe conversion from chrono to sqlx time types
// These conversions should never fail for valid chrono dates/times
fn chrono_to_primitive_datetime(dt: chrono::NaiveDateTime) -> PrimitiveDateTime {
    // These conversions are safe because:
    // 1. chrono::NaiveDateTime always has valid year/month/day/hour/minute/second
    // 2. The ranges are compatible between chrono and sqlx time types
    // We use unwrap_or with fallback to handle the extremely unlikely edge case
    let date = Date::from_ordinal_date(
        dt.year(), 
        dt.ordinal() as u16
    ).unwrap_or_else(|_| {
        // Fallback to Unix epoch if somehow the date is invalid
        tracing::error!("Invalid date conversion from chrono: {}", dt);
        Date::from_ordinal_date(1970, 1)
            .unwrap_or_else(|_| {
                tracing::error!("Failed to create fallback date");
                Date::from_ordinal_date(1970, 2).unwrap_or(Date::from_ordinal_date(1970, 3).unwrap_or(Date::from_ordinal_date(1970, 4).unwrap_or_else(|_| panic!("Failed to create any valid date"))))
            })
    });
    
    let time = Time::from_hms_nano(
        dt.hour() as u8,
        dt.minute() as u8, 
        dt.second() as u8,
        dt.nanosecond()
    ).unwrap_or_else(|_| {
        // Fallback to midnight if somehow the time is invalid
        tracing::error!("Invalid time conversion from chrono: {}", dt);
        Time::MIDNIGHT
    });
    
    PrimitiveDateTime::new(date, time)
}

pub fn share_routes() -> Router {
    Router::new()
        .push(
            Router::with_path("/projects/{project_id}/shares")
                .get(list_shares)
                .post(create_share)
        )
        .push(
            Router::with_path("/shares/{share_token}")
                .get(get_share)
                .put(update_share)
                .delete(delete_share)
        )
        .push(Router::with_path("/shares/{share_token}/data").get(get_shared_data))
        .push(Router::with_path("/shares/{share_token}/session").post(create_session))
}

#[derive(Deserialize)]
struct CreateShareQuery {
    #[allow(dead_code)]
    base_url: Option<String>,
}

// Create a new project share
#[handler]
async fn create_share(req: &mut Request, res: &mut Response) {
    let state = match req.extensions().get::<AppState>() {
        Some(state) => state.clone(),
        None => {
            tracing::error!("AppState not found in request extensions");
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            return;
        }
    };
    let project_id = req.param::<String>("project_id").unwrap_or_default();
    let project_uuid = match Uuid::parse_str(&project_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            res.status_code(StatusCode::BAD_REQUEST);
            return;
        }
    };
    
    // Parse request body
    let request: CreateShareRequest = match req.parse_json().await {
        Ok(req) => req,
        Err(_) => {
            res.status_code(StatusCode::BAD_REQUEST);
            return;
        }
    };

    // Generate unique share token
    let share_token = generate_share_token();
    let share_id = Uuid::new_v4();

    // Serialize settings to JSON
    let settings_json = match serde_json::to_value(&request.settings) {
        Ok(json) => json,
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    // Insert share record
    let _query_result = match sqlx::query(
        r#"
        INSERT INTO project_shares (
            id, project_id, share_token, share_type, settings, 
            is_read_only, max_messages_per_session, expires_at, created_at, updated_at
        ) 
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, created_at, updated_at
        "#
    )
    .bind(share_id)
    .bind(project_uuid)
    .bind(&share_token)
    .bind(share_type_to_string(&request.share_type))
    .bind(&settings_json)
    .bind(request.is_read_only.unwrap_or(false))
    .bind(request.max_messages_per_session)
    .bind(request.expires_at.map(|dt| chrono_to_primitive_datetime(dt.naive_utc())))
    .bind(chrono_to_primitive_datetime(Utc::now().naive_utc()))
    .bind(chrono_to_primitive_datetime(Utc::now().naive_utc()))
    .fetch_one(&state.db_pool)
    .await {
        Ok(result) => result,
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    // If specific conversations, insert those relationships
    if let ShareType::SpecificConversations = request.share_type {
        if let Some(conversation_ids) = request.conversation_ids {
            for conv_id in conversation_ids {
                if sqlx::query(
                    r#"
                    INSERT INTO project_share_conversations (project_share_id, conversation_id)
                    VALUES ($1, $2)
                    "#
                )
                .bind(share_id)
                .bind(&conv_id)
                .execute(&state.db_pool)
                .await.is_err() {
                    res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                    return;
                }
            }
        }
    }

    // Fetch the created share
    let share = match fetch_share_by_token(&state, &share_token).await {
        Ok(share) => share,
        Err(status) => {
            res.status_code(status);
            return;
        }
    };

    // Generate embed codes
    let embed_codes = share.generate_embed_codes("https://clay.studio");
    let embed_url = format!("https://clay.studio/embed/{}", share_token);

    let response = ShareResponse {
        share,
        conversations: None, // Will be populated by frontend if needed
        embed_url,
        embed_codes,
    };

    res.render(Json(response));
}

// List all shares for a project
#[handler]
async fn list_shares(req: &mut Request, res: &mut Response) {
    let state = match req.extensions().get::<AppState>() {
        Some(state) => state.clone(),
        None => {
            tracing::error!("AppState not found in request extensions");
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            return;
        }
    };
    let project_id = req.param::<String>("project_id").unwrap_or_default();
    let project_uuid = match Uuid::parse_str(&project_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            res.status_code(StatusCode::BAD_REQUEST);
            return;
        }
    };

    let shares = match sqlx::query_as!(
        ShareRow,
        r#"
        SELECT id, project_id, share_token, share_type, settings,
               is_public, is_read_only, max_messages_per_session, expires_at,
               created_by, created_at, updated_at, deleted_at, view_count, last_accessed_at
        FROM project_shares 
        WHERE project_id = $1 AND deleted_at IS NULL
        ORDER BY created_at DESC
        "#,
        project_uuid
    )
    .fetch_all(&state.db_pool)
    .await {
        Ok(shares) => shares,
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    let result: Vec<ProjectShare> = shares
        .into_iter()
        .filter_map(|row| share_row_to_model(row).ok())
        .collect();

    res.render(Json(result));
}

// Get share details by token
#[handler]
async fn get_share(req: &mut Request, res: &mut Response) {
    let state = match req.extensions().get::<AppState>() {
        Some(state) => state.clone(),
        None => {
            tracing::error!("AppState not found in request extensions");
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            return;
        }
    };
    let share_token = req.param::<String>("share_token").unwrap_or_default();

    let share = match fetch_share_by_token(&state, &share_token).await {
        Ok(share) => share,
        Err(status) => {
            res.status_code(status);
            return;
        }
    };

    let embed_codes = share.generate_embed_codes("https://clay.studio");
    let embed_url = format!("https://clay.studio/embed/{}", share_token);

    let response = ShareResponse {
        share,
        conversations: None,
        embed_url,
        embed_codes,
    };

    res.render(Json(response));
}

// Update share settings
#[handler]
async fn update_share(req: &mut Request, res: &mut Response) {
    let state = match req.extensions().get::<AppState>() {
        Some(state) => state.clone(),
        None => {
            tracing::error!("AppState not found in request extensions");
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            return;
        }
    };
    let share_token = req.param::<String>("share_token").unwrap_or_default();
    
    let request: UpdateShareRequest = match req.parse_json().await {
        Ok(req) => req,
        Err(_) => {
            res.status_code(StatusCode::BAD_REQUEST);
            return;
        }
    };

    // Fetch current share
    let current_share = match fetch_share_by_token(&state, &share_token).await {
        Ok(share) => share,
        Err(status) => {
            res.status_code(status);
            return;
        }
    };

    // Update settings if provided
    let settings_json = if let Some(settings) = request.settings {
        match serde_json::to_value(&settings) {
            Ok(json) => json,
            Err(_) => {
                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                return;
            }
        }
    } else {
        match serde_json::to_value(&current_share.settings) {
            Ok(json) => json,
            Err(_) => {
                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                return;
            }
        }
    };

    // Update share record
    if sqlx::query(
        r#"
        UPDATE project_shares 
        SET settings = $1, is_read_only = COALESCE($2, is_read_only),
            max_messages_per_session = COALESCE($3, max_messages_per_session),
            expires_at = COALESCE($4, expires_at),
            updated_at = $5
        WHERE share_token = $6 AND deleted_at IS NULL
        "#
    )
    .bind(&settings_json)
    .bind(request.is_read_only)
    .bind(request.max_messages_per_session)
    .bind(request.expires_at)
    .bind(Utc::now())
    .bind(&share_token)
    .execute(&state.db_pool)
    .await.is_err() {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        return;
    }

    // Return updated share
    let updated_share = match fetch_share_by_token(&state, &share_token).await {
        Ok(share) => share,
        Err(status) => {
            res.status_code(status);
            return;
        }
    };

    res.render(Json(updated_share));
}

// Delete (soft delete) a share
#[handler]
async fn delete_share(req: &mut Request, res: &mut Response) {
    let state = match req.extensions().get::<AppState>() {
        Some(state) => state.clone(),
        None => {
            tracing::error!("AppState not found in request extensions");
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            return;
        }
    };
    let share_token = req.param::<String>("share_token").unwrap_or_default();

    let result = match sqlx::query(
        "UPDATE project_shares SET deleted_at = $1 WHERE share_token = $2"
    )
    .bind(Utc::now())
    .bind(&share_token)
    .execute(&state.db_pool)
    .await {
        Ok(result) => result,
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            return;
        }
    };

    if result.rows_affected() == 0 {
        res.status_code(StatusCode::NOT_FOUND);
        return;
    }

    res.status_code(StatusCode::NO_CONTENT);
}

// Get shared project data for embedding (public endpoint)
#[handler]
async fn get_shared_data(req: &mut Request, res: &mut Response) {
    let state = match req.extensions().get::<AppState>() {
        Some(state) => state.clone(),
        None => {
            tracing::error!("AppState not found in request extensions");
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            return;
        }
    };
    let share_token = req.param::<String>("share_token").unwrap_or_default();

    let share = match fetch_share_by_token(&state, &share_token).await {
        Ok(share) => share,
        Err(status) => {
            res.status_code(status);
            return;
        }
    };

    // Check if share is expired
    if !share.is_active() {
        res.status_code(StatusCode::GONE);
        return;
    }

    // Update view count
    let _ = sqlx::query(
        "UPDATE project_shares SET view_count = view_count + 1, last_accessed_at = $1 WHERE id = $2"
    )
    .bind(Utc::now())
    .bind(&share.id)
    .execute(&state.db_pool)
    .await;

    // Return basic share data (detailed implementation would fetch project and conversations)
    res.render(Json(serde_json::json!({
        "share": share,
        "project_name": "Shared Project", // Would fetch actual project name
        "conversations": [] // Would fetch based on share_type
    })));
}

// Create a session for interacting with shared project
#[handler]
async fn create_session(req: &mut Request, res: &mut Response) {
    let state = match req.extensions().get::<AppState>() {
        Some(state) => state.clone(),
        None => {
            tracing::error!("AppState not found in request extensions");
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            return;
        }
    };
    let share_token = req.param::<String>("share_token").unwrap_or_default();

    let share = match fetch_share_by_token(&state, &share_token).await {
        Ok(share) => share,
        Err(status) => {
            res.status_code(status);
            return;
        }
    };

    if !share.is_active() {
        res.status_code(StatusCode::GONE);
        return;
    }

    let session_token = generate_session_token();
    let session_id = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + chrono::Duration::hours(24);

    if sqlx::query(
        r#"
        INSERT INTO project_share_sessions (
            id, project_share_id, session_token, created_at, 
            last_activity_at, expires_at, message_count, max_messages
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#
    )
    .bind(&session_id)
    .bind(&share.id)
    .bind(&session_token)
    .bind(Utc::now())
    .bind(Utc::now())
    .bind(expires_at)
    .bind(0)
    .bind(share.max_messages_per_session.unwrap_or(50))
    .execute(&state.db_pool)
    .await.is_err() {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        return;
    }

    res.render(Json(serde_json::json!({
        "session_token": session_token,
        "expires_at": expires_at
    })));
}

// Helper functions

async fn fetch_share_by_token(
    state: &AppState,
    share_token: &str,
) -> Result<ProjectShare, StatusCode> {
    let share_row = sqlx::query_as!(
        ShareRow,
        r#"
        SELECT id, project_id, share_token, share_type, settings,
               is_public, is_read_only, max_messages_per_session, expires_at,
               created_by, created_at, updated_at, deleted_at, view_count, last_accessed_at
        FROM project_shares 
        WHERE share_token = $1 AND deleted_at IS NULL
        "#,
        share_token
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    share_row_to_model(share_row).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn generate_share_token() -> String {
    format!("share_{}", Uuid::new_v4().simple().to_string()[..16].to_lowercase())
}

fn generate_session_token() -> String {
    format!("sess_{}", Uuid::new_v4().simple().to_string()[..20].to_lowercase())
}

fn share_type_to_string(share_type: &ShareType) -> String {
    match share_type {
        ShareType::NewChat => "new_chat".to_string(),
        ShareType::AllHistory => "all_history".to_string(),
        ShareType::SpecificConversations => "specific_conversations".to_string(),
    }
}

fn string_to_share_type(s: &str) -> Result<ShareType, String> {
    match s {
        "new_chat" => Ok(ShareType::NewChat),
        "all_history" => Ok(ShareType::AllHistory),
        "specific_conversations" => Ok(ShareType::SpecificConversations),
        _ => Err("Invalid share type".to_string()),
    }
}

fn primitive_datetime_to_chrono(dt: PrimitiveDateTime) -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
    let date = chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month() as u32, dt.day() as u32)
        .ok_or_else(|| format!("Invalid date: {}-{}-{}", dt.year(), dt.month(), dt.day()))?;
    let time = chrono::NaiveTime::from_hms_nano_opt(dt.hour() as u32, dt.minute() as u32, dt.second() as u32, dt.nanosecond())
        .ok_or_else(|| format!("Invalid time: {}:{}:{}.{}", dt.hour(), dt.minute(), dt.second(), dt.nanosecond()))?;
    Ok(DateTime::from_naive_utc_and_offset(chrono::NaiveDateTime::new(date, time), Utc))
}

fn share_row_to_model(row: ShareRow) -> Result<ProjectShare, Box<dyn std::error::Error>> {
    let settings: ShareSettings = serde_json::from_value(row.settings)?;
    let share_type = string_to_share_type(&row.share_type).map_err(|e| e.to_string())?;

    Ok(ProjectShare {
        id: row.id.to_string(),
        project_id: row.project_id.to_string(),
        share_token: row.share_token,
        share_type,
        settings,
        is_public: row.is_public,
        is_read_only: row.is_read_only,
        max_messages_per_session: row.max_messages_per_session,
        expires_at: row.expires_at.map(primitive_datetime_to_chrono).transpose()?,
        created_by: row.created_by.as_ref().map(ToString::to_string),
        created_at: primitive_datetime_to_chrono(row.created_at)?,
        updated_at: primitive_datetime_to_chrono(row.updated_at)?,
        deleted_at: row.deleted_at.map(primitive_datetime_to_chrono).transpose()?,
        view_count: row.view_count.unwrap_or(0),
        last_accessed_at: row.last_accessed_at.map(primitive_datetime_to_chrono).transpose()?,
    })
}

// Database row structs

#[derive(sqlx::FromRow)]
struct ShareRow {
    id: Uuid,
    project_id: Uuid,
    share_token: String,
    share_type: String,
    settings: serde_json::Value,
    is_public: bool,
    is_read_only: bool,
    max_messages_per_session: Option<i32>,
    expires_at: Option<PrimitiveDateTime>,
    created_by: Option<Uuid>,
    created_at: PrimitiveDateTime,
    updated_at: PrimitiveDateTime,
    deleted_at: Option<PrimitiveDateTime>,
    view_count: Option<i32>,
    last_accessed_at: Option<PrimitiveDateTime>,
}