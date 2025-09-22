use crate::core::mcp::types::{JsonRpcError, INVALID_PARAMS, INTERNAL_ERROR, METHOD_NOT_FOUND};
use serde_json::Value;

/// Common error response builders for consistent error handling across the application
#[allow(dead_code)]
pub struct ErrorResponses;

#[allow(dead_code)]
impl ErrorResponses {
    /// Create a standard invalid parameters error
    pub fn invalid_params<T: Into<String>>(message: T) -> JsonRpcError {
        JsonRpcError {
            code: INVALID_PARAMS,
            message: message.into(),
            data: None,
        }
    }

    /// Create a standard internal server error
    pub fn internal_error<T: Into<String>>(message: T) -> JsonRpcError {
        JsonRpcError {
            code: INTERNAL_ERROR,
            message: message.into(),
            data: None,
        }
    }

    /// Create a method not found error
    pub fn method_not_found<T: Into<String>>(message: T) -> JsonRpcError {
        JsonRpcError {
            code: METHOD_NOT_FOUND,
            message: message.into(),
            data: None,
        }
    }

    /// Create a database error from sqlx error
    pub fn database_error(error: sqlx::Error) -> JsonRpcError {
        JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Database error: {}", error),
            data: None,
        }
    }

    /// Create a missing parameter error
    pub fn missing_parameter<T: Into<String>>(parameter_name: T) -> JsonRpcError {
        JsonRpcError {
            code: INVALID_PARAMS,
            message: format!("Missing required parameter: {}", parameter_name.into()),
            data: None,
        }
    }

    /// Create a not found error for resources
    pub fn resource_not_found<T: Into<String>>(resource_type: T, id: T) -> JsonRpcError {
        JsonRpcError {
            code: INVALID_PARAMS,
            message: format!("{} not found: {}", resource_type.into(), id.into()),
            data: None,
        }
    }

    /// Extract string parameter from JSON value with validation
    pub fn extract_string_param(params: &Value, key: &str) -> Result<String, JsonRpcError> {
        params
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| Self::missing_parameter(key))
    }

    /// Extract optional string parameter from JSON value
    pub fn extract_optional_string_param(params: &Value, key: &str) -> Option<String> {
        params.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
    }

    /// Extract boolean parameter from JSON value with default
    pub fn extract_bool_param_with_default(params: &Value, key: &str, default: bool) -> bool {
        params
            .get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(default)
    }
}