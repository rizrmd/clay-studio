use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::utils::middleware::{get_current_user_id, is_current_user_root};
use crate::utils::{get_app_state, AppError};

use super::crud::{get_cached_datasource, normalize_database_type};
use super::types::TestConnectionResponse;

/// Test connection with arbitrary config (for form validation)
#[handler]
pub async fn test_connection_with_config(
    req: &mut Request,
    res: &mut Response,
    _depot: &mut Depot,
) -> Result<(), AppError> {
    #[derive(Debug, Serialize, Deserialize)]
    struct TestConfigRequest {
        source_type: String,
        config: Value,
    }

    let test_data: TestConfigRequest = req.parse_json().await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    // Normalize source type
    let normalized_source_type = normalize_database_type(&test_data.source_type);
    
    // Test connection based on source type
    let test_result = match normalized_source_type.as_str() {
        "postgresql" => test_postgres_connection(&test_data.config).await,
        "mysql" => test_mysql_connection(&test_data.config).await,
        "sqlite" => test_sqlite_connection(&test_data.config).await,
        "clickhouse" => test_clickhouse_connection(&test_data.config).await,
        "oracle" => test_oracle_connection(&test_data.config).await,
        "sqlserver" => test_sqlserver_connection(&test_data.config).await,
        _ => TestConnectionResponse {
            success: false,
            message: format!("Connection testing not implemented for {}", normalized_source_type),
            error: Some("Not implemented".to_string()),
        }
    };

    res.render(Json(test_result));
    Ok(())
}

/// Test connection to a datasource
#[handler]
pub async fn test_connection(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let user_id = get_current_user_id(depot)?;
    let datasource_id = req.param::<String>("datasource_id")
        .ok_or_else(|| AppError::BadRequest("Missing datasource_id".to_string()))?;

    // Get datasource and verify ownership using cache
    let cached_datasource = get_cached_datasource(&datasource_id, &user_id, is_current_user_root(depot), &state.db_pool).await?;
    
    let source_type = cached_datasource.datasource_type.clone();
    let config = cached_datasource.connection_config.clone();

    // Test connection based on source type
    let test_result = match source_type.as_str() {
        "postgresql" => test_postgres_connection(&config).await,
        "mysql" => test_mysql_connection(&config).await,
        "sqlite" => test_sqlite_connection(&config).await,
        "clickhouse" => test_clickhouse_connection(&config).await,
        "oracle" => test_oracle_connection(&config).await,
        "sqlserver" => test_sqlserver_connection(&config).await,
        _ => TestConnectionResponse {
            success: false,
            message: format!("Connection testing not implemented for {}", source_type),
            error: Some("Not implemented".to_string()),
        }
    };

    res.render(Json(test_result));
    Ok(())
}

// Helper functions for connection testing
async fn test_postgres_connection(config: &Value) -> TestConnectionResponse {
    let connection_url = if let Some(url) = config.as_str() {
        url.to_string()
    } else if let Some(obj) = config.as_object() {
        // If object has a 'url' field, use that directly
        if let Some(url) = obj.get("url").and_then(|v| v.as_str()) {
            url.to_string()
        } else {
            // Build connection URL from individual fields
            let host = obj.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
            let port = obj.get("port").and_then(|v| v.as_u64()).unwrap_or(5432);
            let database = obj.get("database").and_then(|v| v.as_str()).unwrap_or("");
            let user = obj.get("user").and_then(|v| v.as_str())
                .or_else(|| obj.get("username").and_then(|v| v.as_str()))
                .unwrap_or("");
            let password = obj.get("password").and_then(|v| v.as_str()).unwrap_or("");
            
            format!("postgresql://{}:{}@{}:{}/{}", user, password, host, port, database)
        }
    } else {
        return TestConnectionResponse {
            success: false,
            message: "Invalid configuration format".to_string(),
            error: Some("Config must be a connection URL string or object with connection details".to_string()),
        };
    };

    match sqlx::postgres::PgPool::connect(&connection_url).await {
        Ok(pool) => {
            // Test with a simple query
            match sqlx::query("SELECT 1").fetch_one(&pool).await {
                Ok(_) => TestConnectionResponse {
                    success: true,
                    message: "Connection successful".to_string(),
                    error: None,
                },
                Err(e) => TestConnectionResponse {
                    success: false,
                    message: "Connection established but query failed".to_string(),
                    error: Some(e.to_string()),
                }
            }
        },
        Err(e) => TestConnectionResponse {
            success: false,
            message: "Failed to connect to PostgreSQL".to_string(),
            error: Some(e.to_string()),
        }
    }
}

