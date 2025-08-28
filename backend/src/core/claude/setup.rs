use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;
use expectrl::{Regex as ExpectRegex, spawn, Session};
use serde_json::json;
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use tracing::info;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone)]
pub struct ClaudeSetup {
    pub(crate) client_id: Uuid,
    pub(crate) client_dir: PathBuf,
    pub(crate) bun_path: PathBuf,
    pub(crate) session: Arc<Mutex<Option<Session>>>,
    pub(crate) input_ready: Arc<AtomicBool>,
    pub(crate) oauth_token: Arc<Mutex<Option<String>>>,
    pub(crate) pending_token: Arc<Mutex<Option<String>>>,
    pub(crate) oauth_token_notifier: Arc<tokio::sync::Notify>,
}

impl ClaudeSetup {
    pub fn new(client_id: Uuid) -> Self {
        // Use CLIENTS_DIR env var, or default to ../.clients (project root)
        // This avoids triggering backend file watcher when clients are created
        let clients_base = std::env::var("CLIENTS_DIR")
            .unwrap_or_else(|_| "../.clients".to_string());
        
        let clients_base_path = PathBuf::from(&clients_base);
        let client_dir = clients_base_path.join(format!("{}", client_id));
        let bun_path = clients_base_path.join("bun");
        
        Self {
            client_id,
            client_dir,
            bun_path,
            session: Arc::new(Mutex::new(None)),
            input_ready: Arc::new(AtomicBool::new(false)),
            oauth_token: Arc::new(Mutex::new(None)),
            pending_token: Arc::new(Mutex::new(None)),
            oauth_token_notifier: Arc::new(tokio::sync::Notify::new()),
        }
    }

    pub async fn setup_environment(&self, progress_tx: Option<mpsc::Sender<String>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.create_directories().await?;
        
        if let Some(ref tx) = progress_tx {
            let _ = tx.send("Creating client directory structure...".to_string()).await;
        }
        
        self.download_bun(progress_tx.clone()).await?;
        self.install_claude_code(progress_tx.clone()).await?;
        
        if let Some(ref tx) = progress_tx {
            let _ = tx.send("Claude Code environment ready for authentication".to_string()).await;
        }
        
        Ok(())
    }

    async fn create_directories(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::fs::create_dir_all(&self.client_dir)?;
        std::fs::create_dir_all(".clients")?;
        
        let config_dir = self.client_dir.join(".config/claude");
        std::fs::create_dir_all(&config_dir)?;
        
        let config_file = config_dir.join("config.json");
        if !config_file.exists() {
            let config = json!({
                "theme": "dark",
                "hasSeenWelcome": true,
                "outputStyle": "default"
            });
            std::fs::write(&config_file, serde_json::to_string_pretty(&config)?)?;
        }
        
        let package_json = self.client_dir.join("package.json");
        if !package_json.exists() {
            let package_config = json!({
                "name": format!("claude-client-{}", self.client_id),
                "version": "1.0.0",
                "private": true,
                "description": format!("Claude Code environment for client {}", self.client_id),
                "dependencies": {}
            });
            std::fs::write(&package_json, serde_json::to_string_pretty(&package_config)?)?;
        }
        
        Ok(())
    }

    async fn download_bun(&self, progress_tx: Option<mpsc::Sender<String>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bun_executable = self.bun_path.join("bin/bun");
        
        if bun_executable.exists() {
            if let Some(ref tx) = progress_tx {
                let _ = tx.send("Bun already installed, skipping download...".to_string()).await;
            }
            return Ok(());
        }
        
        if let Some(ref tx) = progress_tx {
            let _ = tx.send("Downloading and installing Bun...".to_string()).await;
        }
        
        std::fs::create_dir_all(&self.bun_path)?;
        
        let output = Command::new("bash")
            .arg("-c")
            .arg("curl -fsSL https://bun.sh/install | bash")
            .env_clear()
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
            .env("HOME", ".clients")
            .env("BUN_INSTALL", self.bun_path.to_str().unwrap())
            .output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to install Bun: {}", stderr).into());
        }
        
        if let Some(ref tx) = progress_tx {
            let _ = tx.send("Bun installed successfully!".to_string()).await;
        }
        
