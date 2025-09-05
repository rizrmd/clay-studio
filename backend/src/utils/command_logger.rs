use std::path::PathBuf;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CommandExecution {
    pub client_id: Uuid,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub home_directory: PathBuf,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub execution_id: Uuid,
}

impl CommandExecution {
    pub fn new(
        client_id: Uuid,
        command: String,
        args: Vec<String>,
        cwd: PathBuf,
        home_directory: PathBuf,
    ) -> Self {
        Self {
            client_id,
            command,
            args,
            cwd,
            home_directory,
            timestamp: chrono::Utc::now(),
            execution_id: Uuid::new_v4(),
        }
    }
}

/// Logs Claude CLI command executions with full context
pub struct CommandLogger;

impl CommandLogger {
    /// Log a command execution before it runs
    pub fn log_command_start(execution: &CommandExecution) {
        info!(
            "CLAUDE_CLI_COMMAND_START: client_id={} execution_id={} command={} args={:?} cwd={:?} home={:?} timestamp={}",
            execution.client_id,
            execution.execution_id,
            execution.command,
            execution.args,
            execution.cwd,
            execution.home_directory,
            execution.timestamp.to_rfc3339()
        );
    }

    /// Log command completion with duration
    pub fn log_command_end(
        execution: &CommandExecution,
        exit_code: Option<i32>,
        duration: std::time::Duration,
    ) {
        info!(
            "CLAUDE_CLI_COMMAND_END: client_id={} execution_id={} command={} exit_code={:?} duration_ms={} timestamp={}",
            execution.client_id,
            execution.execution_id,
            execution.command,
            exit_code,
            duration.as_millis(),
            chrono::Utc::now().to_rfc3339()
        );
    }

    /// Log command failure with error details
    pub fn log_command_error(
        execution: &CommandExecution,
        error: &str,
        duration: std::time::Duration,
    ) {
        info!(
            "CLAUDE_CLI_COMMAND_ERROR: client_id={} execution_id={} command={} error={} duration_ms={} timestamp={}",
            execution.client_id,
            execution.execution_id,
            execution.command,
            error,
            duration.as_millis(),
            chrono::Utc::now().to_rfc3339()
        );
    }


}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_command_execution_creation() {
        let client_id = Uuid::new_v4();
        let command = "bun".to_string();
        let args = vec!["cli.js".to_string(), "--verbose".to_string()];
        let cwd = PathBuf::from("/tmp/test");
        let home = PathBuf::from("/home/user");

        let execution = CommandExecution::new(client_id, command.clone(), args.clone(), cwd.clone(), home.clone());

        assert_eq!(execution.client_id, client_id);
        assert_eq!(execution.command, command);
        assert_eq!(execution.args, args);
        assert_eq!(execution.cwd, cwd);
        assert_eq!(execution.home_directory, home);
    }

    #[test]
    fn test_command_logging() {
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();
        
        let client_id = Uuid::new_v4();
        let execution = CommandExecution::new(
            client_id,
            "test_command".to_string(),
            vec!["--arg1".to_string(), "value1".to_string()],
            working_dir.to_path_buf(),
            PathBuf::from("/home/test"),
        );

        // Test start logging
        CommandLogger::log_command_start(&execution);

        // Test end logging
        let duration = std::time::Duration::from_millis(1500);
        CommandLogger::log_command_end(&execution, Some(0), duration);

        // Verify log files were created
        let now = chrono::Utc::now();
        let month_year = now.format("%Y-%m").to_string();
        let day = now.format("%d").to_string();
        let log_dir = working_dir.join(".claude_logs").join(month_year).join(day);
        
        assert!(log_dir.exists());
        
        // Check that some log files exist
        let log_files: Vec<_> = fs::read_dir(&log_dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.path().extension()
                    .map_or(false, |ext| ext == "log")
            })
            .collect();
            
        assert!(!log_files.is_empty());
    }
}