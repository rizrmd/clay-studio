use serde_json::Value;
use uuid::Uuid;
use crate::core::mcp::types::JsonRpcError;
use super::error_responses::ErrorResponses;

/// Common validation utilities used across the application
#[allow(dead_code)]
pub struct ValidationHelpers;

#[allow(dead_code)]
impl ValidationHelpers {
    /// Validate UUID format
    pub fn validate_uuid(uuid_str: &str, field_name: &str) -> Result<Uuid, JsonRpcError> {
        Uuid::parse_str(uuid_str)
            .map_err(|_| ErrorResponses::invalid_params(format!("Invalid {} format", field_name)))
    }

    /// Validate that a string is not empty
    pub fn validate_non_empty_string(value: &str, field_name: &str) -> Result<(), JsonRpcError> {
        if value.trim().is_empty() {
            return Err(ErrorResponses::invalid_params(format!("{} cannot be empty", field_name)));
        }
        Ok(())
    }

    /// Validate email format (basic validation)
    pub fn validate_email(email: &str) -> Result<(), JsonRpcError> {
        if !email.contains('@') || !email.contains('.') {
            return Err(ErrorResponses::invalid_params("Invalid email format"));
        }
        Ok(())
    }

    /// Validate password strength
    pub fn validate_password(password: &str) -> Result<(), JsonRpcError> {
        if password.len() < 8 {
            return Err(ErrorResponses::invalid_params("Password must be at least 8 characters long"));
        }
        Ok(())
    }

    /// Validate JSON object has required fields
    pub fn validate_required_fields(json: &Value, required_fields: &[&str]) -> Result<(), JsonRpcError> {
        for field in required_fields {
            if json.get(field).is_none() {
                return Err(ErrorResponses::missing_parameter(*field));
            }
        }
        Ok(())
    }

    /// Validate pagination parameters
    pub fn validate_pagination(page: i32, limit: i32) -> Result<(i32, i32), JsonRpcError> {
        if page < 1 {
            return Err(ErrorResponses::invalid_params("Page must be >= 1"));
        }
        if !(1..=100).contains(&limit) {
            return Err(ErrorResponses::invalid_params("Limit must be between 1 and 100"));
        }
        Ok((page, limit))
    }

    /// Validate database table name to prevent SQL injection
    pub fn validate_table_name(table_name: &str) -> Result<(), JsonRpcError> {
        if !table_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(ErrorResponses::invalid_params("Invalid table name format"));
        }
        if table_name.len() > 64 {
            return Err(ErrorResponses::invalid_params("Table name too long"));
        }
        Ok(())
    }

    /// Validate column name to prevent SQL injection
    pub fn validate_column_name(column_name: &str) -> Result<(), JsonRpcError> {
        if !column_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(ErrorResponses::invalid_params("Invalid column name format"));
        }
        if column_name.len() > 64 {
            return Err(ErrorResponses::invalid_params("Column name too long"));
        }
        Ok(())
    }

    /// Sanitize SQL LIMIT value
    pub fn sanitize_limit(limit: i32) -> i32 {
        if limit <= 0 {
            100
        } else if limit > 10000 {
            10000
        } else {
            limit
        }
    }

    /// Sanitize SQL OFFSET value
    pub fn sanitize_offset(offset: i32) -> i32 {
        if offset < 0 {
            0
        } else {
            offset
        }
    }
}