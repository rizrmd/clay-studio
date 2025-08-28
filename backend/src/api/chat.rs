use salvo::prelude::*;
use salvo::sse::{self, SseEvent};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::models::{Message, MessageRole, ToolUsage};
use crate::utils::AppState;
use crate::utils::AppError;
use crate::core::claude::{ClaudeManager, QueryOptions, ClaudeMessage};
use crate::core::projects::ProjectManager;
use chrono::Utc;
use uuid::Uuid;
use sqlx::{Row, PgPool};

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
    pub project_id: String,
    pub conversation_id: Option<String>,
}

// Helper function to ensure conversation exists
async fn ensure_conversation(
    pool: &PgPool,
    conversation_id: &str,
    project_id: &str,
    initial_message: Option<&str>,
) -> Result<bool, AppError> {
    // Check if conversation exists
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM conversations WHERE id = $1)"
    )
    .bind(conversation_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    if !exists {
        // Generate title from initial message if provided, otherwise use a descriptive placeholder
        let title = initial_message
            .map(generate_title_from_message)
            .unwrap_or_else(|| "Untitled conversation".to_string());
        
        // Create conversation if it doesn't exist
        sqlx::query(
            "INSERT INTO conversations (id, project_id, title, message_count, created_at, updated_at, is_title_manually_set) 
             VALUES ($1, $2, $3, 0, $4, $4, false)"
        )
        .bind(conversation_id)
        .bind(project_id)
        .bind(&title)
        .bind(Utc::now())
        .execute(pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to create conversation: {}", e)))?;
        
        tracing::info!("Created new conversation {} with title: {}", conversation_id, title);
        return Ok(true); // Newly created
    }

    Ok(false) // Already existed
}

// Helper function to generate conversation title from message
fn generate_title_from_message(content: &str) -> String {
    // Take first 100 characters or first sentence, whichever is shorter
    let truncated = if content.len() > 100 {
        // Find first sentence ending
        let sentence_end = content[..100]
            .find(['.', '?', '!'])
            .map(|i| i + 1)
            .unwrap_or(100);
        
        let end_pos = sentence_end.min(100);
        let mut title = content[..end_pos].to_string();
        
        // Add ellipsis if truncated
        if end_pos == 100 && !title.ends_with('.') && !title.ends_with('?') && !title.ends_with('!') {
            title.push_str("...");
        }
        
        title
    } else {
        content.to_string()
    };
    
    // Clean up whitespace and ensure it's not empty
    let cleaned = truncated.trim().to_string();
    if cleaned.is_empty() {
        "Untitled conversation".to_string()
    } else {
        cleaned
    }
}

// Helper function to detect if topic has changed significantly
fn has_topic_changed(current_title: &str, new_message: &str, message_count: i32) -> bool {
    // Only consider topic change after at least 5 messages
    if message_count < 5 {
        return false;
    }
    
    // Check if new message is substantially different from title topic
    // Simple heuristic: check for question words or topic-changing phrases
    let topic_change_indicators = [
        "let's talk about",
        "change topic",
        "different question",
        "another topic",
        "switching gears",
        "moving on to",
        "now about",
        "new question",
        "unrelated",
    ];
    
    let lower_message = new_message.to_lowercase();
    let lower_title = current_title.to_lowercase();
    
    // Check for explicit topic change indicators
    for indicator in &topic_change_indicators {
        if lower_message.contains(indicator) {
            return true;
        }
    }
    
    // Check if the message is a completely different question (starts with question word)
    let question_words = ["what", "how", "why", "when", "where", "who", "which", "can you", "could you", "would you"];
    let starts_with_question = question_words.iter().any(|&word| lower_message.starts_with(word));
    
    // If it's a new question and doesn't contain key words from the title, likely a topic change
    if starts_with_question {
        // Extract key words from title (words longer than 3 chars)
        let title_keywords: Vec<String> = lower_title
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .collect();
        
        // Check if any title keywords appear in the new message
        let has_common_keywords = title_keywords.iter()
            .any(|keyword| lower_message.contains(keyword));
        
        return !has_common_keywords;
    }
    
    false
}

// Helper function to save message to database
async fn save_message(
    pool: &PgPool,
    conversation_id: &str,
    message: &Message,
) -> Result<(), AppError> {
    // Use INSERT ON CONFLICT to handle both new messages and updates to assistant message stubs
    sqlx::query(
        "INSERT INTO messages (id, conversation_id, content, role, clay_tools_used, processing_time_ms, created_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (id) DO UPDATE SET
            content = EXCLUDED.content,
            clay_tools_used = EXCLUDED.clay_tools_used,
            processing_time_ms = EXCLUDED.processing_time_ms"
    )
    .bind(&message.id)
    .bind(conversation_id)
    .bind(&message.content)
    .bind(format!("{:?}", message.role).to_lowercase())
    .bind(message.clay_tools_used.as_ref().map(|tools| serde_json::to_value(tools).unwrap_or(serde_json::Value::Null)))
    .bind(message.processing_time_ms)
    .bind(Utc::now())
    .execute(pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to save message: {}", e)))?;
    
    // Save tool usages if any
    if let Some(tool_usages) = &message.tool_usages {
        tracing::info!("Saving {} tool usages for message {}", tool_usages.len(), message.id);
        for tool_usage in tool_usages {
            tracing::info!("Saving tool usage: {} with params: {:?}", tool_usage.tool_name, tool_usage.parameters);
            
            // Use INSERT ON CONFLICT to update if already exists (was saved immediately)
            sqlx::query(
                "INSERT INTO tool_usages (id, message_id, tool_name, parameters, output, execution_time_ms, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (id) DO UPDATE SET
                    output = EXCLUDED.output,
                    execution_time_ms = EXCLUDED.execution_time_ms"
            )
            .bind(tool_usage.id)
            .bind(&message.id)  // message_id is varchar, keep as string
            .bind(&tool_usage.tool_name)
            .bind(&tool_usage.parameters)
            .bind(&tool_usage.output)
            .bind(tool_usage.execution_time_ms)
            .bind(Utc::now())
            .execute(pool)
            .await
            .map_err(|e| AppError::InternalServerError(format!("Failed to save tool usage: {}", e)))?;
        }
    } else {
        tracing::debug!("No tool usages to save for message {}", message.id);
    }

    // Update conversation message count and updated_at
    sqlx::query(
        "UPDATE conversations 
         SET message_count = message_count + 1, updated_at = $1 
         WHERE id = $2"
    )
    .bind(Utc::now())
    .bind(conversation_id)
    .execute(pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to update conversation: {}", e)))?;

    Ok(())
}


#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum StreamMessage {
    #[serde(rename = "start")]
    Start { id: String, conversation_id: String },
    #[serde(rename = "progress")]
    Progress { content: String },
    #[serde(rename = "tool_use")]
    ToolUse { tool: String },
    #[serde(rename = "tool_result")]
    ToolResult { tool: String, result: serde_json::Value, tool_usage_id: String },
    #[serde(rename = "content")]
    Content { content: String },
    #[serde(rename = "complete")]
    Complete { 
        id: String,
        conversation_id: String,
        processing_time_ms: i64,
        tools_used: Vec<String>,
    },
    #[serde(rename = "error")]
    Error { error: String },
}

#[handler]
pub async fn handle_chat_stream(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let chat_request: ChatRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;

    // Validate that messages are provided
    if chat_request.messages.is_empty() {
        return Err(AppError::BadRequest("No messages provided".to_string()));
    }

    // Determine conversation ID
    // Treat "new" as if no conversation_id was provided
    let conversation_id = match chat_request.conversation_id.as_deref() {
        Some("new") | None => {
            format!("conv-{}-{}", 
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis(),
                Uuid::new_v4().to_string().split('-').next().unwrap()
            )
        },
        Some(id) => id.to_string(),
    };

    // Get the first user message content for title generation
    let first_user_message = chat_request.messages.iter()
        .find(|m| m.role == MessageRole::User)
        .map(|m| m.content.as_str());
    
    // Ensure conversation exists and check if it's new
    let is_new_conversation = ensure_conversation(
        &state.db_pool, 
        &conversation_id, 
        &chat_request.project_id,
        first_user_message
    ).await?;

    // Get the first active client from the database
    let client_row = sqlx::query(
        "SELECT id, claude_token, install_path FROM clients WHERE status = 'active' AND claude_token IS NOT NULL LIMIT 1"
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let (client_id, claude_token) = if let Some(row) = client_row {
        let id: Uuid = row.get("id");
        let token: Option<String> = row.get("claude_token");
        (id, token)
    } else {
        // No active client found, return a helpful error message
        return Err(AppError::ServiceUnavailable(
            "No active Claude client available. Please set up a client first.".to_string()
        ));
    };

    tracing::info!(
        "Using client {} for streaming chat request with project {} (conversation: {:?})", 
        client_id, 
        chat_request.project_id,
        chat_request.conversation_id
    );

    // Ensure project directory exists
    let project_manager = ProjectManager::new();
    project_manager.ensure_project_directory(client_id, &chat_request.project_id)?;

    // Check if there are forgotten messages and handle them
    let forgotten_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM messages WHERE conversation_id = $1 AND is_forgotten = true"
    )
    .bind(&conversation_id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(0);

    if forgotten_count > 0 {
        // Delete forgotten messages permanently when sending a new message
        sqlx::query(
            "DELETE FROM messages 
             WHERE conversation_id = $1 AND is_forgotten = true"
        )
        .bind(&conversation_id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to delete forgotten messages: {}", e)))?;

        tracing::info!("Deleted {} forgotten messages for conversation {}", forgotten_count, conversation_id);
    }

    // Load existing (non-forgotten) messages from database to build full conversation context
    let existing_messages = sqlx::query(
        "SELECT content, role FROM messages 
         WHERE conversation_id = $1 
         ORDER BY created_at ASC"
    )
    .bind(&conversation_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to load messages: {}", e)))?;
    
    // Build the conversation context from all messages (existing + new)
    let mut conversation = String::new();
    
    // First add all existing messages from database
    for row in existing_messages {
        let role: String = row.get("role");
        let content: String = row.get("content");
        match role.as_str() {
            "user" => conversation.push_str(&format!("User: {}\n", content)),
            "assistant" => conversation.push_str(&format!("Assistant: {}\n", content)),
            "system" => conversation.push_str(&format!("System: {}\n", content)),
            _ => {}
        }
    }
    
    // Save the new user message to database before adding to conversation
    if let Some(last_msg) = chat_request.messages.last() {
        if last_msg.role == MessageRole::User {
            let user_msg_with_id = Message {
                id: Uuid::new_v4().to_string(),
                content: last_msg.content.clone(),
                role: last_msg.role.clone(),
                created_at: Some(Utc::now().to_rfc3339()),
                clay_tools_used: None,
                processing_time_ms: None,
                file_attachments: None,
                tool_usages: None,
            };
            save_message(&state.db_pool, &conversation_id, &user_msg_with_id).await?;
            
            // Check conversation state for title updates (only if not a new conversation)
            if !is_new_conversation {
                let conv_info = sqlx::query_as::<_, (Option<String>, Option<bool>, i32)>(
                    "SELECT 
                        c.title, 
                        c.is_title_manually_set, 
                        (
                            SELECT COUNT(*)::INTEGER 
                            FROM messages m 
                            WHERE m.conversation_id = c.id
                            AND (m.is_forgotten = false OR m.is_forgotten IS NULL)
                        ) AS message_count
                     FROM conversations c 
                     WHERE c.id = $1"
                )
                .bind(&conversation_id)
                .fetch_one(&state.db_pool)
                .await
                .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;
                
                let (current_title, is_manually_set, message_count) = conv_info;
                let is_manually_set = is_manually_set.unwrap_or(false);
                
                // Only update title if it wasn't manually set
                if !is_manually_set {
                    let should_update = if let Some(ref title) = current_title {
                        // Check if it's still a placeholder or topic has changed
                        if title == "Untitled conversation" || title.is_empty() {
                            true
                        } else {
                            has_topic_changed(title, &last_msg.content, message_count)
                        }
                    } else {
                        true // No title yet
                    };
                    
                    if should_update {
                        let new_title = generate_title_from_message(&last_msg.content);
                        sqlx::query(
                            "UPDATE conversations SET title = $1, updated_at = $2 WHERE id = $3"
                        )
                        .bind(&new_title)
                        .bind(Utc::now())
                        .bind(&conversation_id)
                        .execute(&state.db_pool)
                        .await
                        .map_err(|e| AppError::InternalServerError(format!("Failed to update conversation title: {}", e)))?;
                        
                        tracing::info!("Updated conversation {} title from '{}' to: '{}' (topic change detected)", 
                            conversation_id, 
                            current_title.unwrap_or_else(|| "None".to_string()),
                            new_title
                        );
                    }
                }
            }
        }
        
        // Add the new message to conversation context
        match last_msg.role {
            MessageRole::User => conversation.push_str(&format!("User: {}\n", last_msg.content)),
            MessageRole::Assistant => conversation.push_str(&format!("Assistant: {}\n", last_msg.content)),
            MessageRole::System => conversation.push_str(&format!("System: {}\n", last_msg.content)),
        }
    }
    
    // Configure query options with context-aware system prompt
    // Generate a message_id early so we can include it in the prompt
    let assistant_message_id = Uuid::new_v4().to_string();
    
    let system_prompt = format!(
        "You are a helpful AI assistant integrated into Clay Studio.\n\n\
        IMPORTANT: When using any MCP tools (especially data_query), you MUST include these additional parameters:\n\
        - _conversation_id: {}\n\
        - _message_id: {}\n\n\
        These parameters are critical for tracking and must be included in EVERY tool call.",
        conversation_id, assistant_message_id
    );
    
    let options = QueryOptions {
        system_prompt: Some(system_prompt),
        max_turns: Some(1),
        allowed_tools: None,
        permission_mode: None,
        resume_session_id: None,
        output_format: None,
    };

    // Create a channel for SSE events
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<SseEvent, std::io::Error>>(100);
    
    // Save assistant message stub BEFORE spawning task so tool_usages can reference it
    if let Err(e) = sqlx::query(
        "INSERT INTO messages (id, conversation_id, content, role, created_at) 
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (id) DO NOTHING"
    )
    .bind(&assistant_message_id)
    .bind(&conversation_id)
    .bind("")  // Empty content for now, will update later
    .bind("assistant")
    .bind(Utc::now())
    .execute(&state.db_pool)
    .await {
        tracing::error!("Failed to create assistant message stub: {}", e);
    } else {
        tracing::info!("Created assistant message stub with id: {}", assistant_message_id);
    }
    
    // Clone necessary data for the spawned task
    let project_id = chat_request.project_id.clone();
    let conversation_id_clone = conversation_id.clone();
    let db_pool = state.db_pool.clone();
    
    // Spawn task to process Claude messages
    tokio::spawn(async move {
        let start_time = std::time::Instant::now();
        let message_id = assistant_message_id;  // Use the pre-generated message_id
        let mut tools_used = Vec::new();
        let mut tool_usages: Vec<ToolUsage> = Vec::new();
        let mut tool_start_times: std::collections::HashMap<String, std::time::Instant> = std::collections::HashMap::new();
        
        // Send start event
        if let Ok(event) = SseEvent::default()
            .name("message")
            .json(StreamMessage::Start { 
                id: message_id.clone(),
                conversation_id: conversation_id_clone.clone(),
            }) {
            let _ = tx.send(Ok(event)).await;
        }
        
        // Execute the Claude query with project context
        let mut assistant_content = String::new();
        
        match ClaudeManager::query_claude_with_project_and_token(
            client_id, 
            &project_id,
            conversation.clone(), 
            Some(options),
            claude_token
        ).await {
            Ok(mut receiver) => {
                // Process streaming messages
                let mut accumulated_text = String::new();
                
                while let Some(message) = receiver.recv().await {
                    match message {
                        ClaudeMessage::Progress { content } => {
                            // Log the raw content for debugging
                            tracing::debug!("Received progress message: {}", content);
                            
                            // Parse the stream-json to extract text content
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                                // Check various message formats from Claude
                                if let Some(msg_type) = parsed.get("type").and_then(|v| v.as_str()) {
                                    tracing::debug!("Message type: {}", msg_type);
                                    
                                    // Handle text delta messages
                                    if msg_type == "text" {
                                        if let Some(text) = parsed.get("text").and_then(|v| v.as_str()) {
                                            accumulated_text.push_str(text);
                                            tracing::debug!("Accumulated text from 'text' type: {}", text);
                                        }
                                    }
                                    // Handle content_block_delta messages
                                    else if msg_type == "content_block_delta" {
                                        if let Some(delta) = parsed.get("delta") {
                                            if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                                                accumulated_text.push_str(text);
                                                tracing::debug!("Accumulated text from 'content_block_delta': {}", text);
                                            }
                                        }
                                    }
                                }
                            } else {
                                // If not JSON, it might be plain text
                                tracing::debug!("Content is not JSON, treating as plain text");
                            }
                            
                            // Send progress to frontend
                            if let Ok(event) = SseEvent::default()
                                .name("message")
                                .json(StreamMessage::Progress { content: content.clone() }) {
                                let _ = tx.send(Ok(event)).await;
                            }
                        }
                        ClaudeMessage::ToolUse { tool, args, tool_use_id } => {
                            let tool_start = std::time::Instant::now();
                            tools_used.push(tool.clone());
                            
                            // Store the start time for this tool invocation if we have a tool_use_id
                            if let Some(ref id) = tool_use_id {
                                tool_start_times.insert(id.clone(), tool_start);
                            }
                            
                            // Create tool usage record with parameters
                            // Start with executing status - results will be updated later
                            let executing_output = serde_json::json!({
                                "status": "executing",
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                "tool_use_id": tool_use_id.clone()
                            });
                            
                            let tool_usage = ToolUsage::new(
                                message_id.clone(),
                                tool.clone(),
                                Some(args.clone())
                            )
                            .with_output(executing_output.clone());
                            
                            tracing::info!("[TIMING] Tool usage detected at {:?}: {} with params: {:?}", 
                                chrono::Utc::now(), tool, args);
                            
                            // Save tool usage immediately to database so MCP server can update it
                            let tool_usage_id = tool_usage.id;
                            let save_start = std::time::Instant::now();
                            
                            if let Err(e) = sqlx::query(
                                "INSERT INTO tool_usages (id, message_id, tool_name, parameters, output, execution_time_ms, created_at)
                                 VALUES ($1, $2, $3, $4, $5, $6, $7)"
                            )
                            .bind(tool_usage.id)
                            .bind(&message_id)
                            .bind(&tool_usage.tool_name)  // Use tool_usage.tool_name to preserve exact name
                            .bind(&args)
                            .bind(&executing_output)
                            .bind(tool_usage.execution_time_ms)
                            .bind(Utc::now())
                            .execute(&db_pool)
                            .await {
                                tracing::error!("Failed to save tool usage immediately: {}", e);
                            } else {
                                let save_time = save_start.elapsed();
                                tracing::info!("[TIMING] Tool usage {} saved to DB in {:?}ms at {:?}", 
                                    tool_usage_id, save_time.as_millis(), chrono::Utc::now());
                            }
                            
                            tool_usages.push(tool_usage);
                            
                            tracing::info!("[TIMING] Total ToolUse processing took {:?}ms", 
                                tool_start.elapsed().as_millis());
                            
                            if let Ok(event) = SseEvent::default()
                                .name("message")
                                .json(StreamMessage::ToolUse { tool }) {
                                let _ = tx.send(Ok(event)).await;
                            }
                        }
                        ClaudeMessage::ToolResult { tool, result } => {
                            // Update the corresponding tool usage with the actual result
                            tracing::info!("Received ToolResult for {} with result: {:?}", tool, result);
                            
                            // Extract text content from the result if it's in the expected format
                            let result_text = if result.is_array() {
                                result.as_array()
                                    .and_then(|arr| arr.first())
                                    .and_then(|item| item.get("text"))
                                    .and_then(|text| text.as_str())
                                    .unwrap_or("")
                                    .to_string()
                            } else if result.is_object() {
                                if let Some(text) = result.get("text").and_then(|t| t.as_str()) {
                                    text.to_string()
                                } else {
                                    serde_json::to_string_pretty(&result).unwrap_or_default()
                                }
                            } else if result.is_string() {
                                result.as_str().unwrap_or("").to_string()
                            } else {
                                result.to_string()
                            };
                            
                            // Send tool result to frontend via SSE immediately
                            if !result_text.is_empty() {
                                if let Ok(event) = SseEvent::default()
                                    .name("message")
                                    .json(StreamMessage::ToolResult { 
                                        tool: tool.clone(),
                                        result: json!({
                                            "text": result_text.clone(),
                                            "raw": result.clone()
                                        }),
                                        tool_usage_id: tool_usages.iter()
                                            .find(|tu| tu.tool_name == tool)
                                            .map(|tu| tu.id.to_string())
                                            .unwrap_or_else(|| Uuid::new_v4().to_string()),
                                    }) {
                                    let _ = tx.send(Ok(event)).await;
                                }
                            }
                            
                            // Find the tool usage to update by matching tool_use_id from the output
                            if let Some(tool_usage) = tool_usages.iter_mut()
                                .find(|tu| {
                                    // Try to match by tool_use_id stored in the output
                                    if let Some(output) = &tu.output {
                                        if let Some(stored_tool_use_id) = output.get("tool_use_id").and_then(|id| id.as_str()) {
                                            return stored_tool_use_id == tool;
                                        }
                                    }
                                    // Fallback to matching by tool name and executing status
                                    tu.tool_name == tool && tu.output.as_ref()
                                        .and_then(|o| o.get("status"))
                                        .and_then(|s| s.as_str()) == Some("executing")
                                }) {
                                
                                // Calculate execution time if we have the start time
                                let execution_time_ms = if let Some(output) = &tool_usage.output {
                                    if let Some(tool_use_id) = output.get("tool_use_id").and_then(|id| id.as_str()) {
                                        tool_start_times.remove(tool_use_id)
                                            .map(|start| start.elapsed().as_millis() as i64)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };
                                
                                // Update the tool usage with the actual result
                                let actual_output = serde_json::json!({
                                    "status": "completed",
                                    "result": result_text,
                                    "raw_result": result,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                
                                // Update in memory
                                let tool_usage_id = tool_usage.id;
                                tool_usage.output = Some(actual_output.clone());
                                tool_usage.execution_time_ms = execution_time_ms;
                                
                                // Also update in database immediately with execution time
                                let update_start = std::time::Instant::now();
                                match sqlx::query(
                                    "UPDATE tool_usages 
                                     SET output = $1, execution_time_ms = $2
                                     WHERE id = $3"
                                )
                                .bind(&actual_output)
                                .bind(execution_time_ms)
                                .bind(tool_usage_id)
                                .execute(&db_pool)
                                .await {
                                    Ok(_) => {
                                        let update_time = update_start.elapsed();
                                        tracing::info!(
                                            "[TIMING] Tool usage {} updated with actual result and execution time {:?}ms in {:?}ms", 
                                            tool_usage_id, 
                                            execution_time_ms.unwrap_or(0),
                                            update_time.as_millis()
                                        );
                                    },
                                    Err(e) => {
                                        tracing::error!("Failed to update tool usage {} with result: {}", tool_usage_id, e);
                                    }
                                }
                            } else {
                                // Try to find in database by tool_use_id
                                tracing::info!("Trying to find tool usage in database by tool_use_id: {}", tool);
                                if let Ok(Some(row)) = sqlx::query_as::<_, (String,)>(
                                    "SELECT id FROM tool_usages 
                                     WHERE message_id = $1 
                                     AND output->>'tool_use_id' = $2
                                     AND output->>'status' = 'executing'
                                     ORDER BY created_at DESC
                                     LIMIT 1"
                                )
                                .bind(&message_id)
                                .bind(&tool)
                                .fetch_optional(&db_pool)
                                .await {
                                    let (found_tool_usage_id,) = row;
                                    
                                    // Calculate execution time if we have the start time
                                    let execution_time_ms = tool_start_times.remove(&tool)
                                        .map(|start| start.elapsed().as_millis() as i64);
                                    
                                    let actual_output = serde_json::json!({
                                        "status": "completed",
                                        "result": result_text,
                                        "raw_result": result,
                                        "timestamp": chrono::Utc::now().to_rfc3339()
                                    });
                                    
                                    if (sqlx::query(
                                        "UPDATE tool_usages 
                                         SET output = $1, execution_time_ms = $2
                                         WHERE id = $3"
                                    )
                                    .bind(&actual_output)
                                    .bind(execution_time_ms)
                                    .bind(&found_tool_usage_id)
                                    .execute(&db_pool)
                                    .await).is_ok() {
                                        tracing::info!("Tool usage {} updated with result and execution time {:?}ms from database lookup", 
                                            found_tool_usage_id, execution_time_ms.unwrap_or(0));
                                    }
                                } else {
                                    tracing::warn!("Could not find executing tool usage for {} to update with result", tool);
                                }
                            }
                        }
                        ClaudeMessage::Result { result } => {
                            // If we get an explicit result, use it instead of accumulated
                            tracing::debug!("Received Result message with content: {}", result);
                            assistant_content = result.clone();
                            accumulated_text.clear();
                            
                            // Result messages are also passed as-is to frontend
                            if let Ok(event) = SseEvent::default()
                                .name("message")
                                .json(StreamMessage::Content { content: result }) {
                                let _ = tx.send(Ok(event)).await;
                            }
                        }
                        ClaudeMessage::Error { error } => {
                            if let Ok(event) = SseEvent::default()
                                .name("message")
                                .json(StreamMessage::Error { error }) {
                                let _ = tx.send(Ok(event)).await;
                            }
                            break;
                        }
                        _ => continue,
                    }
                }
                
                // Use accumulated text if no explicit result was received
                if assistant_content.is_empty() && !accumulated_text.is_empty() {
                    assistant_content = accumulated_text;
                }
                
                // Keep tool usages in their current state - MCP server will update them directly
                // This preserves the actual results instead of overwriting with generic message
                for tool_usage in &tool_usages {
                    if let Some(output) = &tool_usage.output {
                        if output.get("status").and_then(|s| s.as_str()) == Some("executing") {
                            tracing::info!("Tool usage {} still executing - MCP server will update with results", tool_usage.tool_name);
                        }
                    }
                }
                
                // Save assistant message to database if we have content
                let processing_time_ms = start_time.elapsed().as_millis() as i64;
                
                if !assistant_content.is_empty() {
                    let assistant_message = Message {
                        id: message_id.clone(),
                        content: assistant_content.clone(),
                        role: MessageRole::Assistant,
                        created_at: Some(Utc::now().to_rfc3339()),
                        clay_tools_used: if tools_used.is_empty() { None } else { Some(tools_used.clone()) },
                        processing_time_ms: Some(processing_time_ms),
                        file_attachments: None,
                        tool_usages: if tool_usages.is_empty() { None } else { Some(tool_usages.clone()) },
                    };
                    
                    tracing::info!("Assistant message created with {} tool usages", tool_usages.len());
                    tracing::debug!("Saving assistant message to database - ID: {}, Content length: {}", 
                        message_id, assistant_content.len());
                    
                    if let Err(e) = save_message(&db_pool, &conversation_id_clone, &assistant_message).await {
                        tracing::error!("Failed to save assistant message: {}", e);
                    } else {
                        tracing::debug!("Successfully saved assistant message to database");
                    }
                } else {
                    tracing::warn!("Assistant content is empty, not saving to database");
                }
                
                // Send completion event
                if let Ok(event) = SseEvent::default()
                    .name("message")
                    .json(StreamMessage::Complete { 
                        id: message_id,
                        conversation_id: conversation_id_clone.clone(),
                        processing_time_ms,
                        tools_used,
                    }) {
                    let _ = tx.send(Ok(event)).await;
                }
            }
            Err(e) => {
                tracing::error!("Failed to query Claude: {}", e);
                if let Ok(event) = SseEvent::default()
                    .name("message")
                    .json(StreamMessage::Error { 
                        error: format!("Failed to query Claude: {}", e) 
                    }) {
                    let _ = tx.send(Ok(event)).await;
                }
            }
        }
    });

    // Create stream from channel receiver
    let sse_stream = async_stream::stream! {
        while let Some(event) = rx.recv().await {
            yield event;
        }
    };
    
    // Use the sse::stream function to properly convert the stream for SSE response
    sse::stream(res, sse_stream);
    Ok(())
}