use super::base::DataSourceConnector;
use super::postgres::PostgreSQLConnector;
use super::mysql::MySQLConnector;
use super::sqlite::SQLiteConnector;
use serde_json::Value;
use std::error::Error;

#[derive(Debug)]
pub enum DataSourceType {
    PostgreSQL,
    MySQL,
    SQLite,
    Csv,
}

impl From<&str> for DataSourceType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "postgresql" | "postgres" => DataSourceType::PostgreSQL,
            "mysql" => DataSourceType::MySQL,
            "sqlite" => DataSourceType::SQLite,
            "csv" => DataSourceType::Csv,
            _ => DataSourceType::PostgreSQL, // default
        }
    }
}

pub async fn create_connector(source_type: &str, config: &Value) -> Result<Box<dyn DataSourceConnector>, Box<dyn Error>> {
    match DataSourceType::from(source_type) {
        DataSourceType::PostgreSQL => {
            Ok(Box::new(PostgreSQLConnector::new(config)?))
        },
        DataSourceType::MySQL => {
            Ok(Box::new(MySQLConnector::new(config)?))
        },
        DataSourceType::SQLite => {
            Ok(Box::new(SQLiteConnector::new(config)?))
        },
        DataSourceType::Csv => {
            // CSV connector can be added later if needed
            Err("CSV connector not yet implemented in new structure".into())
        }
    }
}