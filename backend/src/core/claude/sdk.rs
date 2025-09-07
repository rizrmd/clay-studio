use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command as TokioCommand;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use super::types::{AskUserOption, ClaudeMessage, QueryRequest};
use crate::utils::log_organizer::auto_organize_logs;
use crate::utils::command_logger::{CommandLogger, CommandExecution};
use crate::core::mcp::handlers::McpHandlers;

#[derive(Debug, Clone)]
pub struct ClaudeSDK {
    client_id: Uuid,
    client_dir: PathBuf,
    project_dir: Option<PathBuf>,
    bun_path: PathBuf,
    oauth_token: Arc<Mutex<Option<String>>>,
}

impl ClaudeSDK {
    /// Get all available MCP tools for allowed tools configuration
    fn get_available_mcp_tools() -> Vec<String> {
        McpHandlers::get_all_available_mcp_tools()
    }

    pub fn new(client_id: Uuid, oauth_token: Option<String>) -> Self {
        let clients_base =
            std::env::var("CLIENTS_DIR").unwrap_or_else(|_| "../.clients".to_string());

        let clients_base_path = PathBuf::from(&clients_base);
        let client_dir = clients_base_path.join(format!("{}", client_id));
        let bun_path = clients_base_path.join("bun");

        Self {
            client_id,
            client_dir,
            project_dir: None,
            bun_path,
            oauth_token: Arc::new(Mutex::new(oauth_token)),
        }
    }

    pub fn with_project(mut self, project_id: &str) -> Self {
        let project_dir = self.client_dir.join(project_id);

        // Ensure project directory exists
        if !project_dir.exists() {
            let _ = std::fs::create_dir_all(&project_dir);
        }

        self.project_dir = Some(project_dir);
        // Note: MCP config setup is now async and called during query
        self
    }

    async fn ensure_mcp_config(&self, project_id: &str) {
        // Only create MCP config in the project directory
        if let Some(ref project_dir) = self.project_dir {
            let claude_dir = project_dir.join(".claude");
            let mcp_servers_file = claude_dir.join("mcp_servers.json");

            // Create .claude directory if it doesn't exist
            if !claude_dir.exists() {
                let _ = std::fs::create_dir_all(&claude_dir);
            }

            // Check if existing config is valid, otherwise overwrite it
            let force_update = if mcp_servers_file.exists() {
                match std::fs::read_to_string(&mcp_servers_file) {
                    Ok(content) => {
                        // Try to parse as JSON and validate structure
                        let is_valid_config = match serde_json::from_str::<serde_json::Value>(&content) {
                            Ok(json) => {
                                // Check if it has the expected structure with centralized server URLs
                                if let Some(mcp_servers) = json.get("mcpServers") {
                                    let data_analysis_valid = mcp_servers
                                        .get("data-analysis")
                                        .and_then(|da| da.get("url"))
                                        .and_then(|url| url.as_str())
                                        .map(|url| url.contains(":7670/data-analysis/"))
                                        .unwrap_or(false);
                                    
                                    let interaction_valid = mcp_servers
                                        .get("interaction")
                                        .and_then(|ia| ia.get("url"))
                                        .and_then(|url| url.as_str())
                                        .map(|url| url.contains(":7670/interaction/"))
                                        .unwrap_or(false);
                                    
                                    data_analysis_valid && interaction_valid
                                } else {
                                    false
                                }
                            }
                            Err(_) => false,
                        };
                        
                        if is_valid_config {
                            tracing::debug!("✅ MCP config is valid for project {}", project_id);
                            false
                        } else {
                            tracing::info!("🔄 Invalid or outdated MCP config detected, updating for project {}", project_id);
                            // Backup the invalid config
                            let backup_path = claude_dir.join("mcp_servers_backup.json");
                            let _ = std::fs::write(&backup_path, &content);
                            true
                        }
                    }
                    Err(_) => {
                        tracing::warn!("⚠️ Cannot read existing MCP config, recreating for project {}", project_id);
                        true
                    }
                }
            } else {
                tracing::info!("📝 Creating new HTTP MCP config for project {}", project_id);
                true
            };

            // No longer need MCP server path since we use centralized server

            // Check if centralized MCP server is ready on port 7670
            if !force_update {
                if Self::check_centralized_mcp_server_ready().await {
                    tracing::debug!("✅ Centralized MCP server already ready for project {}", project_id);
                    return;
                } else {
                    tracing::warn!("⚠️ Centralized MCP server not ready, config will still be written for project {}", project_id);
                }
            }

            // Use centralized MCP server on port 7670 with URL path-based routing
            let mcp_servers = json!({
                "mcpServers": {
                    "data-analysis": {
                        "type": "http",
                        "url": format!("http://localhost:7670/data-analysis/{}/{}", self.client_id, project_id)
                    },
                    "interaction": {
                        "type": "http",
                        "url": format!("http://localhost:7670/interaction/{}/{}", self.client_id, project_id)
                    }
                }
            });

            // Write the configuration - no need to start servers as they're managed by the backend
            let _ = std::fs::write(
                &mcp_servers_file,
                serde_json::to_string_pretty(&mcp_servers).unwrap_or_default(),
            );
            tracing::info!("✅ MCP configuration written for project {} using centralized server on port 7670", project_id);
        }
    }

