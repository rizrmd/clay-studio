use serde_json::{json, Value};
use std::error::Error;
use std::fmt;

/// Common error types for database operations
#[derive(Debug)]
#[allow(dead_code)]
pub enum DatabaseError {
    ConnectionFailed(String),
    QueryFailed(String),
    SchemaError(String),
    ConfigurationError(String),
    TimeoutError(String),
    AuthenticationError(String),
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            DatabaseError::QueryFailed(msg) => write!(f, "Query failed: {}", msg),
            DatabaseError::SchemaError(msg) => write!(f, "Schema error: {}", msg),
            DatabaseError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            DatabaseError::TimeoutError(msg) => write!(f, "Timeout error: {}", msg),
            DatabaseError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
        }
    }
}

impl Error for DatabaseError {}

/// Convert various database errors to standardized JSON responses
#[allow(dead_code)]
pub trait ErrorMapper {
    fn map_connection_error(&self, error: &dyn Error) -> Value;
    fn map_query_error(&self, error: &dyn Error) -> Value;
    fn map_schema_error(&self, error: &dyn Error) -> Value;
}

/// Default error mapper implementation
#[allow(dead_code)]
pub struct DefaultErrorMapper;

#[allow(dead_code)]
impl ErrorMapper for DefaultErrorMapper {
    fn map_connection_error(&self, error: &dyn Error) -> Value {
        let error_msg = error.to_string();
        
        // Try to categorize the error
        if error_msg.contains("authentication") || error_msg.contains("password") || error_msg.contains("access denied") {
            json!({
                "error": "Authentication failed",
                "details": error_msg,
                "category": "auth"
            })
        } else if error_msg.contains("timeout") || error_msg.contains("timed out") {
            json!({
                "error": "Connection timeout",
                "details": error_msg,
                "category": "timeout"
            })
        } else if error_msg.contains("connection refused") || error_msg.contains("network") {
            json!({
                "error": "Network connection failed",
                "details": error_msg,
                "category": "network"
            })
        } else {
            json!({
                "error": "Database connection failed",
                "details": error_msg,
                "category": "connection"
            })
        }
    }

    fn map_query_error(&self, error: &dyn Error) -> Value {
        let error_msg = error.to_string();
        
        if error_msg.contains("syntax") || error_msg.contains("parse") {
            json!({
                "error": "SQL syntax error",
                "details": error_msg,
                "category": "syntax"
            })
        } else if error_msg.contains("permission") || error_msg.contains("privilege") {
            json!({
                "error": "Insufficient permissions",
                "details": error_msg,
                "category": "permission"
            })
        } else if error_msg.contains("table") && error_msg.contains("not found") {
            json!({
                "error": "Table not found",
                "details": error_msg,
                "category": "schema"
            })
        } else {
            json!({
                "error": "Query execution failed",
                "details": error_msg,
                "category": "query"
            })
        }
    }

    fn map_schema_error(&self, error: &dyn Error) -> Value {
        json!({
            "error": "Schema operation failed",
            "details": error.to_string(),
            "category": "schema"
        })
    }
}

/// Helper function to create a standardized error response
#[allow(dead_code)]
pub fn create_error_response(category: &str, message: &str, details: Option<&str>) -> Value {
    let mut response = json!({
        "success": false,
        "error": message,
        "category": category
    });

    if let Some(details) = details {
        response["details"] = Value::String(details.to_string());
    }

    response
}

/// Helper function to create a standardized success response with timing
#[allow(dead_code)]
pub fn create_success_response(data: Value, execution_time_ms: u64) -> Value {
    json!({
        "success": true,
        "data": data,
        "execution_time_ms": execution_time_ms
    })
}