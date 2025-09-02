use std::path::PathBuf;
use uuid::Uuid;
use serde_json::json;
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use tokio::process::Command as TokioCommand;

use super::types::{QueryRequest, ClaudeMessage, AskUserOption};

#[derive(Debug, Clone)]
pub struct ClaudeSDK {
    client_id: Uuid,
    client_dir: PathBuf,
    project_dir: Option<PathBuf>,
    bun_path: PathBuf,
    oauth_token: Arc<Mutex<Option<String>>>,
}

impl ClaudeSDK {
    pub fn new(client_id: Uuid, oauth_token: Option<String>) -> Self {
        let clients_base = std::env::var("CLIENTS_DIR")
            .unwrap_or_else(|_| "../.clients".to_string());
        
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
        self.ensure_mcp_config(project_id);
        self
    }
    
    fn ensure_mcp_config(&self, project_id: &str) {
        // Only create MCP config in the project directory
        if let Some(ref project_dir) = self.project_dir {
            let claude_dir = project_dir.join(".claude");
            let mcp_servers_file = claude_dir.join("mcp_servers.json");
            
            // Create .claude directory if it doesn't exist
            if !claude_dir.exists() {
                let _ = std::fs::create_dir_all(&claude_dir);
            }
            
            // Check if we're in Docker/production environment
            let is_production = std::env::var("STATIC_FILES_PATH").unwrap_or_default().contains("/app/frontend")
                || std::env::var("HOME").unwrap_or_default() == "/app"
                || PathBuf::from("/app/clay-studio-backend").exists();
            
            // Prepare MCP server path based on environment
            let mcp_server_path = {
                
                if is_production {
                    // Production/Docker environment - use fixed path
                    PathBuf::from("/app/mcp_server")
                } else {
                    // Development environment - search for executable
                    let current_dir = std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."));
                    
                    // Try backend directory first (if we're in project root)
                    let backend_release = current_dir.join("backend/target/release/mcp_server");
                    let backend_debug = current_dir.join("backend/target/debug/mcp_server");
                    
                    // Try from backend directory (if we're inside backend)
                    let release_path = current_dir.join("target/release/mcp_server");
                    let debug_path = current_dir.join("target/debug/mcp_server");
                    
                    if backend_debug.exists() {
                        backend_debug.canonicalize().unwrap_or(backend_debug)
                    } else if backend_release.exists() {
                        backend_release.canonicalize().unwrap_or(backend_release)
                    } else if debug_path.exists() {
                        debug_path.canonicalize().unwrap_or(debug_path)
                    } else if release_path.exists() {
                        release_path.canonicalize().unwrap_or(release_path)
                    } else {
                        // Fallback to relative path from project root
                        PathBuf::from("backend/target/debug/mcp_server")
                    }
                }
            };
            
            // Create MCP servers configuration - pass DATABASE_URL as environment variable
            let database_url = std::env::var("DATABASE_URL").unwrap_or_default();
            let mcp_servers = json!({
                "mcpServers": {
                    "data-analysis": {
                        "type": "stdio",
                        "command": mcp_server_path.to_string_lossy(),
                        "args": [
                            "--project-id", project_id,
                            "--client-id", self.client_id.to_string()
                        ],
                        "env": {
                            "DATABASE_URL": database_url
                        }
                    },
                    "interaction": {
                        "type": "stdio",
                        "command": mcp_server_path.to_string_lossy(),
                        "args": [
                            "--project-id", project_id,
                            "--client-id", self.client_id.to_string(),
                            "--server-type", "interaction"
                        ],
                        "env": {
                            "DATABASE_URL": database_url
                        }
                    }
                }
            });
            
            // Write the MCP servers configuration
            let _ = std::fs::write(&mcp_servers_file, serde_json::to_string_pretty(&mcp_servers).unwrap_or_default());
        }
    }
    