        Ok(())
    }

    pub async fn install_claude_code(&self, progress_tx: Option<mpsc::Sender<String>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref tx) = progress_tx {
            let _ = tx.send("Installing @anthropic-ai/claude-code package...".to_string()).await;
        }
        
        // Ensure client directory exists before trying to canonicalize
        std::fs::create_dir_all(&self.client_dir)?;
        
        // Initialize package.json if it doesn't exist
        let package_json = self.client_dir.join("package.json");
        if !package_json.exists() {
            let package_json_content = r#"{"name":"claude-client","version":"1.0.0","type":"module"}"#;
            std::fs::write(&package_json, package_json_content)?;
        }
        
        let bun_executable = self.bun_path.join("bin/bun");
        let bun_path = bun_executable.canonicalize()?;
        let client_dir = self.client_dir.canonicalize()?;
        
        let output = Command::new(&bun_path)
            .args(["add", "@anthropic-ai/claude-code"])
            .current_dir(&client_dir)
            .env("PATH", format!("{}/bin:/usr/bin:/bin:/usr/local/bin", self.bun_path.to_str().unwrap()))
            .env("HOME", std::env::var("HOME").unwrap_or_else(|_| client_dir.to_string_lossy().to_string()))
            .env("BUN_INSTALL", self.bun_path.canonicalize()?.to_str().unwrap())
            .output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(format!("Failed to install Claude Code. stdout: {}, stderr: {}", stdout, stderr).into());
        }
        
        if let Some(ref tx) = progress_tx {
            let _ = tx.send("Claude Code package installed successfully!".to_string()).await;
        }
        
        Ok(())
    }

    pub async fn start_setup_token_stream(&self, progress_tx: mpsc::Sender<String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Use the actual CLI script directly instead of the symlink
        let claude_bin = self.client_dir.join("node_modules/@anthropic-ai/claude-code/cli.js");
        let claude_path = claude_bin.canonicalize()?;
        
        info!("Starting claude setup-token streaming for client {}", self.client_id);
        
        // Kill any existing Claude CLI processes first
        let _ = Command::new("sh")
            .arg("-c")
            .arg("pkill -f 'claude.*setup-token' 2>/dev/null || true")
            .output();
        
        // Check if port 54545 is already in use and clean up if needed
        let port_check = Command::new("sh")
            .arg("-c")
            .arg("lsof -ti:54545")
            .output();
            
        if let Ok(output) = port_check {
            if !output.stdout.is_empty() {
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg("lsof -ti:54545 | xargs kill -9 2>/dev/null || true")
                    .output();
                
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            }
        }
        
        // Set environment to prevent browser opening
        std::env::set_var("BROWSER", "echo");
        
        let _ = progress_tx.send("Starting Claude CLI authentication process...".to_string()).await;
        
        // Run in a blocking task since expectrl is not async
        let claude_path_str = claude_path.to_str().unwrap().to_string();
        let client_id = self.client_id;
        let client_dir = self.client_dir.clone();
        let session_arc = self.session.clone();
        let input_ready_arc = self.input_ready.clone();
        let oauth_token_arc = self.oauth_token.clone();
        let pending_token_arc = self.pending_token.clone();
        let oauth_notifier_arc = self.oauth_token_notifier.clone();
        
        // Define prompt patterns to detect when ready for input
        let prompt_patterns = vec![
            "Paste code here if prompted",
            "Paste code here",
            "Enter setup token", 
            "Setup token:",
            "Token:",
            "Paste the setup token",
            ">"
        ];
        
        tokio::task::spawn_blocking(move || {
            let session = spawn(format!("{} setup-token", claude_path_str).as_str())?;
            
            // Store session for later use in token submission
            {
                let mut session_guard = session_arc.blocking_lock();
                *session_guard = Some(session);
            }
            
            let timeout = std::time::Duration::from_secs(300); // 5 minute timeout
            let start = std::time::Instant::now();
            
            // Stream all output until the process exits or times out
            while start.elapsed() < timeout {
                // Check for pending token to send first
                {
                    let mut pending_guard = pending_token_arc.blocking_lock();
                    if let Some(token) = pending_guard.take() {
                        drop(pending_guard);
                        info!("Found pending token to send for client {}", client_id);
                        
                        let mut session_guard = session_arc.blocking_lock();
                        if let Some(ref mut session) = *session_guard {
                            // Send token first
                            match session.send(&token) {
                                Ok(_) => {
                                    info!("Token text sent to Claude CLI for client {}", client_id);
                                    
                                    // Small delay before sending Enter
                                    std::thread::sleep(std::time::Duration::from_millis(100));
                                    
                                    // Then send Enter to submit
                                    info!("Attempting to send Enter to submit token for client {}", client_id);
                                    match session.send("\r") {
                                        Ok(_) => {
                                            info!("✅ Successfully submitted token to Claude CLI for client {}", client_id);
                                        }
                                        Err(e) => {
                                            info!("❌ Failed to submit token (Enter) to Claude CLI for client {}: {}", client_id, e);
                                            
                                            // Try alternative line ending
                                            info!("Trying \\n instead for client {}", client_id);
                                            match session.send("\n") {
                                                Ok(_) => {
                                                    info!("✅ Successfully submitted token with \\n for client {}", client_id);
                                                }
                                                Err(e2) => {
                                                    info!("❌ Failed with \\n too for client {}: {}", client_id, e2);
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    info!("❌ Failed to send token text to Claude CLI for client {}: {}", client_id, e);
                                }
                            }
                        }
                        drop(session_guard);
                    }
                }
                
                let mut session_guard = session_arc.blocking_lock();
                if let Some(ref mut _session) = *session_guard {
                    drop(session_guard); // Release lock before expect call
                    
                    let mut session_guard = session_arc.blocking_lock();
                    match session_guard.as_mut().unwrap().expect(ExpectRegex(".+")) {
                        Ok(output) => {
                            let output_str = String::from_utf8_lossy(output.as_bytes()).to_string();
                            
                            // Log the output with more detail for debugging
                            info!("Claude CLI output for {} (length: {}): {}", client_id, output_str.len(), output_str);
                            
                            // Also log if we see specific patterns
                            if output_str.contains("OAuth") || output_str.contains("oauth") {
                                info!("🔍 Output contains OAuth reference");
                            }
                            if output_str.contains("token") || output_str.contains("Token") {
                                info!("🔍 Output contains token reference");
                            }
                            
                            // Always check for OAuth token, not just when specific conditions are met
                            if output_str.contains("sk-ant-") {
                                info!("🔍 Detected sk-ant- token in output!");
                                
                                // Try multiple extraction patterns
                                let mut found_token: Option<String> = None;
                                
                                // Pattern 1: Token on its own line
                                for line in output_str.lines() {
                                    let line = line.trim();
                                    if line.starts_with("sk-ant-") {
                                        found_token = Some(line.to_string());
                                        info!("Found token on separate line: {}", line);
                                        break;
                                    }
                                }
                                
                                // Pattern 2: Extract token using regex from anywhere in the text
                                if found_token.is_none() {
                                    // Look for sk-ant- followed by alphanumeric characters and dashes
                                    if let Some(start_idx) = output_str.find("sk-ant-") {
                                        let token_str = &output_str[start_idx..];
                                        // Extract until we hit whitespace or non-token character
                                        let mut end_idx = 0;
                                        for (i, ch) in token_str.chars().enumerate() {
                                            if ch.is_alphanumeric() || ch == '-' || ch == '_' {
                                                end_idx = i + 1;
                                            } else {
                                                break;
                                            }
                                        }
                                        
                                        if end_idx > 7 { // "sk-ant-" is 7 chars, we need more
                                            let extracted = &token_str[..end_idx];
                                            found_token = Some(extracted.to_string());
                                            info!("Extracted token from text: {}", extracted);
                                        }
                                    }
                                }
                                
                                if let Some(token) = found_token {
                                    info!("✅ Storing OAuth token (length {}): {}", token.len(), token);
                                    let mut oauth_guard = oauth_token_arc.blocking_lock();
                                    *oauth_guard = Some(token.clone());
                                    info!("✅ OAuth token stored successfully");
                                    
                                    // Save the token to .env file immediately
                                    let env_file = client_dir.join(".env");
                                    let env_content = format!(
                                        "# Claude Code Environment Configuration\nCLAUDE_CODE_OAUTH_TOKEN={}\n",
                                        token
                                    );
                                    
                                    if let Err(e) = std::fs::write(&env_file, env_content) {
                                        info!("Failed to write .env file: {}", e);
                                    } else {
                                        info!("Created .env file for client {} with CLAUDE_CODE_OAUTH_TOKEN", client_id);
                                    }
                                    
                                    // Notify that OAuth token is ready
                                    oauth_notifier_arc.notify_one();
                                    info!("🔔 Notified waiting tasks that OAuth token is ready");
                                } else {
                                    info!("⚠️ Could not extract OAuth token from output containing sk-ant-");
                                }
                            }
                            
                            // Check for prompt patterns that indicate ready for token input
                            for pattern in &prompt_patterns {
                                if output_str.contains(pattern) {
                                    info!("Found prompt pattern '{}' - setting input_ready to true", pattern);
                                    input_ready_arc.store(true, Ordering::SeqCst);
                                    
                                    // Send special message to frontend indicating ready for input
                                    let tx_clone = progress_tx.clone();
                                    tokio::spawn(async move {
                                        let _ = tx_clone.send("INPUT_READY".to_string()).await;
                                    });
                                    
                                    // Check if token was already provided on the same line
                                    if let Some(prompt_pos) = output_str.find(pattern) {
                                        let after_prompt = &output_str[prompt_pos + pattern.len()..];
                                        if after_prompt.trim().len() > 20 { // Token should be reasonably long
                                            info!("Token appears to already be provided on same line, continuing...");
                                            // Don't break here, continue processing to capture the response
                                        }
                                    }
                                    break;
                                }
                            }
                            
                            // OAuth token detection is now done earlier, right after logging
                            
                            // Check for authentication success indicators
                            if output_str.contains("Authentication successful") || 
                               output_str.contains("Login successful") ||
                               output_str.contains("Setup complete") ||
                               output_str.contains("✓") {
                                info!("Authentication completed for client {}", client_id);
                                
                                // Double-check for OAuth token one more time in case it appeared
                                if oauth_token_arc.blocking_lock().is_none() && output_str.contains("sk-ant-") {
                                    info!("Re-checking for OAuth token after authentication success");
                                    // Re-run the extraction logic
                                    if let Some(start_idx) = output_str.find("sk-ant-") {
                                        let token_str = &output_str[start_idx..];
                                        let mut end_idx = 0;
                                        for (i, ch) in token_str.chars().enumerate() {
                                            if ch.is_alphanumeric() || ch == '-' || ch == '_' {
                                                end_idx = i + 1;
                                            } else {
                                                break;
                                            }
                                        }
                                        
                                        if end_idx > 7 {
                                            let extracted = &token_str[..end_idx];
                                            info!("✅ Late extraction of OAuth token: {}", extracted);
                                            let mut oauth_guard = oauth_token_arc.blocking_lock();
                                            *oauth_guard = Some(extracted.to_string());
                                            oauth_notifier_arc.notify_one();
                                        }
                                    }
                                }
                            }
                            
                            // Handle interactive prompts automatically
                            if output_str.contains("Press Enter to retry") || output_str.contains("Port 54545 is already in use") {
                                info!("Detected port conflict prompt, sending Enter to retry");
                                let _ = session_guard.as_mut().unwrap().send_line("");
                                drop(session_guard);
                                continue;
                            }
                            drop(session_guard);
                            
                            // Stream output to frontend
                            let tx_clone = progress_tx.clone();
                            let output_str_clone = output_str.clone();
                            tokio::spawn(async move {
                                let _ = tx_clone.send(output_str_clone).await;
                            });
                            
                            // Check if the process has completed (look for success indicators or exit)
                            if output_str.contains("Login successful") || 
                               output_str.contains("Authentication successful") ||
                               output_str.contains("✓") ||
                               output_str.contains("Setup complete") {
                                // Wait a bit more to capture any final output
                                std::thread::sleep(std::time::Duration::from_millis(2000));
                                break;
                            }
                        },
                        Err(_) => {
                            drop(session_guard);
                            // Check if process has exited
                            let mut session_guard = session_arc.blocking_lock();
                            if let Some(ref mut session) = *session_guard {
                                if !session.is_alive().unwrap_or(true) {
                                    info!("Claude CLI process has exited for client {}", client_id);
                                    break;
                                }
                            }
                            drop(session_guard);
                            // No match or timeout, continue
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                    }
                } else {
                    break;
                }
            }
            
            // Send completion signal
            let tx_clone = progress_tx.clone();
            tokio::spawn(async move {
                let _ = tx_clone.send("STREAM_COMPLETE".to_string()).await;
            });
            
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        }).await??;
        
        Ok(())
    }

    pub async fn submit_setup_token(&self, setup_token: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client_dir_path = self.client_dir.canonicalize()?;
        
        info!("Submitting setup token for client {}", self.client_id);
        
        // Wait for input_ready to be true before proceeding
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(30); // 30 second timeout
        
        while !self.input_ready.load(Ordering::SeqCst) {
            if start_time.elapsed() > timeout {
                return Err("Timeout waiting for input ready signal".into());
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        info!("Input ready detected, submitting token...");
        
        // Check if we already have a token (might have been captured from the same line)
        if let Some(existing_token) = self.get_oauth_token().await {
            info!("Token already captured from streaming output: {}", existing_token);
            self.save_env_file(&existing_token).await?;
            return Ok(existing_token);
        }
        
        // Queue the real token to be sent by the streaming task
        {
            let mut pending_guard = self.pending_token.lock().await;
            *pending_guard = Some(setup_token.to_string());
            info!("Queued token for sending by streaming task for client {}", self.client_id);
        }
        
        // Wait for OAuth token to be detected by streaming task
        let mut final_oauth_token = String::new();
        
        info!("Waiting for OAuth token notification...");
        
        // Wait for notification with timeout
        let timeout = tokio::time::Duration::from_secs(20);
        let notifier = self.oauth_token_notifier.clone();
        
        match tokio::time::timeout(timeout, notifier.notified()).await {
            Ok(_) => {
                // Notification received, get the token
                if let Some(token) = self.get_oauth_token().await {
                    final_oauth_token = token.clone();
                    info!("✅ Retrieved OAuth token after notification: {}", token);
                } else {
                    info!("⚠️ Notification received but no token found");
                }
            }
            Err(_) => {
                info!("⏰ Timeout waiting for OAuth token (20 seconds)");
                // One last check in case we missed the notification
                if let Some(token) = self.get_oauth_token().await {
                    final_oauth_token = token.clone();
                    info!("✅ Found OAuth token after timeout: {}", token);
                }
            }
        }
        
        // Check credential files for the actual OAuth token in multiple locations
        let home_dir = std::env::var("HOME").unwrap_or_default();
        let auth_files = vec![
            client_dir_path.join(".config/claude/auth.json"),
            client_dir_path.join(".config/claude/credentials.json"), 
            client_dir_path.join(".claude/auth.json"),
            client_dir_path.join(".claude/credentials.json"),
            // Also check home directory where Claude CLI might store global credentials
            PathBuf::from(&home_dir).join(".config/claude/auth.json"),
            PathBuf::from(&home_dir).join(".config/claude/credentials.json"),
            PathBuf::from(&home_dir).join(".claude/auth.json"),
            PathBuf::from(&home_dir).join(".claude/credentials.json"),
        ];
        
        // If we still don't have a token, check credential files
        if final_oauth_token.is_empty() {
            info!("No token from streaming output, checking credential files...");
            for auth_file in &auth_files {
                info!("Checking file: {:?}", auth_file);
                if auth_file.exists() {
                    if let Ok(content) = std::fs::read_to_string(auth_file) {
                        info!("File content: {}", content);
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(token) = json.get("oauth_token").and_then(|v| v.as_str()) {
                                final_oauth_token = token.to_string();
                                info!("Found oauth_token: {}", final_oauth_token);
                                break;
                            }
                            if let Some(token) = json.get("token").and_then(|v| v.as_str()) {
                                final_oauth_token = token.to_string();
                                info!("Found token: {}", final_oauth_token);
                                break;
                            }
                            if let Some(access_token) = json.get("access_token").and_then(|v| v.as_str()) {
                                final_oauth_token = access_token.to_string();
                                info!("Found access_token: {}", final_oauth_token);
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        if final_oauth_token.is_empty() {
            final_oauth_token = format!("authenticated_client_{}", self.client_id);
            info!("Could not extract OAUTH_TOKEN from streaming or files, using placeholder token: {}", final_oauth_token);
        } else {
            info!("Final OAuth token to be saved: {}", final_oauth_token);
        }
        
        self.save_env_file(&final_oauth_token).await?;
        
        Ok(final_oauth_token)
    }

    #[allow(dead_code)]
    pub fn is_input_ready(&self) -> bool {
        self.input_ready.load(Ordering::SeqCst)
    }
    
    
    pub async fn get_oauth_token(&self) -> Option<String> {
        let guard = self.oauth_token.lock().await;
        guard.clone()
    }
    
    async fn save_env_file(&self, oauth_token: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let env_file = self.client_dir.join(".env");
        let env_content = format!(
            "# Claude Code Environment Configuration\nCLAUDE_CODE_OAUTH_TOKEN={}\n",
            oauth_token
        );
        
        std::fs::write(&env_file, env_content)?;
        info!("Created .env file for client {} with CLAUDE_CODE_OAUTH_TOKEN", self.client_id);
        
        Ok(())
    }
}
