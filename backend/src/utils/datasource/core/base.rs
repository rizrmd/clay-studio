use async_trait::async_trait;
use serde_json::Value;
use std::error::Error;

#[async_trait]
#[allow(dead_code)]
pub trait DataSourceConnector: Send + Sync {
    // Core connection methods
    #[allow(dead_code)]
    async fn test_connection(&mut self) -> Result<bool, Box<dyn Error + Send + Sync>>;
    async fn execute_query(&self, query: &str, limit: i32) -> Result<Value, Box<dyn Error + Send + Sync>>;
    
    // Table data methods
    #[allow(dead_code)]
    async fn get_table_data_with_pagination(
        &self, 
        table_name: &str, 
        page: i32, 
        limit: i32, 
        sort_column: Option<&str>, 
        sort_direction: Option<&str>
    ) -> Result<Value, Box<dyn Error + Send + Sync>>;

    // Schema inspection methods
    #[allow(dead_code)]
    async fn fetch_schema(&self) -> Result<Value, Box<dyn Error + Send + Sync>>;
    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error + Send + Sync>>;

    // Advanced inspection methods
    #[allow(dead_code)]
    async fn analyze_database(&self) -> Result<Value, Box<dyn Error + Send + Sync>>;
    async fn get_tables_schema(&self, tables: Vec<&str>) -> Result<Value, Box<dyn Error + Send + Sync>>;
    #[allow(dead_code)]
    async fn search_tables(&self, pattern: &str) -> Result<Value, Box<dyn Error + Send + Sync>>;
    #[allow(dead_code)]
    async fn get_related_tables(&self, table: &str) -> Result<Value, Box<dyn Error + Send + Sync>>;
    #[allow(dead_code)]
    async fn get_database_stats(&self) -> Result<Value, Box<dyn Error + Send + Sync>>;
}

// Helper function to format bytes
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_idx])
}