async fn test_mysql_connection(config: &Value) -> TestConnectionResponse {
    let connection_url = if let Some(url) = config.as_str() {
        url.to_string()
    } else if let Some(obj) = config.as_object() {
        // If object has a 'url' field, use that directly
        if let Some(url) = obj.get("url").and_then(|v| v.as_str()) {
            url.to_string()
        } else {
            // Build connection URL from individual fields
            let host = obj.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
            let port = obj.get("port").and_then(|v| v.as_u64()).unwrap_or(3306);
            let database = obj.get("database").and_then(|v| v.as_str()).unwrap_or("");
            let user = obj.get("user").and_then(|v| v.as_str())
                .or_else(|| obj.get("username").and_then(|v| v.as_str()))
                .unwrap_or("");
            let password = obj.get("password").and_then(|v| v.as_str()).unwrap_or("");
            
            format!("mysql://{}:{}@{}:{}/{}", user, password, host, port, database)
        }
    } else {
        return TestConnectionResponse {
            success: false,
            message: "Invalid configuration format".to_string(),
            error: Some("Config must be a connection URL string or object with connection details".to_string()),
        };
    };

    match sqlx::mysql::MySqlPool::connect(&connection_url).await {
        Ok(pool) => {
            // Test with a simple query
            match sqlx::query("SELECT 1").fetch_one(&pool).await {
                Ok(_) => TestConnectionResponse {
                    success: true,
                    message: "Connection successful".to_string(),
                    error: None,
                },
                Err(e) => TestConnectionResponse {
                    success: false,
                    message: "Connection established but query failed".to_string(),
                    error: Some(e.to_string()),
                }
            }
        },
        Err(e) => TestConnectionResponse {
            success: false,
            message: "Failed to connect to MySQL".to_string(),
            error: Some(e.to_string()),
        }
    }
}

async fn test_sqlite_connection(config: &Value) -> TestConnectionResponse {
    let connection_url = if let Some(url) = config.as_str() {
        if url.starts_with("sqlite://") {
            url.to_string()
        } else {
            format!("sqlite://{}", url)
        }
    } else if let Some(obj) = config.as_object() {
        // If object has a 'url' field, use that directly
        if let Some(url) = obj.get("url").and_then(|v| v.as_str()) {
            if url.starts_with("sqlite://") {
                url.to_string()
            } else {
                format!("sqlite://{}", url)
            }
        } else {
            // Build connection URL from path field
            let path = obj.get("path").and_then(|v| v.as_str())
                .or_else(|| obj.get("file").and_then(|v| v.as_str()))
                .or_else(|| obj.get("database").and_then(|v| v.as_str()))
                .unwrap_or(":memory:");
            
            format!("sqlite://{}", path)
        }
    } else {
        return TestConnectionResponse {
            success: false,
            message: "Invalid configuration format".to_string(),
            error: Some("Config must be a connection URL string or object with path/file field".to_string()),
        };
    };

    match sqlx::sqlite::SqlitePool::connect(&connection_url).await {
        Ok(pool) => {
            // Test with a simple query
            match sqlx::query("SELECT 1").fetch_one(&pool).await {
                Ok(_) => TestConnectionResponse {
                    success: true,
                    message: "Connection successful".to_string(),
                    error: None,
                },
                Err(e) => TestConnectionResponse {
                    success: false,
                    message: "Connection established but query failed".to_string(),
                    error: Some(e.to_string()),
                }
            }
        },
        Err(e) => TestConnectionResponse {
            success: false,
            message: "Failed to connect to SQLite".to_string(),
            error: Some(e.to_string()),
        }
    }
}

async fn test_clickhouse_connection(config: &Value) -> TestConnectionResponse {
    use clickhouse::Client;
    
    let (host, port, database, user, password) = if let Some(url) = config.as_str() {
        // Parse ClickHouse URL - basic implementation
        if url.starts_with("clickhouse://") || url.starts_with("http://") {
            // For now, use default values for URL parsing
            ("localhost", 8123, "default", "default", "")
        } else {
            return TestConnectionResponse {
                success: false,
                message: "Invalid ClickHouse URL format".to_string(),
                error: Some("URL should start with clickhouse:// or http://".to_string()),
            };
        }
    } else if let Some(obj) = config.as_object() {
        let host = obj.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
        let port = obj.get("port").and_then(|v| v.as_u64()).unwrap_or(8123) as u16;
        let database = obj.get("database").and_then(|v| v.as_str()).unwrap_or("default");
        let user = obj.get("user").and_then(|v| v.as_str())
            .or_else(|| obj.get("username").and_then(|v| v.as_str()))
            .unwrap_or("default");
        let password = obj.get("password").and_then(|v| v.as_str()).unwrap_or("");
        
        (host, port, database, user, password)
    } else {
        return TestConnectionResponse {
            success: false,
            message: "Invalid configuration format".to_string(),
            error: Some("Config must be a connection URL string or object with connection details".to_string()),
        };
    };

    let client = Client::default()
        .with_url(format!("http://{}:{}", host, port))
        .with_user(user)
        .with_password(password)
        .with_database(database);

    match client.query("SELECT 1").fetch_one::<u8>().await {
        Ok(_) => TestConnectionResponse {
            success: true,
            message: "Connection successful".to_string(),
            error: None,
        },
        Err(e) => TestConnectionResponse {
            success: false,
            message: "Failed to connect to ClickHouse".to_string(),
            error: Some(e.to_string()),
        }
    }
}

