use salvo::prelude::*;
use serde::Deserialize;
use crate::models::*;
use crate::state::AppState;
use crate::error::AppError;
use chrono::Utc;
use uuid::Uuid;

#[handler]
pub async fn get_conversation_context(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let _state = depot.obtain::<AppState>().unwrap();
    let conversation_id = req.param::<String>("conversation_id")
        .ok_or(AppError::BadRequest("Missing conversation_id".to_string()))?;

    // Mock data for now
    let context = ConversationContext {
        conversation_id: conversation_id.clone(),
        project_id: "project-1".to_string(),
        messages: vec![
            Message::new_user("Hello, can you help me analyze this data?".to_string()),
            Message::new_assistant("Of course! I'd be happy to help you analyze your data.".to_string()),
        ],
        summary: None,
        data_sources: vec![
            DataSourceContext {
                id: "ds-1".to_string(),
                name: "Main Database".to_string(),
                source_type: "postgresql".to_string(),
                connection_config: serde_json::json!({}),
                schema_info: Some(serde_json::json!({
                    "has_time_column": true,
                    "numerical_columns": ["revenue", "cost", "profit"]
                })),
                preview_data: None,
                table_list: Some(vec!["users".to_string(), "transactions".to_string()]),
                last_tested_at: Some(Utc::now().to_rfc3339()),
                is_active: true,
            }
        ],
        available_tools: vec![
            ToolContext {
                name: "SQL Query".to_string(),
                category: "sql".to_string(),
                description: "Execute SQL queries on connected databases".to_string(),
                parameters: serde_json::json!({}),
                applicable: true,
                usage_examples: vec!["SELECT * FROM users".to_string()],
            },
            ToolContext {
                name: "Time Series Analysis".to_string(),
                category: "time_series".to_string(),
                description: "Analyze time-based patterns in data".to_string(),
                parameters: serde_json::json!({}),
                applicable: true,
                usage_examples: vec!["Analyze monthly trends".to_string()],
            }
        ],
        project_settings: ProjectSettings {
            project_id: "project-1".to_string(),
            user_id: "user-1".to_string(),
            client_id: Some("client-1".to_string()),
            name: "My Analytics Project".to_string(),
            settings: serde_json::json!({}),
            organization_settings: serde_json::json!({}),
            default_analysis_preferences: AnalysisPreferences::default(),
        },
        total_messages: 2,
        context_strategy: ContextStrategy::FullHistory,
    };

    res.render(Json(context));
    Ok(())
}

#[handler]
pub async fn list_conversations(
    _req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let _state = depot.obtain::<AppState>().unwrap();
    
    // Mock data
    let conversations = vec![
        Conversation {
            id: "conv-1".to_string(),
            project_id: "project-1".to_string(),
            title: Some("Data Analysis Session".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            message_count: 5,
        },
        Conversation {
            id: "conv-2".to_string(),
            project_id: "project-1".to_string(),
            title: Some("Revenue Forecasting".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            message_count: 10,
        },
    ];

    res.render(Json(conversations));
    Ok(())
}

#[handler]
pub async fn get_conversation(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let conversation_id = req.param::<String>("conversation_id")
        .ok_or(AppError::BadRequest("Missing conversation_id".to_string()))?;
    
    let conversation = Conversation {
        id: conversation_id,
        project_id: "project-1".to_string(),
        title: Some("Data Analysis Session".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        message_count: 5,
    };

    res.render(Json(conversation));
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    pub project_id: String,
    pub title: Option<String>,
}

#[handler]
pub async fn create_conversation(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let create_req: CreateConversationRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;
    
    let conversation = Conversation {
        id: Uuid::new_v4().to_string(),
        project_id: create_req.project_id,
        title: create_req.title,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        message_count: 0,
    };

    res.render(Json(conversation));
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct UpdateConversationRequest {
    pub title: Option<String>,
}

#[handler]
pub async fn update_conversation(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let conversation_id = req.param::<String>("conversation_id")
        .ok_or(AppError::BadRequest("Missing conversation_id".to_string()))?;
    
    let update_req: UpdateConversationRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;
    
    let conversation = Conversation {
        id: conversation_id,
        project_id: "project-1".to_string(),
        title: update_req.title,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        message_count: 5,
    };

    res.render(Json(conversation));
    Ok(())
}

#[handler]
pub async fn delete_conversation(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let conversation_id = req.param::<String>("conversation_id")
        .ok_or(AppError::BadRequest("Missing conversation_id".to_string()))?;
    
    res.render(Json(serde_json::json!({
        "success": true,
        "deleted_id": conversation_id
    })));
    Ok(())
}