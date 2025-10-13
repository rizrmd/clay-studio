/// Common utilities for database connectors
/// Reduces code duplication across PostgreSQL, MySQL, SQLite, and SQL Server connectors

use serde_json::Value;
use std::error::Error;

/// Extract datasource ID from config (optional - only needed for connection pooling)
pub fn extract_datasource_id(config: &Value) -> String {
    config
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("temp-connection-test")
        .to_string()
}

/// Extract SSL configuration from config
pub struct SslConfig {
    pub disable_ssl: bool,
    pub ssl_mode: Option<String>,
}

impl SslConfig {
    pub fn from_config(config: &Value) -> Self {
        Self {
            disable_ssl: config
                .get("disable_ssl")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            ssl_mode: config
                .get("ssl_mode")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        }
    }

    pub fn should_disable_ssl(&self) -> bool {
        self.disable_ssl || self.ssl_mode.as_deref() == Some("disable")
    }
}

/// Build connection string from URL or individual components
pub struct ConnectionStringBuilder {
    protocol: String,
    default_host: String,
    default_port: u64,
    default_user: String,
}

impl ConnectionStringBuilder {
    pub fn new(protocol: &str, default_host: &str, default_port: u64, default_user: &str) -> Self {
        Self {
            protocol: protocol.to_string(),
            default_host: default_host.to_string(),
            default_port,
            default_user: default_user.to_string(),
        }
    }

    pub fn build(&self, config: &Value) -> Result<String, Box<dyn Error + Send + Sync>> {
        // Prefer URL if provided
        if let Some(url) = config.get("url").and_then(|v| v.as_str()) {
            return Ok(url.to_string());
        }

        // Build from components
        let host = config
            .get("host")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.default_host);

        let port = config
            .get("port")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.default_port);

        let database = config
            .get("database")
            .and_then(|v| v.as_str())
            .ok_or("Missing database name")?;

        let username = config
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.default_user);

        let password = config
            .get("password")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // URL encode credentials
        let encoded_username = urlencoding::encode(username);
        let encoded_password = if password.is_empty() {
            String::new()
        } else {
            urlencoding::encode(password).to_string()
        };

        // Build connection string
        let connection_string = if encoded_password.is_empty() {
            format!(
                "{}://{}@{}:{}/{}",
                self.protocol, encoded_username, host, port, database
            )
        } else {
            format!(
                "{}://{}:{}@{}:{}/{}",
                self.protocol, encoded_username, encoded_password, host, port, database
            )
        };

        Ok(connection_string)
    }
}

/// Add SSL parameters to connection string
pub fn add_ssl_parameter(
    connection_string: &mut String,
    param_name: &str,
    param_value: &str,
) {
    let separator = if connection_string.contains('?') {
        "&"
    } else {
        "?"
    };
    connection_string.push_str(&format!("{}{}", separator, param_name));
    if !param_value.is_empty() {
        connection_string.push_str(&format!("={}", param_value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_datasource_id() {
        let config = json!({
            "id": "test-id-123"
        });
        assert_eq!(extract_datasource_id(&config), "test-id-123");
    }

    #[test]
    fn test_extract_datasource_id_missing() {
        let config = json!({});
        assert_eq!(extract_datasource_id(&config), "temp-connection-test");
    }

    #[test]
    fn test_ssl_config() {
        let config = json!({
            "disable_ssl": true
        });
        let ssl_config = SslConfig::from_config(&config);
        assert!(ssl_config.should_disable_ssl());
    }

    #[test]
    fn test_connection_string_builder_from_components() {
        let config = json!({
            "host": "localhost",
            "port": 5432,
            "database": "testdb",
            "username": "user",
            "password": "pass"
        });
        let builder = ConnectionStringBuilder::new("postgres", "localhost", 5432, "postgres");
        let result = builder.build(&config).unwrap();
        assert_eq!(result, "postgres://user:pass@localhost:5432/testdb");
    }

    #[test]
    fn test_connection_string_builder_from_url() {
        let config = json!({
            "url": "postgres://user:pass@localhost:5432/testdb"
        });
        let builder = ConnectionStringBuilder::new("postgres", "localhost", 5432, "postgres");
        let result = builder.build(&config).unwrap();
        assert_eq!(result, "postgres://user:pass@localhost:5432/testdb");
    }

    #[test]
    fn test_add_ssl_parameter() {
        let mut conn_str = "postgres://user@localhost/db".to_string();
        add_ssl_parameter(&mut conn_str, "sslmode", "disable");
        assert_eq!(conn_str, "postgres://user@localhost/db?sslmode=disable");

        add_ssl_parameter(&mut conn_str, "sslrootcert", "/path/to/cert");
        assert_eq!(conn_str, "postgres://user@localhost/db?sslmode=disable&sslrootcert=/path/to/cert");
    }
}