async fn test_oracle_connection(config: &Value) -> TestConnectionResponse {
    use oracle::Connection;
    
    let (username, password, connect_string) = if let Some(url) = config.as_str() {
        // For URL format, try to parse it
        ("".to_string(), "".to_string(), url.to_string())
    } else if let Some(obj) = config.as_object() {
        if let Some(url) = obj.get("url").and_then(|v| v.as_str()) {
            ("".to_string(), "".to_string(), url.to_string())
        } else {
            let host = obj.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
            let port = obj.get("port").and_then(|v| v.as_u64()).unwrap_or(1521);
            let service_name = obj.get("service_name").and_then(|v| v.as_str())
                .or_else(|| obj.get("database").and_then(|v| v.as_str()))
                .unwrap_or("XE");
            let user = obj.get("user").and_then(|v| v.as_str())
                .or_else(|| obj.get("username").and_then(|v| v.as_str()))
                .unwrap_or("").to_string();
            let password = obj.get("password").and_then(|v| v.as_str()).unwrap_or("").to_string();
            
            let connect_string = format!("{}:{}/{}@{}:{}", user, password, service_name, host, port);
            (user, password, connect_string)
        }
    } else {
        return TestConnectionResponse {
            success: false,
            message: "Invalid configuration format".to_string(),
            error: Some("Config must be a connection URL string or object with connection details".to_string()),
        };
    };

    // Oracle connections are synchronous, so we need to run in blocking context  
    let result = tokio::task::spawn_blocking(move || {
        match Connection::connect(&username, &password, &connect_string) {
            Ok(conn) => {
                // Test with a simple query
                match conn.query("SELECT 1 FROM dual", &[]) {
                    Ok(_) => TestConnectionResponse {
                        success: true,
                        message: "Connection successful".to_string(),
                        error: None,
                    },
                    Err(e) => TestConnectionResponse {
                        success: false,
                        message: "Connection established but query failed".to_string(),
                        error: Some(e.to_string()),
                    }
                }
            },
            Err(e) => TestConnectionResponse {
                success: false,
                message: "Failed to connect to Oracle".to_string(),
                error: Some(e.to_string()),
            }
        }
    }).await;

    match result {
        Ok(response) => response,
        Err(e) => TestConnectionResponse {
            success: false,
            message: "Failed to test Oracle connection".to_string(),
            error: Some(e.to_string()),
        }
    }
}

async fn test_sqlserver_connection(config: &Value) -> TestConnectionResponse {
    use tiberius::{Client, Config, AuthMethod};
    use tokio_util::compat::TokioAsyncWriteCompatExt;
    
    let connection_config = if let Some(url) = config.as_str() {
        // Parse SQL Server connection string
        match Config::from_ado_string(url) {
            Ok(config) => config,
            Err(e) => return TestConnectionResponse {
                success: false,
                message: "Invalid SQL Server connection string".to_string(),
                error: Some(e.to_string()),
            }
        }
    } else if let Some(obj) = config.as_object() {
        let host = obj.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
        let port = obj.get("port").and_then(|v| v.as_u64()).unwrap_or(1433) as u16;
        let database = obj.get("database").and_then(|v| v.as_str()).unwrap_or("");
        let user = obj.get("user").and_then(|v| v.as_str())
            .or_else(|| obj.get("username").and_then(|v| v.as_str()))
            .unwrap_or("");
        let password = obj.get("password").and_then(|v| v.as_str()).unwrap_or("");
        
        let mut config = Config::new();
        config.host(host);
        config.port(port);
        config.database(database);
        config.authentication(AuthMethod::sql_server(user, password));
        config.trust_cert(); // For development/testing - should be configured properly in production
        
        config
    } else {
        return TestConnectionResponse {
            success: false,
            message: "Invalid configuration format".to_string(),
            error: Some("Config must be a connection URL string or object with connection details".to_string()),
        };
    };

    match tokio::net::TcpStream::connect(connection_config.get_addr()).await {
        Ok(tcp) => {
            match Client::connect(connection_config, tcp.compat_write()).await {
                Ok(mut client) => {
                    // Test with a simple query
                    match client.query("SELECT 1", &[]).await {
                        Ok(_) => TestConnectionResponse {
                            success: true,
                            message: "Connection successful".to_string(),
                            error: None,
                        },
                        Err(e) => TestConnectionResponse {
                            success: false,
                            message: "Connection established but query failed".to_string(),
                            error: Some(e.to_string()),
                        }
                    }
                },
                Err(e) => TestConnectionResponse {
                    success: false,
                    message: "Failed to authenticate with SQL Server".to_string(),
                    error: Some(e.to_string()),
                }
            }
        },
        Err(e) => TestConnectionResponse {
            success: false,
            message: "Failed to connect to SQL Server".to_string(),
            error: Some(e.to_string()),
        }
    }
}