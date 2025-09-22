use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDatasourceRequest {
    pub name: String,
    pub source_type: String, // postgresql, mysql, clickhouse, sqlite, oracle, sqlserver
    pub config: Value, // Can be string (URL) or object (individual fields)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateDatasourceRequest {
    pub name: Option<String>,
    pub config: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasourceResponse {
    pub id: String,
    pub name: String,
    pub source_type: String,
    pub config: Value,
    pub created_at: String,
    pub updated_at: String,
    pub project_id: String,
    pub schema_info: Option<Value>,
    pub connection_status: Option<String>,
    pub connection_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryRequest {
    pub query: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableDataRequest {
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub sort_column: Option<String>,
    pub sort_direction: Option<String>, // "asc" or "desc"
    pub filters: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DistinctValuesRequest {
    pub column: String,
    pub limit: Option<i32>, // Limit number of distinct values returned
    pub search: Option<String>, // Optional search filter
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RowIdsRequest {
    pub id_column: Option<String>, // Primary key column name (defaults to 'id')
    pub limit: Option<i32>, // Limit number of row IDs returned for performance
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteRowsRequest {
    pub row_ids: Vec<String>, // IDs or conditions to identify rows to delete
    pub id_column: Option<String>, // Primary key column name (defaults to 'id')
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRowsRequest {
    pub updates: std::collections::HashMap<String, std::collections::HashMap<String, Value>>, // rowId -> columnKey -> newValue
    pub id_column: Option<String>, // Primary key column name (defaults to 'id')
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsertRowsRequest {
    pub rows: Vec<std::collections::HashMap<String, Value>>, // Array of row objects
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableColumn {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub column_default: Option<String>,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
    pub character_maximum_length: Option<i32>,
    pub numeric_precision: Option<i32>,
    pub numeric_scale: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ForeignKeyInfo {
    pub column_name: String,
    pub referenced_table: String,
    pub referenced_column: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableStructure {
    pub table_name: String,
    pub columns: Vec<TableColumn>,
    pub primary_keys: Vec<String>,
    pub foreign_keys: Vec<ForeignKeyInfo>,
    pub indexes: Vec<IndexInfo>,
}