    async fn ensure_requirements_met(&self, tx: &mpsc::Sender<ClaudeMessage>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bun_executable = self.bun_path.join("bin/bun");
        let claude_cli = self.client_dir.join("node_modules/@anthropic-ai/claude-code/cli.js");
        
        // Check if global Bun executable exists (it should have been installed at server startup)
        if !bun_executable.exists() {
            return Err("Global Bun installation not found. This should have been installed at server startup.".into());
        }
        
        // Check if Claude CLI is installed for this specific client
        if !claude_cli.exists() {
            tracing::info!("Claude CLI not found for client {}, installing...", self.client_id);
            
            let _ = tx.send(ClaudeMessage::Progress { 
                content: serde_json::Value::String("Setting up client environment - Installing Claude CLI package...".to_string())
            }).await;
            
            use crate::core::claude::setup::ClaudeSetup;
            
            let setup = ClaudeSetup::new(self.client_id);
            setup.install_claude_code(None).await?;
            
            let _ = tx.send(ClaudeMessage::Progress { 
                content: serde_json::Value::String("Client environment setup complete!".to_string())
            }).await;
            
            tracing::info!("Claude CLI installation completed for client {}", self.client_id);
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
        
        let oauth_token = self.oauth_token.lock().await.clone()
            .ok_or("No OAuth token available")?;
        
        let working_dir = self.project_dir.as_ref().unwrap_or(&self.client_dir);
        
        let working_dir_clone = working_dir.clone();
        let client_id = self.client_id;
        let prompt = request.prompt.clone();
        let _project_dir = self.project_dir.clone();
        
        tracing::info!("Claude SDK working directory: {:?}", working_dir_clone);
        tracing::info!("Project directory: {:?}", _project_dir);
        tracing::info!("HOME env var: {:?}", std::env::var("HOME"));
        tracing::info!("Current user: uid={}", unsafe { libc::getuid() });
        
        let bun_executable = self.bun_path.join("bin/bun");
        // Claude CLI is installed at the client level, not project level
        let claude_cli_path = self.client_dir.join("node_modules/@anthropic-ai/claude-code/cli.js");
        let command_debug = format!("{} {} -p --verbose --dangerously-skip-permissions --disallowedTools \"Bash\" --output-format stream-json [prompt]", bun_executable.display(), claude_cli_path.display());
        let command_debug_clone = command_debug.clone();
        let command_debug_clone2 = command_debug.clone();
        let command_debug_clone3 = command_debug.clone();
        let claude_cli_path_clone = claude_cli_path.clone();
        let claude_cli_path_clone2 = claude_cli_path.clone();
        
        // Move tx into the spawned task to keep the channel alive
        tokio::spawn(async move {
            // Take ownership of tx
            let tx = tx;
            // Check if we're running as root AND in production
            let is_root = unsafe { libc::getuid() } == 0;
            let is_production = std::env::var("RUST_ENV").unwrap_or_default() == "production";
            let use_su_workaround = is_root && is_production;
            
            let mut cmd_builder = if use_su_workaround {
                
                // First ensure /app/.clients is accessible to nobody user
                let _ = std::process::Command::new("chown")
                    .args(["-R", "nobody:nogroup", "/app/.clients"])
                    .output();
                
                // Build the full command as a string for su
                let mcp_arg = if let Some(ref project_dir) = _project_dir {
                    let mcp_config_path = project_dir.join(".claude/mcp_servers.json");
                    if mcp_config_path.exists() {
                        " --mcp-config .claude/mcp_servers.json".to_string()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                
                let full_command = format!(
                    "cd '{}' && CLAUDE_CODE_OAUTH_TOKEN='{}' echo '{}' | {} {}{} -p - --verbose --dangerously-skip-permissions --disallowedTools \"Bash\" --output-format stream-json",
                    working_dir_clone.display(),
                    oauth_token,
                    prompt.replace("'", "'\\''"),
                    bun_executable.display(),
                    claude_cli_path_clone.display(),
                    mcp_arg
                );
                
                // Use su to run as nobody user
                let mut su_cmd = TokioCommand::new("su");
                su_cmd
                    .arg("-s")
                    .arg("/bin/sh")
                    .arg("nobody")
                    .arg("-c")
                    .arg(&full_command);
                su_cmd
            } else {
                // Normal execution path for non-root or development
                let mut cmd = TokioCommand::new(&bun_executable);
                cmd.arg(&claude_cli_path_clone);
                
                // Add MCP config if we have a project directory - must come before --print
                if let Some(ref project_dir) = _project_dir {
                    let mcp_config_path = project_dir.join(".claude/mcp_servers.json");
                    if mcp_config_path.exists() {
                        cmd.arg("--mcp-config")
                           .arg(".claude/mcp_servers.json");
                    }
                }
                
                cmd.arg("-p")
                   .arg("-")  // Read from stdin
                   .arg("--verbose")
                   .arg("--dangerously-skip-permissions")
                   .arg("--disallowedTools")
                   .arg("Bash")
                   .arg("--output-format")
                   .arg("stream-json")
                   .current_dir(&working_dir_clone)
                   .env("CLAUDE_CODE_OAUTH_TOKEN", oauth_token);
                cmd
            };
            
            // Set stdio for both paths
            cmd_builder
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            
            tracing::debug!("Executing Claude CLI command: {:?}", cmd_builder);
            
            let mut cmd = match cmd_builder.spawn() {
                Ok(child) => child,
                Err(e) => {
                    tracing::error!("Failed to spawn Claude CLI process: {}", e);
                    tracing::error!("Command was: {}", command_debug);
                    tracing::error!("Working directory: {:?}", working_dir_clone);
                    tracing::error!("Bun executable: {:?}", bun_executable);
                    let _ = tx.send(ClaudeMessage::Error {
                        error: format!("Failed to spawn Claude CLI: {}", e),
                    }).await;
                    return;
                }
            };
            
            // Write prompt to stdin (only for non-su path, su path uses echo in the command)
            if !use_su_workaround {
                if let Some(mut stdin) = cmd.stdin.take() {
                    use tokio::io::AsyncWriteExt;
                    if let Err(e) = stdin.write_all(prompt.as_bytes()).await {
                        tracing::error!("Failed to write prompt to stdin: {}", e);
                        let _ = tx.send(ClaudeMessage::Error {
                            error: format!("Failed to write prompt to stdin: {}", e),
                        }).await;
                        return;
                    }
                    // Close stdin to signal EOF
                    drop(stdin);
                }
            }
            
            // Also capture stderr for debugging
            if let Some(stderr) = cmd.stderr.take() {
                let _tx_stderr = tx.clone();
                tokio::spawn(async move {
                    use tokio::io::{BufReader, AsyncBufReadExt};
                    let reader = BufReader::new(stderr);
                    let mut lines = reader.lines();
                    
                    while let Ok(Some(line)) = lines.next_line().await {
                        // Log stderr as error so it shows in production
                        tracing::error!("[CLAUDE_STDERR] {}", line);
                    }
                });
            }
            
            let stdout_handle = if let Some(stdout) = cmd.stdout.take() {
                let tx_clone = tx.clone();
                Some(tokio::spawn(async move {
                    use tokio::io::{BufReader, AsyncBufReadExt};
                    let reader = BufReader::new(stdout);
                    let mut lines = reader.lines();
                    
                    let mut line_count = 0;
                    loop {
                        match lines.next_line().await {
                            Ok(Some(line)) => {
                                line_count += 1;
                        
                        if !line.trim().is_empty() {
                            // Parse line as JSON or create a string value
                            let mut json_value = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                                json
                            } else {
                                // If not valid JSON, wrap as string
                                serde_json::Value::String(line.clone())
                            };
                            
                            // Check for tool use and tool results in addition to sending progress
                            if let Some(msg_type) = json_value.get("type").and_then(|v| v.as_str()) {
                                match msg_type {
                                    "assistant" => {
                                        // Check for tool use blocks in assistant messages
                                        if let Some(message) = json_value.get("message") {
                                            if let Some(content) = message.get("content") {
                                                if let Some(blocks) = content.as_array() {
                                                    for block in blocks {
                                                        if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                                                            if let Some(name) = block.get("name").and_then(|n| n.as_str()) {
                                                                // Skip TodoWrite - it's not a real tool, just task tracking
                                                                if name != "TodoWrite" {
                                                                    let tool_use_id = block.get("id").and_then(|id| id.as_str()).map(|s| s.to_string());
                                                                    let args = block.get("input")
                                                                        .or_else(|| block.get("arguments"))
                                                                        .cloned()
                                                                        .unwrap_or(json!({}));
                                                                    
                                                                    tracing::info!("Detected tool usage: {} with id: {:?}", name, tool_use_id);
                                                                    let _ = tx_clone.send(ClaudeMessage::ToolUse {
                                                                        tool: name.to_string(),
                                                                        args,
                                                                        tool_use_id,
                                                                    }).await;
                                                                } else {
                                                                    // TodoWrite is handled separately - just log it
                                                                    tracing::info!("Detected TodoWrite update (not counted as tool use)");
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    "tool_result" => {
                                        // Handle explicit tool_result messages
                                        if let (Some(tool_name), Some(result)) = 
                                            (json_value.get("tool_name").and_then(|v| v.as_str()),
                                             json_value.get("result")) {
                                            tracing::info!("Detected tool result for {}", tool_name);
                                            let _ = tx_clone.send(ClaudeMessage::ToolResult {
                                                tool: tool_name.to_string(),
                                                result: result.clone(),
                                            }).await;
                                        }
                                    }
                                    "user" => {
                                        // Handle user messages that contain tool_result content
                                        if let Some(message) = json_value.get("message") {
                                            if let Some(content) = message.get("content") {
                                                if let Some(blocks) = content.as_array() {
                                                    for block in blocks {
                                                        if block.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
                                                            if let Some(tool_use_id) = block.get("tool_use_id").and_then(|id| id.as_str()) {
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
                                        if let Some(result) = json_value.get("result").and_then(|v| v.as_str()) {
                                            if !result.is_empty() {
                                                tracing::info!("Detected final result message");
                                                let _ = tx_clone.send(ClaudeMessage::Result {
                                                    result: result.to_string()
                                                }).await;
                                            }
                                        }
                                    }
                                    "ask_user" => {
                                        // Handle ask_user interaction events
                                        tracing::info!("Detected ask_user event");
                                        
                                        let prompt_type = json_value.get("prompt_type")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("input")
                                            .to_string();
                                        
                                        let title = json_value.get("title")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        
                                        let options = if let Some(opts_array) = json_value.get("options").and_then(|v| v.as_array()) {
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
                                            if parsed_options.is_empty() { None } else { Some(parsed_options) }
                                        } else {
                                            None
                                        };
                                        
                                        let input_type = json_value.get("input_type")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                        
                                        let placeholder = json_value.get("placeholder")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                        
                                        let tool_use_id = json_value.get("tool_use_id")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                        
                                        let _ = tx_clone.send(ClaudeMessage::AskUser {
                                            prompt_type,
                                            title,
                                            options,
                                            input_type,
                                            placeholder,
                                            tool_use_id,
                                        }).await;
                                    }
                                    "error" => {
                                        // Handle error messages
                                        if let Some(error) = json_value.get("error").and_then(|v| v.as_str()) {
                                            tracing::error!("Detected error message: {}", error);
                                            let _ = tx_clone.send(ClaudeMessage::Error {
                                                error: error.to_string()
                                            }).await;
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
                            let _ = tx_clone.send(ClaudeMessage::Progress {
                                content: json_value
                            }).await;
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
                            tracing::error!("Claude CLI not found at expected path: {:?}", claude_cli_path_clone2);
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
                    use tokio::io::{BufReader, AsyncBufReadExt};
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
            
            match cmd.wait().await {
                Ok(status) if !status.success() => {
                    tracing::error!("Claude CLI exited with non-zero status: {}", status);
                    tracing::error!("Command was: {}", command_debug_clone3);
                    let _ = tx.send(ClaudeMessage::Error {
                        error: format!("Process exited with status: {}", status),
                    }).await;
                }
                Err(e) => {
                    tracing::error!("Error waiting for Claude CLI process: {}", e);
                    tracing::error!("Command was: {}", command_debug_clone3);
                    let _ = tx.send(ClaudeMessage::Error {
                        error: format!("Process error: {}", e),
                    }).await;
                }
                Ok(status) => {
                    tracing::debug!("Claude CLI exited successfully with status: {}", status);
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
}