use serde_json::Value;
use std::error::Error;
use tracing::debug;

/// Common database connection configuration handling
#[allow(dead_code)]
pub struct ConnectionConfig {
    pub url: String,
    pub original_url: String,
    pub ssl_mode: Option<String>,
}

#[allow(dead_code)]
impl ConnectionConfig {
    /// Parse connection configuration from JSON value
    pub fn from_config(config: &Value, default_port: u16, default_user: &str) -> Result<Self, Box<dyn Error>> {
        let original_url = config.to_string();
        
        // Prefer URL if provided, otherwise construct from individual components
        let mut connection_string = if let Some(url) = config.get("url").and_then(|v| v.as_str()) {
            debug!("Using provided database URL directly");
            url.to_string()
        } else {
            // Build from components
            Self::build_connection_url(config, default_port, default_user)?
        };

        // Handle SSL configuration
        let ssl_mode = Self::apply_ssl_config(&mut connection_string, config);

        Ok(ConnectionConfig {
            url: connection_string,
            original_url,
            ssl_mode,
        })
    }

    /// Build connection URL from individual components
    fn build_connection_url(config: &Value, default_port: u16, default_user: &str) -> Result<String, Box<dyn Error>> {
        let host = config
            .get("host")
            .and_then(|v| v.as_str())
            .unwrap_or("localhost");
        let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(default_port as u64);
        let database = config
            .get("database")
            .and_then(|v| v.as_str())
            .ok_or("Missing database name")?;
        let username = config
            .get("username")
            .and_then(|v| v.as_str())
            .or_else(|| config.get("user").and_then(|v| v.as_str()))
            .unwrap_or(default_user);
        let password = config
            .get("password")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        debug!(
            "Building database URL from components: host={}, port={}, database={}, username={}",
            host, port, database, username
        );

        // URL encode username and password to handle special characters
        let encoded_username = urlencoding::encode(username);
        let encoded_password = if password.is_empty() {
            String::new()
        } else {
            urlencoding::encode(password).to_string()
        };

        let scheme = Self::determine_scheme(config);
        
        if encoded_password.is_empty() {
            Ok(format!(
                "{}://{}@{}:{}/{}",
                scheme, encoded_username, host, port, database
            ))
        } else {
            Ok(format!(
                "{}://{}:{}@{}:{}/{}",
                scheme, encoded_username, encoded_password, host, port, database
            ))
        }
    }

    /// Determine URL scheme based on database type
    fn determine_scheme(_config: &Value) -> &'static str {
        // This would need to be passed in or determined from context
        // For now, defaulting to generic
        "mysql" // This should be parameterized
    }

    /// Apply SSL configuration to connection string
    fn apply_ssl_config(connection_string: &mut String, config: &Value) -> Option<String> {
        let disable_ssl = config
            .get("disable_ssl")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let ssl_mode = config.get("ssl_mode").and_then(|v| v.as_str());

        if disable_ssl || ssl_mode == Some("disable") {
            Self::add_ssl_parameter(connection_string, "sslmode", "disable");
            debug!("SSL disabled for database connection");
            Some("disable".to_string())
        } else if let Some(mode) = ssl_mode {
            Self::add_ssl_parameter(connection_string, "sslmode", mode);
            debug!("SSL mode set to: {}", mode);
            Some(mode.to_string())
        } else {
            None
        }
    }

    /// Add SSL parameter to connection string if not already present
    fn add_ssl_parameter(connection_string: &mut String, param: &str, value: &str) {
        let param_string = format!("{}={}", param, value);
        if !connection_string.contains(&format!("{}=", param)) {
            let separator = if connection_string.contains('?') {
                "&"
            } else {
                "?"
            };
            connection_string.push_str(&format!("{}{}", separator, param_string));
        }
    }
}

/// Extract schema name from config with database-specific defaults
#[allow(dead_code)]
pub fn extract_schema_name(config: &Value, default_schema: &str) -> String {
    config
        .get("schema")
        .and_then(|v| v.as_str())
        .unwrap_or(default_schema)
        .to_string()
}