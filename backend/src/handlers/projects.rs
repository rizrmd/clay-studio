use salvo::prelude::*;
use crate::models::*;
use crate::state::AppState;
use crate::error::AppError;
use chrono::Utc;

#[handler]
pub async fn get_project_context(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let _state = depot.obtain::<AppState>().unwrap();
    let project_id = req.param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;

    // Mock data for now
    let project_context = ProjectContextResponse {
        project_id: project_id.clone(),
        project_settings: ProjectSettings {
            project_id: project_id.clone(),
            name: "My Analytics Project".to_string(),
            settings: serde_json::json!({
                "theme": "dark",
                "notifications": true
            }),
            organization_settings: serde_json::json!({
                "org_name": "Clay Studio",
                "tier": "premium"
            }),
            default_analysis_preferences: AnalysisPreferences::default(),
        },
        data_sources: vec![
            DataSourceContext {
                id: "ds-1".to_string(),
                name: "Main Database".to_string(),
                source_type: "postgresql".to_string(),
                connection_config: serde_json::json!({
                    "host": "localhost",
                    "port": 5432,
                    "database": "analytics"
                }),
                schema_info: Some(serde_json::json!({
                    "has_time_column": true,
                    "numerical_columns": ["revenue", "cost", "profit"],
                    "tables": ["users", "transactions", "products"]
                })),
                preview_data: None,
                table_list: Some(vec![
                    "users".to_string(),
                    "transactions".to_string(),
                    "products".to_string()
                ]),
                last_tested_at: Some(Utc::now().to_rfc3339()),
                is_active: true,
            },
            DataSourceContext {
                id: "ds-2".to_string(),
                name: "CSV Uploads".to_string(),
                source_type: "csv".to_string(),
                connection_config: serde_json::json!({
                    "storage_path": "/data/csv"
                }),
                schema_info: None,
                preview_data: None,
                table_list: None,
                last_tested_at: Some(Utc::now().to_rfc3339()),
                is_active: true,
            }
        ],
        available_tools: vec![
            ToolContext {
                name: "SQL Query".to_string(),
                category: "sql".to_string(),
                description: "Execute SQL queries on connected databases".to_string(),
                parameters: serde_json::json!({
                    "query": "string",
                    "database": "string"
                }),
                applicable: true,
                usage_examples: vec![
                    "SELECT * FROM users LIMIT 10".to_string(),
                    "SELECT COUNT(*) FROM transactions".to_string(),
                ],
            },
            ToolContext {
                name: "Time Series Analysis".to_string(),
                category: "time_series".to_string(),
                description: "Analyze time-based patterns and trends in data".to_string(),
                parameters: serde_json::json!({
                    "data_source": "string",
                    "time_column": "string",
                    "value_column": "string",
                    "aggregation": "string"
                }),
                applicable: true,
                usage_examples: vec![
                    "Analyze monthly revenue trends".to_string(),
                    "Forecast next quarter sales".to_string(),
                ],
            },
            ToolContext {
                name: "Statistical Analysis".to_string(),
                category: "statistics".to_string(),
                description: "Perform statistical calculations and analysis".to_string(),
                parameters: serde_json::json!({
                    "operation": "string",
                    "columns": "array"
                }),
                applicable: true,
                usage_examples: vec![
                    "Calculate correlation between price and sales".to_string(),
                    "Get distribution statistics for revenue".to_string(),
                ],
            },
            ToolContext {
                name: "Data Quality Check".to_string(),
                category: "data_quality".to_string(),
                description: "Check data quality and identify issues".to_string(),
                parameters: serde_json::json!({
                    "table": "string",
                    "checks": "array"
                }),
                applicable: true,
                usage_examples: vec![
                    "Check for null values in critical columns".to_string(),
                    "Identify duplicate records".to_string(),
                ],
            },
            ToolContext {
                name: "Data Explorer".to_string(),
                category: "data_exploration".to_string(),
                description: "Explore and visualize data interactively".to_string(),
                parameters: serde_json::json!({
                    "source": "string",
                    "limit": "number"
                }),
                applicable: true,
                usage_examples: vec![
                    "Show sample data from users table".to_string(),
                    "Preview uploaded CSV file".to_string(),
                ],
            }
        ],
        total_conversations: 15,
        recent_activity: vec![
            RecentActivity {
                activity_type: "message".to_string(),
                description: "Analyzed Q4 revenue trends".to_string(),
                timestamp: Utc::now().to_rfc3339(),
                conversation_id: Some("conv-1".to_string()),
            },
            RecentActivity {
                activity_type: "data_source".to_string(),
                description: "Connected new PostgreSQL database".to_string(),
                timestamp: Utc::now().to_rfc3339(),
                conversation_id: None,
            },
            RecentActivity {
                activity_type: "message".to_string(),
                description: "Generated customer segmentation report".to_string(),
                timestamp: Utc::now().to_rfc3339(),
                conversation_id: Some("conv-2".to_string()),
            },
        ],
    };

    res.render(Json(project_context));
    Ok(())
}