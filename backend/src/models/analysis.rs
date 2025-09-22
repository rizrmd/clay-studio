use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analysis {
    pub id: Uuid,
    pub title: String,
    pub script_content: String,
    pub project_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub version: i32,
    pub is_active: bool,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisVersion {
    pub id: Uuid,
    pub analysis_id: Uuid,
    pub version_number: i32,
    pub script_content: String,
    pub change_description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSchedule {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisJob {
    pub id: Uuid,
    pub analysis_id: Uuid,
    pub status: JobStatus,
    pub parameters: Value,
    pub result: Option<Value>,
    pub error_message: Option<String>,
    pub logs: Vec<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub execution_time_ms: Option<i64>,
    pub triggered_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisDependency {
    pub id: Uuid,
    pub analysis_id: Uuid,
    pub dependency_type: DependencyType,
    pub dependency_name: String,
    pub dependency_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResultStorage {
    pub id: Uuid,
    pub job_id: Uuid,
    pub storage_path: String,
    pub size_bytes: i64,
    pub checksum: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "pending"),
            JobStatus::Running => write!(f, "running"),
            JobStatus::Completed => write!(f, "completed"),
            JobStatus::Failed => write!(f, "failed"),
            JobStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    Datasource,
    Analysis,
}

impl std::fmt::Display for DependencyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DependencyType::Datasource => write!(f, "datasource"),
            DependencyType::Analysis => write!(f, "analysis"),
        }
    }
}

// DTOs for API requests/responses
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAnalysisRequest {
    pub title: String,
    pub script_content: String,
    pub project_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAnalysisRequest {
    pub title: Option<String>,
    pub script_content: Option<String>,
    pub change_description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteAnalysisRequest {
    pub parameters: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteAnalysisResponse {
    pub job_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisParameter {
    pub name: String,
    pub param_type: ParameterType,
    pub required: bool,
    pub description: Option<String>,
    pub default_value: Option<Value>,
    pub options: Option<Vec<ParameterOption>>,
    pub has_dynamic_options: bool,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    Text,
    Number,
    Date,
    Select,
    Boolean,
    Object,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterOption {
    pub value: String,
    pub label: String,
    pub options: Option<Vec<ParameterOption>>, // For grouped options
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisMetadata {
    pub id: String,
    pub title: String,
    pub parameters: std::collections::HashMap<String, AnalysisParameter>,
    pub dependencies: AnalysisDependencies,
    pub schedule: Option<ScheduleConfig>,
    pub created_at: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisDependencies {
    pub datasources: Vec<String>,
    pub analyses: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub cron: String,
    pub timezone: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub metadata: Option<AnalysisMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParameterOptionsRequest {
    pub current_params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParameterOptionsResponse {
    pub options: Vec<ParameterOption>,
}