use serde_json::Value;
use std::error::Error;
use async_trait::async_trait;

#[async_trait]
pub trait DataSourceConnector: Send + Sync {
    // Core connection methods
    async fn test_connection(&self) -> Result<bool, Box<dyn Error>>;
    async fn execute_query(&self, query: &str, limit: i32) -> Result<Value, Box<dyn Error>>;
    
    // Schema inspection methods
    async fn fetch_schema(&self) -> Result<Value, Box<dyn Error>>;
    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error>>;
    
    // Advanced inspection methods
    async fn analyze_database(&self) -> Result<Value, Box<dyn Error>>;
    async fn get_tables_schema(&self, tables: Vec<&str>) -> Result<Value, Box<dyn Error>>;
    async fn search_tables(&self, pattern: &str) -> Result<Value, Box<dyn Error>>;
    async fn get_related_tables(&self, table: &str) -> Result<Value, Box<dyn Error>>;
    async fn get_database_stats(&self) -> Result<Value, Box<dyn Error>>;
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