    async fn ensure_requirements_met(
        &self,
        tx: &mpsc::Sender<ClaudeMessage>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bun_executable = self.bun_path.join("bin/bun");
        let claude_cli = self
            .client_dir
            .join("node_modules/@anthropic-ai/claude-code/cli.js");

        // Check if global Bun executable exists (it should have been installed at server startup)
        if !bun_executable.exists() {
            return Err("Global Bun installation not found. This should have been installed at server startup.".into());
        }

        // Check if Claude CLI is installed for this specific client
        if !claude_cli.exists() {
            tracing::info!(
                "Claude CLI not found for client {}, installing...",
                self.client_id
            );

            let _ = tx
                .send(ClaudeMessage::Progress {
                    content: serde_json::Value::String(
                        "Setting up client environment - Installing Claude CLI package..."
                            .to_string(),
                    ),
                })
                .await;

            use crate::core::claude::setup::ClaudeSetup;

            let setup = ClaudeSetup::new(self.client_id);
            setup.install_claude_code(None).await?;

            let _ = tx
                .send(ClaudeMessage::Progress {
                    content: serde_json::Value::String(
                        "Client environment setup complete!".to_string(),
                    ),
                })
                .await;

            tracing::info!(
                "Claude CLI installation completed for client {}",
                self.client_id
            );
        }

        Ok(())
    }

