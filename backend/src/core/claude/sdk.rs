use std::path::PathBuf;
use uuid::Uuid;
use serde_json::json;
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use tracing::info;
use tokio::process::Command as TokioCommand;

use super::types::{QueryRequest, ClaudeMessage};

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
            
            // Prepare MCP server path
            let mcp_server_path = {
                let release_path = std::env::current_dir()
                    .map(|p| p.join("target/release/mcp_server"))
                    .unwrap_or_else(|_| PathBuf::from("target/release/mcp_server"));
                let debug_path = std::env::current_dir()
                    .map(|p| p.join("target/debug/mcp_server"))
                    .unwrap_or_else(|_| PathBuf::from("target/debug/mcp_server"));
                
                if debug_path.exists() {
                    debug_path.canonicalize().unwrap_or(debug_path)
                } else if release_path.exists() {
                    release_path.canonicalize().unwrap_or(release_path)
                } else {
                    PathBuf::from("/usr/local/bin/mcp_server")
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
                    }
                }
            });
            
            // Write the MCP servers configuration
            let _ = std::fs::write(&mcp_servers_file, serde_json::to_string_pretty(&mcp_servers).unwrap_or_default());
            info!("Created MCP servers configuration for project {} at {:?}", project_id, mcp_servers_file);
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
                content: "Setting up client environment - Installing Claude CLI package...".to_string() 
            }).await;
            
            use crate::core::claude::setup::ClaudeSetup;
            
            let setup = ClaudeSetup::new(self.client_id);
            setup.install_claude_code(None).await?;
            
            let _ = tx.send(ClaudeMessage::Progress { 
                content: "Client environment setup complete!".to_string() 
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
        
        // Ensure all requirements are met before proceeding
        self.ensure_requirements_met(&tx).await?;
        
        let oauth_token = self.oauth_token.lock().await.clone()
            .ok_or("No OAuth token available")?;
        
        let working_dir = self.project_dir.as_ref().unwrap_or(&self.client_dir);
        
        let working_dir_clone = working_dir.clone();
        let client_id = self.client_id;
        let prompt = request.prompt.clone();
        let _project_dir = self.project_dir.clone();
        
        let bun_executable = self.bun_path.join("bin/bun");
        
        tokio::spawn(async move {
            let mut cmd_builder = TokioCommand::new(&bun_executable);
            cmd_builder
                .arg("claude");
            
            // Add MCP config if we have a project directory - must come before --print
            if let Some(ref project_dir) = _project_dir {
                let mcp_config_path = project_dir.join(".claude/mcp_servers.json");
                if mcp_config_path.exists() {
                    cmd_builder
                        .arg("--mcp-config")
                        .arg(".claude/mcp_servers.json");
                }
            }
            
            cmd_builder
                .arg("-p")
                .arg("--verbose")
                .arg("--dangerously-skip-permissions")
                .arg("--output-format")
                .arg("stream-json");
            
            let cmd = cmd_builder
                .arg(&prompt)
                .current_dir(&working_dir_clone)
                .env("HOME", &working_dir_clone)
                .env("CLAUDE_CODE_OAUTH_TOKEN", oauth_token)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            
            tracing::debug!("Executing Claude CLI command: {:?}", cmd);
            
            let mut cmd = cmd
                .spawn()
                .expect("Failed to spawn Claude CLI process");
            
            // Also capture stderr for debugging
            if let Some(stderr) = cmd.stderr.take() {
                let _tx_stderr = tx.clone();
                tokio::spawn(async move {
                    use tokio::io::{BufReader, AsyncBufReadExt};
                    let reader = BufReader::new(stderr);
                    let mut lines = reader.lines();
                    
                    while let Ok(Some(line)) = lines.next_line().await {
                        // Log stderr for debugging
                        tracing::debug!("[CLAUDE_STDERR] {}", line);
                    }
                });
            }
            
            if let Some(stdout) = cmd.stdout.take() {
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    use tokio::io::{BufReader, AsyncBufReadExt};
                    let reader = BufReader::new(stdout);
                    let mut lines = reader.lines();
                    
                    let mut line_count = 0;
                    while let Ok(Some(line)) = lines.next_line().await {
                        line_count += 1;
                        tracing::info!("[CLAUDE_STDOUT] Line {}: {}", line_count, line);
                        
                        if !line.trim().is_empty() {
                            // Always send raw line as Progress for debugging
                            let _ = tx_clone.send(ClaudeMessage::Progress {
                                content: line.clone()
                            }).await;
                            
                            // Try to parse the line as JSON to determine message type
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                                // Log ALL message types to understand what Claude sends
                                if let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) {
                                    tracing::info!("[CLAUDE_MSG] Received message type '{}'", msg_type);
                                    
                                    // Handle tool_result messages
                                    if msg_type == "tool_result" {
                                        if let (Some(tool_name), Some(result)) = 
                                            (json.get("tool_name").and_then(|v| v.as_str()),
                                             json.get("result")) {
                                            tracing::info!("Detected tool result for {}: {:?}", tool_name, result);
                                            let _ = tx_clone.send(ClaudeMessage::ToolResult {
                                                tool: tool_name.to_string(),
                                                result: result.clone(),
                                            }).await;
                                        }
                                    } else if msg_type == "assistant" {
                                        // This is the assistant message with content
                                        if let Some(message) = json.get("message") {
                                            if let Some(content) = message.get("content") {
                                                // Check for tool use blocks in the content array
                                                if content.is_array() {
                                                    if let Some(blocks) = content.as_array() {
                                                        for block in blocks {
                                                            // Check for tool_use blocks
                                                            if let Some(block_type) = block.get("type").and_then(|t| t.as_str()) {
                                                                if block_type == "tool_use" {
                                                                    // Extract tool name, tool_use_id and send ToolUse event
                                                                    if let Some(name) = block.get("name").and_then(|n| n.as_str()) {
                                                                        let tool_use_id = block.get("id").and_then(|id| id.as_str()).map(|s| s.to_string());
                                                                        tracing::info!("Detected tool usage: {} with id: {:?}", name, tool_use_id);
                                                                        let args = block.get("input").cloned().unwrap_or(json!({}));
                                                                        tracing::info!("Sending ToolUse event: {}", name);
                                                                        let _ = tx_clone.send(ClaudeMessage::ToolUse {
                                                                            tool: name.to_string(),
                                                                            args,
                                                                            tool_use_id,
                                                                        }).await;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        
                                                        // Extract text from content blocks
                                                        let text = blocks.iter()
                                                            .filter_map(|block| {
                                                                if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                                                                    block.get("text").and_then(|t| t.as_str())
                                                                } else {
                                                                    None
                                                                }
                                                            })
                                                            .collect::<Vec<_>>()
                                                            .join("");
                                                        
                                                        if !text.is_empty() {
                                                            let _ = tx_clone.send(ClaudeMessage::Result {
                                                                result: text
                                                            }).await;
                                                        }
                                                    }
                                                } else if content.is_string() {
                                                    let text = content.as_str().unwrap_or("").to_string();
                                                    if !text.is_empty() {
                                                        let _ = tx_clone.send(ClaudeMessage::Result {
                                                            result: text
                                                        }).await;
                                                    }
                                                }
                                            }
                                        }
                                    } else if msg_type == "user" {
                                        // Handle user messages that contain tool_result content
                                        if let Some(message) = json.get("message") {
                                            if let Some(content) = message.get("content") {
                                                if content.is_array() {
                                                    if let Some(blocks) = content.as_array() {
                                                        for block in blocks {
                                                            if let Some(block_type) = block.get("type").and_then(|t| t.as_str()) {
                                                                if block_type == "tool_result" {
                                                                    // Extract tool_use_id and content
                                                                    if let Some(tool_use_id) = block.get("tool_use_id").and_then(|id| id.as_str()) {
                                                                        // Try to extract the tool name from the ID or content
                                                                        let tool_name = if let Some(content_array) = block.get("content").and_then(|c| c.as_array()) {
                                                                            // Extract tool name from the content if possible
                                                                            if let Some(first_content) = content_array.first() {
                                                                                if let Some(text) = first_content.get("text").and_then(|t| t.as_str()) {
                                                                                    // Try to infer tool name from the content
                                                                                    if text.contains("Query executed on") {
                                                                                        "mcp__data-analysis__data_query"
                                                                                    } else if text.contains("Data sources") {
                                                                                        "mcp__data-analysis__datasource_list"
                                                                                    } else if text.contains("Database statistics") {
                                                                                        "mcp__data-analysis__schema_stats"
                                                                                    } else if text.contains("Database compatibility") || text.contains("MCP Server Error") {
                                                                                        "mcp__data-analysis__datasource_inspect"
                                                                                    } else if text.contains("Connection successful") {
                                                                                        "mcp__data-analysis__datasource_test"
                                                                                    } else if text.contains("Data source") && text.contains("added successfully") {
                                                                                        "mcp__data-analysis__datasource_add"
                                                                                    } else if text.contains("Tables matching") || text.contains("Schema for table") {
                                                                                        "mcp__data-analysis__schema_search"
                                                                                    } else if text.contains("columns matching") {
                                                                                        "mcp__data-analysis__schema_get"
                                                                                    } else {
                                                                                        // Use tool_use_id as fallback
                                                                                        tool_use_id
                                                                                    }
                                                                                } else {
                                                                                    tool_use_id
                                                                                }
                                                                            } else {
                                                                                tool_use_id
                                                                            }
                                                                        } else {
                                                                            tool_use_id
                                                                        };
                                                                        
                                                                        tracing::info!("Detected tool result in user message with tool_use_id: {} -> mapped to tool: {}", tool_use_id, tool_name);
                                                                        
                                                                        // Send both the tool_use_id and the inferred tool name
                                                                        let _ = tx_clone.send(ClaudeMessage::ToolResult {
                                                                            tool: tool_use_id.to_string(), // Use the tool_use_id as the tool identifier
                                                                            result: block.get("content").cloned().unwrap_or(json!([])),
                                                                        }).await;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else if msg_type == "result" {
                                        // Also handle the explicit result message
                                        if let Some(result) = json.get("result").and_then(|v| v.as_str()) {
                                            if !result.is_empty() {
                                                let _ = tx_clone.send(ClaudeMessage::Result {
                                                    result: result.to_string()
                                                }).await;
                                            }
                                        }
                                    } else if msg_type == "tool_call" || msg_type == "tool_execution" {
                                        // Handle standalone tool execution messages
                                        if let Some(tool_name) = json.get("tool").and_then(|t| t.as_str()) {
                                            let tool_use_id = json.get("id").and_then(|id| id.as_str()).map(|s| s.to_string());
                                            tracing::info!("Detected standalone tool call: {} with id: {:?}", tool_name, tool_use_id);
                                            let args = json.get("arguments").cloned().unwrap_or(json!({}));
                                            tracing::info!("Sending standalone ToolUse event: {}", tool_name);
                                            let _ = tx_clone.send(ClaudeMessage::ToolUse {
                                                tool: tool_name.to_string(),
                                                args,
                                                tool_use_id,
                                            }).await;
                                        }
                                    }
                                }
                                
                                // Check for JSON-RPC result patterns (MCP server responses)
                                if let Some(jsonrpc) = json.get("jsonrpc") {
                                    if jsonrpc.as_str() == Some("2.0") {
                                        if let Some(result) = json.get("result") {
                                            if let Some(content) = result.get("content") {
                                                if content.is_array() {
                                                    // This is definitely an MCP tool result
                                                    // Infer tool name from content pattern
                                                    let tool_name = if let Some(first_content) = content.as_array()
                                                        .and_then(|arr| arr.first()) 
                                                        .and_then(|item| item.get("text"))
                                                        .and_then(|text| text.as_str()) {
                                                        
                                                        if first_content.contains("Data sources") {
                                                            "mcp__data-analysis__datasource_list"
                                                        } else if first_content.contains("Database statistics") {
                                                            "mcp__data-analysis__schema_stats"
                                                        } else if first_content.contains("Database compatibility") || first_content.contains("MCP Server Error") {
                                                            "mcp__data-analysis__datasource_inspect"
                                                        } else {
                                                            "mcp_tool"
                                                        }
                                                    } else {
                                                        "mcp_tool"
                                                    };
                                                    
                                                    tracing::info!("Detected MCP JSON-RPC tool result for: {}", tool_name);
                                                    let _ = tx_clone.send(ClaudeMessage::ToolResult {
                                                        tool: tool_name.to_string(),
                                                        result: result.clone(),
                                                    }).await;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // Always send as progress for frontend streaming
                            let _ = tx_clone.send(ClaudeMessage::Progress {
                                content: line
                            }).await;
                        }
                    }
                    
                    tracing::info!("[CLAUDE_STDOUT] Stream ended after {} lines", line_count);
                });
            }
            
            if let Some(stderr) = cmd.stderr.take() {
                tokio::spawn(async move {
                    use tokio::io::{BufReader, AsyncBufReadExt};
                    let reader = BufReader::new(stderr);
                    let mut lines = reader.lines();
                    
                    while let Ok(Some(line)) = lines.next_line().await {
                        tracing::debug!("Claude SDK stderr: {}", line);
                    }
                });
            }
            
            match cmd.wait().await {
                Ok(status) if !status.success() => {
                    let _ = tx.send(ClaudeMessage::Error {
                        error: format!("Process exited with status: {}", status),
                    }).await;
                }
                Err(e) => {
                    let _ = tx.send(ClaudeMessage::Error {
                        error: format!("Process error: {}", e),
                    }).await;
                }
                _ => {}
            }
            
            tracing::debug!("Query completed for client {}", client_id);
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