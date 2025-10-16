use super::base::DataSourceConnector;
use super::super::connectors::clickhouse::ClickHouseConnector;
use super::super::connectors::csv::CsvConnector;
use super::super::connectors::excel::ExcelConnector;
use super::super::connectors::json::JsonConnector;
use super::super::connectors::mysql::MySQLConnector;
use super::super::connectors::oracle::OracleConnector;
use super::super::connectors::postgres::PostgreSQLConnector;
use super::super::connectors::sqlite::SQLiteConnector;
use super::super::connectors::sqlserver::SqlServerConnector;
use serde_json::Value;
use std::error::Error;

// Helper function to convert error types
fn convert_error<E: Into<Box<dyn Error + Send + Sync>>>(e: E) -> Box<dyn Error> {
    let boxed: Box<dyn Error + Send + Sync> = e.into();
    boxed
}
use super::super::pooling::get_pool_manager;

#[derive(Debug)]
pub enum DataSourceType {
    PostgreSQL,
    MySQL,
    SQLite,
    ClickHouse,
    SqlServer,
    Oracle,
    Csv,
    Excel,
    Json,
}

impl From<&str> for DataSourceType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "postgresql" | "postgres" => DataSourceType::PostgreSQL,
            "mysql" => DataSourceType::MySQL,
            "sqlite" => DataSourceType::SQLite,
            "clickhouse" | "ch" => DataSourceType::ClickHouse,
            "sqlserver" | "mssql" | "sql_server" => DataSourceType::SqlServer,
            "oracle" => DataSourceType::Oracle,
            "csv" | "tsv" => DataSourceType::Csv,
            "excel" | "xlsx" | "xls" | "xlsm" => DataSourceType::Excel,
            "json" | "jsonl" => DataSourceType::Json,
            _ => DataSourceType::PostgreSQL, // default
        }
    }
}

pub async fn create_connector(
    source_type: &str,
    config: &Value,
) -> Result<Box<dyn DataSourceConnector>, Box<dyn Error>> {
    match DataSourceType::from(source_type) {
        DataSourceType::PostgreSQL => {
            let connector = PostgreSQLConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        },
        DataSourceType::MySQL => {
            let connector = MySQLConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        },
        DataSourceType::SQLite => {
            let connector = SQLiteConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        },
        DataSourceType::ClickHouse => {
            let connector = ClickHouseConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        },
        DataSourceType::SqlServer => {
            let connector = SqlServerConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        },
        DataSourceType::Oracle => {
            let connector = OracleConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        },
        DataSourceType::Csv => {
            let connector = CsvConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        },
        DataSourceType::Excel => {
            let connector = ExcelConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        },
        DataSourceType::Json => {
            let connector = JsonConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        }
    }
}

/// Create a connector that uses the global connection pool manager
/// For SQLx databases (PostgreSQL, MySQL, SQLite), this will use the global pool
/// For other databases, it falls back to individual connection management
#[allow(dead_code)]
pub async fn create_connector_with_pooling(
    datasource_id: &str,
    source_type: &str,
    config: &Value,
) -> Result<Box<dyn DataSourceConnector>, Box<dyn Error>> {
    match DataSourceType::from(source_type) {
        DataSourceType::PostgreSQL => {
            // Use global pool for PostgreSQL
            let pool_manager = get_pool_manager().await;
            let _pool = pool_manager.get_pool(datasource_id, source_type, config).await
                .map_err(|e| format!("Failed to get global pool: {}", e))?;
            // For now, still create individual connector but it should use the pool
            // TODO: Create a new PooledPostgreSQLConnector that uses the global pool
            let connector = PostgreSQLConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        },
        DataSourceType::MySQL => {
            // Use global pool for MySQL
            let pool_manager = get_pool_manager().await;
            let _pool = pool_manager.get_pool(datasource_id, source_type, config).await
                .map_err(|e| format!("Failed to get global pool: {}", e))?;
            // TODO: Create a new PooledMySQLConnector
            let connector = MySQLConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        },
        DataSourceType::SQLite => {
            // Use global pool for SQLite
            let pool_manager = get_pool_manager().await;
            let _pool = pool_manager.get_pool(datasource_id, source_type, config).await
                .map_err(|e| format!("Failed to get global pool: {}", e))?;
            // TODO: Create a new PooledSQLiteConnector
            let connector = SQLiteConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        },
        // For non-SQLx databases, use individual connectors
        DataSourceType::ClickHouse => {
            let connector = ClickHouseConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        },
        DataSourceType::SqlServer => {
            let connector = SqlServerConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        },
        DataSourceType::Oracle => {
            let connector = OracleConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        },
        DataSourceType::Csv => {
            let connector = CsvConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        },
        DataSourceType::Excel => {
            let connector = ExcelConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        },
        DataSourceType::Json => {
            let connector = JsonConnector::new(config).map_err(convert_error)?;
            Ok(Box::new(connector))
        }
    }
}