    pub async fn query(
        &self,
        request: QueryRequest,
    ) -> Result<mpsc::Receiver<ClaudeMessage>, Box<dyn std::error::Error + Send + Sync>> {
        let (tx, rx) = mpsc::channel(100);

        // Clone tx for ensure_requirements_met
        let tx_ensure = tx.clone();

        // Ensure all requirements are met before proceeding
        self.ensure_requirements_met(&tx_ensure).await?;

        // Ensure MCP configuration is set up
        if let Some(ref project_dir) = self.project_dir {
            let project_id = project_dir.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown");
            self.ensure_mcp_config(project_id).await;
        }

        let oauth_token = self
            .oauth_token
            .lock()
            .await
            .clone()
            .ok_or("No OAuth token available")?;

        let working_dir = self.project_dir.as_ref().unwrap_or(&self.client_dir);

        let working_dir_clone = working_dir.clone();
        let working_dir_for_auto_organize = working_dir.clone();
        let client_id = self.client_id;
        let prompt = request.prompt.clone();
        let _project_dir = self.project_dir.clone();

        tracing::info!("Claude SDK working directory: {:?}", working_dir_clone);
        tracing::info!("Project directory: {:?}", _project_dir);
        tracing::info!("HOME env var: {:?}", std::env::var("HOME"));
        tracing::info!("Current user: uid={}", unsafe { libc::getuid() });

        let bun_executable = self.bun_path.join("bin/bun");
        // Claude CLI is installed at the client level, not project level
        let claude_cli_path = self
            .client_dir
            .join("node_modules/@anthropic-ai/claude-code/cli.js");
        let allowed_tools_debug = Self::get_available_mcp_tools().join(",");
        let command_debug = format!("{} {} -p --verbose --allowedTools \"{}\" --output-format stream-json [prompt]", bun_executable.display(), claude_cli_path.display(), allowed_tools_debug);
        let command_debug_clone = command_debug.clone();
        let command_debug_clone2 = command_debug.clone();
        let command_debug_clone3 = command_debug.clone();
        let claude_cli_path_clone = claude_cli_path.clone();
        let claude_cli_path_clone2 = claude_cli_path.clone();

        // Move tx into the spawned task to keep the channel alive
        tokio::spawn(async move {
            // Take ownership of tx
            let tx = tx;
            
            // Build Claude CLI command
            let mut cmd_builder = TokioCommand::new(&bun_executable);
            cmd_builder.arg(&claude_cli_path_clone);

            // Add MCP config if we have a project directory
            if let Some(ref project_dir) = _project_dir {
                let mcp_config_path = project_dir.join(".claude/mcp_servers.json");
                if mcp_config_path.exists() {
                    cmd_builder.arg("--mcp-config").arg(".claude/mcp_servers.json");
                }
            }

            // Create cache directory
            let cache_dir = working_dir_clone.join(".cache");
            let _ = std::fs::create_dir_all(&cache_dir);

            cmd_builder.arg("-p")
                .arg("-") // Read from stdin
                .arg("--verbose");

            // Add allowed tools dynamically
            let allowed_tools = Self::get_available_mcp_tools().join(",");
            cmd_builder.arg("--allowedTools")
                .arg(&allowed_tools);

            // Add disallowed tools
            cmd_builder.arg("--disallowedTools")
                .arg("Bash,Glob,LS");

            cmd_builder.arg("--output-format")
                .arg("stream-json")
                .current_dir(&working_dir_clone)
                .env("HOME", &working_dir_clone)
                .env("XDG_CACHE_HOME", &cache_dir)
                .env("CLAUDE_CODE_OAUTH_TOKEN", oauth_token);

            // Set stdio for both paths
            cmd_builder
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            tracing::debug!("Executing Claude CLI command: {:?}", cmd_builder);

            // Log command execution start
            let command_execution = CommandExecution::new(
                client_id,
                cmd_builder.as_std().get_program().to_string_lossy().to_string(),
                cmd_builder.as_std().get_args().map(|arg| arg.to_string_lossy().to_string()).collect(),
                working_dir_clone.clone(),
                std::env::var("HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| working_dir_clone.clone()),
            );
            CommandLogger::log_command_start(&command_execution);

            let spawn_start = std::time::Instant::now();
            let mut cmd = match cmd_builder.spawn() {
                Ok(child) => {
                    tracing::info!("Claude CLI process spawned in {:?}", spawn_start.elapsed());
                    child
                }
                Err(e) => {
                    tracing::error!("Failed to spawn Claude CLI process: {}", e);
                    tracing::error!("Command was: {}", command_debug);
                    tracing::error!("Working directory: {:?}", working_dir_clone);
                    tracing::error!("Bun executable: {:?}", bun_executable);
                    
                    // Log spawn failure
                    CommandLogger::log_command_error(
                        &command_execution,
                        &format!("Failed to spawn process: {}", e),
                        spawn_start.elapsed()
                    );
                    
                    let _ = tx
                        .send(ClaudeMessage::Error {
                            error: format!("Failed to spawn Claude CLI: {}", e),
                        })
                        .await;
                    return;
                }
            };

            // Write prompt to stdin
            if let Some(mut stdin) = cmd.stdin.take() {
                use tokio::io::AsyncWriteExt;
                let stdin_start = std::time::Instant::now();
                if let Err(e) = stdin.write_all(prompt.as_bytes()).await {
                    tracing::error!("Failed to write prompt to stdin: {}", e);
                    let _ = tx
                        .send(ClaudeMessage::Error {
                            error: format!("Failed to write prompt to stdin: {}", e),
                        })
                        .await;
                    return;
                }
                tracing::info!("Prompt written to stdin in {:?}", stdin_start.elapsed());
                // Close stdin to signal EOF
                drop(stdin);
            }

            // Also capture stderr for debugging
            if let Some(stderr) = cmd.stderr.take() {
                let _tx_stderr = tx.clone();

                // Create stderr log file with organized directory structure
                let now = chrono::Utc::now();
                let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
                let month_year = now.format("%Y-%m").to_string();
                let day = now.format("%d").to_string();
                let log_base_dir = working_dir_clone.join(".claude_logs");
                let log_dir = log_base_dir.join(&month_year).join(&day);
                let _ = std::fs::create_dir_all(&log_dir);
                let stderr_log_path = log_dir.join(format!("query_{}_stderr.log", timestamp));
                let mut stderr_log_file = std::fs::File::create(&stderr_log_path).ok();

                tracing::info!("Claude CLI stderr will be logged to: {:?}", stderr_log_path);

                tokio::spawn(async move {
                    use tokio::io::{AsyncBufReadExt, BufReader};
                    let reader = BufReader::new(stderr);
                    let mut lines = reader.lines();

                    let stderr_start = std::time::Instant::now();
                    let mut first_stderr = true;
                    while let Ok(Some(line)) = lines.next_line().await {
                        if first_stderr {
                            tracing::info!(
                                "[CLAUDE_STDERR] First stderr line after {:?}: {}",
                                stderr_start.elapsed(),
                                line
                            );
                            first_stderr = false;
                        } else {
                            tracing::error!("[CLAUDE_STDERR] {}", line);
                        }

                        // Write to stderr log file
                        if let Some(ref mut file) = stderr_log_file {
                            use std::io::Write;
                            let timestamp = chrono::Utc::now().to_rfc3339();
                            let _ = writeln!(file, "[{}] {}", timestamp, line);
                            let _ = file.flush();
                        }
                    }
                });
            }

            let stdout_handle = if let Some(stdout) = cmd.stdout.take() {
                let tx_clone = tx.clone();

                // Create debug log file for this query with organized directory structure
                let now = chrono::Utc::now();
                let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
                let month_year = now.format("%Y-%m").to_string();
                let day = now.format("%d").to_string();
                let log_base_dir = working_dir_clone.join(".claude_logs");
                let log_dir = log_base_dir.join(&month_year).join(&day);
                let _ = std::fs::create_dir_all(&log_dir);
                let log_file_path = log_dir.join(format!("query_{}.log", timestamp));
                let mut log_file = std::fs::File::create(&log_file_path).ok();

                // Write command info to log file
                if let Some(ref mut file) = log_file {
                    use std::io::Write;
                    let _ = writeln!(file, "=== Claude CLI Query Log ===");
                    let _ = writeln!(file, "Timestamp: {}", chrono::Utc::now().to_rfc3339());
                    let _ = writeln!(file, "Working Dir: {:?}", working_dir_clone);
                    let _ = writeln!(file, "Command: {}", command_debug_clone3);
                    let _ = writeln!(file, "Prompt Length: {} chars", prompt.len());
                    let _ = writeln!(
                        file,
                        "First 500 chars of prompt: {}...",
                        &prompt.chars().take(500).collect::<String>()
                    );
                    let _ = writeln!(file, "\n=== Output ===");
                    let _ = file.flush();
                }

                tracing::info!("Claude CLI output will be logged to: {:?}", log_file_path);

                Some(tokio::spawn(async move {
                    use tokio::io::{AsyncBufReadExt, BufReader};
                    let reader = BufReader::new(stdout);
                    let mut lines = reader.lines();

                    tracing::info!(
                        "Claude CLI stdout reader initialized, waiting for first line..."
                    );
                    let mut line_count = 0;
                    let mut first_message_time: Option<std::time::Instant> = None;
                    let start_time = std::time::Instant::now();

                    loop {
                        match lines.next_line().await {
                            Ok(Some(line)) => {
                                line_count += 1;

                                // Log timing for first message
                                if first_message_time.is_none() {
                                    let elapsed = start_time.elapsed();
                                    tracing::info!(
                                        "First message from Claude CLI received after {:?}",
                                        elapsed
                                    );
                                    first_message_time = Some(std::time::Instant::now());
                                }

                                // Write to log file
                                if let Some(ref mut file) = log_file {
                                    use std::io::Write;
                                    let timestamp = chrono::Utc::now().to_rfc3339();
                                    let _ = writeln!(file, "[{}] {}", timestamp, line);
                                    let _ = file.flush();
                                }

                                if !line.trim().is_empty() {
                                    // Parse line as JSON or create a string value
                                    let mut json_value = if let Ok(json) =
                                        serde_json::from_str::<serde_json::Value>(&line)
                                    {
                                        json
                                    } else {
                                        // If not valid JSON, wrap as string
                                        serde_json::Value::String(line.clone())
                                    };

                                    // Check for tool use and tool results in addition to sending progress
                                    if let Some(msg_type) =
                                        json_value.get("type").and_then(|v| v.as_str())
                                    {
                                        match msg_type {
                                            "assistant" => {
                                                // Check for tool use blocks in assistant messages
                                                if let Some(message) = json_value.get("message") {
                                                    if let Some(content) = message.get("content") {
                                                        if let Some(blocks) = content.as_array() {
                                                            for block in blocks {
                                                                if block
                                                                    .get("type")
                                                                    .and_then(|t| t.as_str())
                                                                    == Some("tool_use")
                                                                {
                                                                    if let Some(name) = block
                                                                        .get("name")
                                                                        .and_then(|n| n.as_str())
                                                                    {
                                                                        let tool_use_id = block
                                                                            .get("id")
                                                                            .and_then(|id| {
                                                                                id.as_str()
                                                                            })
                                                                            .map(|s| {
                                                                                s.to_string()
                                                                            });
                                                                        let args = block
                                                                            .get("input")
                                                                            .or_else(|| {
                                                                                block.get(
                                                                                    "arguments",
                                                                                )
                                                                            })
                                                                            .cloned()
                                                                            .unwrap_or(json!(
                                                                                {}
                                                                            ));

                                                                        tracing::info!("Detected tool usage: {} with id: {:?}", name, tool_use_id);
                                                                        let _ = tx_clone.send(ClaudeMessage::ToolUse {
                                                                    tool: name.to_string(),
                                                                    args,
                                                                    tool_use_id,
                                                                }).await;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            "tool_result" => {
                                                // Handle explicit tool_result messages
                                                if let (Some(tool_name), Some(result)) = (
                                                    json_value
                                                        .get("tool_name")
                                                        .and_then(|v| v.as_str()),
                                                    json_value.get("result"),
                                                ) {
                                                    tracing::info!(
                                                        "Detected tool result for {}",
                                                        tool_name
                                                    );
                                                    let _ = tx_clone
                                                        .send(ClaudeMessage::ToolResult {
                                                            tool: tool_name.to_string(),
                                                            result: result.clone(),
                                                        })
                                                        .await;
                                                }
                                            }
                                            "user" => {
                                                // Handle user messages that contain tool_result content
                                                if let Some(message) = json_value.get("message") {
                                                    if let Some(content) = message.get("content") {
                                                        if let Some(blocks) = content.as_array() {
                                                            for block in blocks {
                                                                if block
                                                                    .get("type")
                                                                    .and_then(|t| t.as_str())
                                                                    == Some("tool_result")
                                                                {
                                                                    if let Some(tool_use_id) = block
                                                                        .get("tool_use_id")
                                                                        .and_then(|id| id.as_str())
                                                                    {
                                                                        tracing::info!("Detected tool result in user message with tool_use_id: {}", tool_use_id);
                                                                        let _ = tx_clone.send(ClaudeMessage::ToolResult {
                                                                    tool: tool_use_id.to_string(),
                                                                    result: block.get("content").cloned().unwrap_or(json!([])),
                                                                }).await;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            "result" => {
                                                // Handle final result messages
                                                if let Some(result) = json_value
                                                    .get("result")
                                                    .and_then(|v| v.as_str())
                                                {
                                                    if !result.is_empty() {
                                                        tracing::info!(
                                                            "Detected final result message"
                                                        );
                                                        let _ = tx_clone
                                                            .send(ClaudeMessage::Result {
                                                                result: result.to_string(),
                                                            })
                                                            .await;
                                                    }
                                                }
                                            }
                                            "ask_user" => {
                                                // Handle ask_user interaction events
                                                tracing::info!("Detected ask_user event");

                                                let prompt_type = json_value
                                                    .get("prompt_type")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("input")
                                                    .to_string();

                                                let title = json_value
                                                    .get("title")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("")
                                                    .to_string();

                                                let options = if let Some(opts_array) = json_value
                                                    .get("options")
                                                    .and_then(|v| v.as_array())
                                                {
                                                    let parsed_options: Vec<AskUserOption> = opts_array.iter()
                                                .filter_map(|opt| {
                                                    if let (Some(value), Some(label)) = 
                                                        (opt.get("value").and_then(|v| v.as_str()),
                                                         opt.get("label").and_then(|l| l.as_str())) {
                                                        Some(AskUserOption {
                                                            value: value.to_string(),
                                                            label: label.to_string(),
                                                            description: opt.get("description")
                                                                .and_then(|d| d.as_str())
                                                                .map(|s| s.to_string()),
                                                        })
                                                    } else {
                                                        None
                                                    }
                                                })
                                                .collect();
                                                    if parsed_options.is_empty() {
                                                        None
                                                    } else {
                                                        Some(parsed_options)
                                                    }
                                                } else {
                                                    None
                                                };

                                                let input_type = json_value
                                                    .get("input_type")
                                                    .and_then(|v| v.as_str())
                                                    .map(|s| s.to_string());

                                                let placeholder = json_value
                                                    .get("placeholder")
                                                    .and_then(|v| v.as_str())
                                                    .map(|s| s.to_string());

                                                let tool_use_id = json_value
                                                    .get("tool_use_id")
                                                    .and_then(|v| v.as_str())
                                                    .map(|s| s.to_string());

                                                let _ = tx_clone
                                                    .send(ClaudeMessage::AskUser {
                                                        prompt_type,
                                                        title,
                                                        options,
                                                        input_type,
                                                        placeholder,
                                                        tool_use_id,
                                                    })
                                                    .await;
                                            }
                                            "error" => {
                                                // Handle error messages
                                                if let Some(error) =
                                                    json_value.get("error").and_then(|v| v.as_str())
                                                {
                                                    tracing::error!(
                                                        "Detected error message: {}",
                                                        error
                                                    );
                                                    let _ = tx_clone
                                                        .send(ClaudeMessage::Error {
                                                            error: error.to_string(),
                                                        })
                                                        .await;
                                                }
                                            }
                                            "show_table" | "show_chart" => {
                                                // These are visualization events that should be passed through progress
                                                // They contain structured data for rendering tables/charts
                                                tracing::info!("Detected {} event", msg_type);
                                                // These will be handled via the progress message with their full JSON
                                            }
                                            _ => {} // Other message types
                                        }
                                    }

                                    // Remove cwd field if it exists
                                    if let Some(obj) = json_value.as_object_mut() {
                                        obj.remove("cwd");
                                    }

                                    // Send the cleaned JSON as progress
                                    let _ = tx_clone
                                        .send(ClaudeMessage::Progress {
                                            content: json_value,
                                        })
                                        .await;
                                }
                            }
                            Ok(None) => {
                                // End of stream
                                tracing::debug!("Stdout stream ended after {} lines", line_count);
                                break;
                            }
                            Err(e) => {
                                tracing::error!("Error reading stdout line: {}", e);
                                break;
                            }
                        }
                    }

                    // Debug information when no output is received
                    if line_count == 0 {
                        tracing::error!("Claude CLI produced no output!");
                        tracing::error!("Command executed: {}", command_debug_clone);
                        tracing::error!("Working directory: {:?}", working_dir_clone);
                        tracing::error!("Bun path: {:?}", bun_executable);
                        tracing::error!("Claude CLI path: {:?}", claude_cli_path_clone2);

                        // Check if the Claude CLI exists
                        if !claude_cli_path_clone2.exists() {
                            tracing::error!(
                                "Claude CLI not found at expected path: {:?}",
                                claude_cli_path_clone2
                            );
                        } else {
                            tracing::info!("Claude CLI exists at: {:?}", claude_cli_path_clone2);
                        }
                    }
                }))
            } else {
                None
            };

            if let Some(stderr) = cmd.stderr.take() {
                tokio::spawn(async move {
                    use tokio::io::{AsyncBufReadExt, BufReader};
                    let reader = BufReader::new(stderr);
                    let mut lines = reader.lines();
                    let mut stderr_lines = Vec::new();

                    while let Ok(Some(line)) = lines.next_line().await {
                        stderr_lines.push(line.clone());
                        tracing::debug!("Claude SDK stderr: {}", line);
                    }

                    // If we got stderr output, log it as error
                    if !stderr_lines.is_empty() {
                        tracing::error!("Claude CLI stderr output:");
                        for line in &stderr_lines {
                            tracing::error!("  {}", line);
                        }
                        tracing::error!("Command that produced stderr: {}", command_debug_clone2);
                    }
                });
            }

            let total_duration = spawn_start.elapsed();
            
            match cmd.wait().await {
                Ok(status) if !status.success() => {
                    tracing::error!("Claude CLI exited with non-zero status: {}", status);
                    tracing::error!("Command was: {}", command_debug_clone3);
                    
                    // Log command failure
                    CommandLogger::log_command_end(
                        &command_execution,
                        status.code(),
                        total_duration
                    );
                    
                    let _ = tx
                        .send(ClaudeMessage::Error {
                            error: format!("Process exited with status: {}", status),
                        })
                        .await;
                }
                Ok(status) => {
                    // Log successful command completion
                    CommandLogger::log_command_end(
                        &command_execution,
                        status.code(),
                        total_duration
                    );
                }
                Err(e) => {
                    tracing::error!("Error waiting for Claude CLI process: {}", e);
                    tracing::error!("Command was: {}", command_debug_clone3);
                    
                    // Log command error
                    CommandLogger::log_command_error(
                        &command_execution,
                        &format!("Process error: {}", e),
                        total_duration
                    );
                    
                    let _ = tx
                        .send(ClaudeMessage::Error {
                            error: format!("Process error: {}", e),
                        })
                        .await;
                }
            }

            // Wait for stdout processing to complete before dropping tx
            if let Some(handle) = stdout_handle {
                tracing::debug!("Waiting for stdout processing to complete...");
                if let Err(e) = handle.await {
                    tracing::error!("Error waiting for stdout processing: {}", e);
                } else {
                    tracing::debug!("Stdout processing completed successfully");
                }
            }

            // Auto-organize any loose log files in the claude_logs directory
            let log_base_dir = working_dir_for_auto_organize.join(".claude_logs");
            if let Err(e) = auto_organize_logs(&log_base_dir) {
                tracing::warn!("Failed to auto-organize logs in {:?}: {}", log_base_dir, e);
            }

            // Keep tx alive a bit longer to ensure all messages are processed
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            tracing::debug!("Query completed for client {}, dropping tx now", client_id);
        });

        Ok(rx)
    }

    #[allow(dead_code)]
    pub async fn set_oauth_token(&self, token: String) {
        let mut guard = self.oauth_token.lock().await;
        *guard = Some(token);
    }

    #[allow(dead_code)]
    pub async fn get_oauth_token(&self) -> Option<String> {
        let guard = self.oauth_token.lock().await;
        guard.clone()
    }

    /// Check if centralized MCP server is ready on port 7670
    async fn check_centralized_mcp_server_ready() -> bool {
        use tokio::time::{timeout, Duration};
        
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build() 
        {
            Ok(client) => client,
            Err(_) => return false,
        };
        
        // Test with a dummy client/project ID to see if server responds
        let url = "http://localhost:7670/data-analysis/test-client/test-project";
        
        let test_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {"roots": {}},
                "clientInfo": {"name": "readiness-check", "version": "1.0.0"}
            }
        });
        
        let request_future = client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&test_request)
            .send();
            
        match timeout(Duration::from_secs(2), request_future).await {
            Ok(Ok(response)) => response.status().is_success(),
            _ => false,
        }
    }
}
