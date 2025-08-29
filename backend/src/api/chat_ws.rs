use salvo::prelude::*;
use serde::Deserialize;
use serde_json::json;
use crate::models::{Message, MessageRole};
use crate::utils::{AppState, StreamingState};
use crate::utils::AppError;
use crate::core::claude::{ClaudeManager, QueryOptions, ClaudeMessage};
use chrono::Utc;
use uuid::Uuid;
use sqlx::{Row, PgPool};

use crate::api::websocket::{broadcast_to_subscribers, ServerMessage};

// Generate a concise title from the user's first message
fn generate_conversation_title(content: &str) -> String {
    let content = content.trim();
    
    // Remove file attachment mentions
    let content = if let Some(idx) = content.find("\n\nAttached files:") {
        &content[..idx]
    } else {
        content
    };
    
    // Take first line or sentence
    let title = content
        .lines()
        .next()
        .unwrap_or(content)
        .trim();
    
    // Truncate to reasonable length (50 chars)
    if title.len() > 50 {
        let truncated = &title[..47];
        // Find last word boundary to avoid cutting mid-word
        if let Some(last_space) = truncated.rfind(' ') {
            format!("{}...", &title[..last_space])
        } else {
            format!("{}...", truncated)
        }
    } else if title.is_empty() {
        "New Conversation".to_string()
    } else {
        title.to_string()
    }
}

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
) -> Result<(), AppError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM conversations WHERE id = $1 AND project_id = $2)"
    )
    .bind(conversation_id)
    .bind(project_id)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !exists {
        sqlx::query(
            "INSERT INTO conversations (id, project_id, title, created_at, is_title_manually_set) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(conversation_id)
        .bind(project_id)
        .bind("New Conversation")
        .bind(Utc::now())
        .bind(false) // Auto-generated title, not manually set
        .execute(pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to create conversation: {}", e)))?;
    }

    Ok(())
}

async fn save_message(
    pool: &PgPool,
    conversation_id: &str,
    message: &Message,
) -> Result<(), AppError> {
    // Save the message
    sqlx::query(
        "INSERT INTO messages (id, conversation_id, content, role, processing_time_ms, created_at) 
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (id) DO UPDATE SET
            content = EXCLUDED.content,
            processing_time_ms = EXCLUDED.processing_time_ms"
    )
    .bind(&message.id)
    .bind(conversation_id)
    .bind(&message.content)
    .bind(match message.role {
        MessageRole::System => "system",
        MessageRole::User => "user", 
        MessageRole::Assistant => "assistant",
    })
    .bind(message.processing_time_ms)
    .bind(message.created_at.as_ref().map(|dt| dt.parse::<chrono::DateTime<Utc>>().unwrap()).unwrap_or(Utc::now()))
    .execute(pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to save message: {}", e)))?;

    // Save tool usages if present
    if let Some(tool_usages) = &message.tool_usages {
        for tool_usage in tool_usages {
            sqlx::query(
                "INSERT INTO tool_usages (id, message_id, tool_name, parameters, output, execution_time_ms, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (id) DO UPDATE SET
                    parameters = EXCLUDED.parameters,
                    output = EXCLUDED.output,
                    execution_time_ms = EXCLUDED.execution_time_ms"
            )
            .bind(&tool_usage.id)
            .bind(&message.id)
            .bind(&tool_usage.tool_name)
            .bind(&tool_usage.parameters)
            .bind(&tool_usage.output)
            .bind(tool_usage.execution_time_ms)
            .bind(tool_usage.created_at.as_ref().map(|dt| dt.parse::<chrono::DateTime<Utc>>().unwrap()).unwrap_or(Utc::now()))
            .execute(pool)
            .await
            .map_err(|e| AppError::InternalServerError(format!("Failed to save tool usage: {}", e)))?;
        }
    }

    Ok(())
}

#[handler]
pub async fn handle_chat_stream_ws(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let chat_request: ChatRequest = req.parse_json().await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;

    // Get the first active client from the database
    let client_row = sqlx::query(
        "SELECT id, claude_token FROM clients WHERE status = 'active' AND claude_token IS NOT NULL LIMIT 1"
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let (client_id, claude_token) = if let Some(row) = client_row {
        let id: Uuid = row.get("id");
        let token: Option<String> = row.get("claude_token");
        (id, token)
    } else {
        return Err(AppError::ServiceUnavailable(
            "No active Claude client available. Please set up a client first.".to_string()
        ));
    };

    tracing::info!(
        "Using client {} for WebSocket streaming chat request with project {} (conversation: {:?})", 
        client_id, 
        chat_request.project_id,
        chat_request.conversation_id
    );

    // Generate conversation ID
    let conversation_id = match chat_request.conversation_id.as_deref() {
        Some("new") | None => format!("conv-{}-{}", chrono::Utc::now().timestamp_millis(), Uuid::new_v4()),
        Some(id) => id.to_string(),
    };

    let _is_new_conversation = conversation_id.starts_with("conv-") || conversation_id == "new";

    // Ensure conversation exists
    ensure_conversation(&state.db_pool, &conversation_id, &chat_request.project_id).await?;

    // Generate assistant message ID
    let assistant_message_id = Uuid::new_v4().to_string();
    
    // Track tool usages for this message
    let _tracked_tool_usages: Vec<crate::models::tool_usage::ToolUsage> = Vec::new();

    // Save user message if provided
    if let Some(last_msg) = chat_request.messages.last() {
        if last_msg.role == MessageRole::User {
            let existing_user_msg = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(
                    SELECT 1 FROM messages 
                    WHERE conversation_id = $1 
                    AND role = 'user' 
                    AND content = $2 
                    AND created_at > NOW() - INTERVAL '30 seconds'
                )"
            )
            .bind(&conversation_id)
            .bind(&last_msg.content)
            .fetch_one(&state.db_pool)
            .await
            .unwrap_or(false);
            
            if !existing_user_msg {
                let user_msg_with_id = Message {
                    id: Uuid::new_v4().to_string(),
                    content: last_msg.content.clone(),
                    role: last_msg.role.clone(),
                    created_at: Some(Utc::now().to_rfc3339()),
                    processing_time_ms: None,
                    file_attachments: None,
                    tool_usages: None,
                };
                save_message(&state.db_pool, &conversation_id, &user_msg_with_id).await?;
            }
        }
    }

    // Create assistant message stub
    if let Err(e) = sqlx::query(
        "INSERT INTO messages (id, conversation_id, content, role, created_at) 
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (id) DO NOTHING"
    )
    .bind(&assistant_message_id)
    .bind(&conversation_id)
    .bind("")
    .bind("assistant")
    .bind(Utc::now())
    .execute(&state.db_pool)
    .await {
        tracing::error!("Failed to create assistant message stub: {}", e);
    }

    // Prepare conversation string for Claude
    // Claude Code SDK handles message compaction internally to stay within context limits
    let mut conversation_text = String::new();
    
    // System message (always included)
    let system_message = format!("System: You are Claude Code, Anthropic's CLI tool for developers. Project: {}\n", chat_request.project_id);
    conversation_text.push_str(&system_message);
    
    // Track context usage
    let mut total_chars = system_message.len();
    let mut message_count = 0;
    
    // Load existing messages from database unless this is a brand new conversation
    // We should load messages for all existing conversations
    if conversation_id != "new" {
        let existing_messages = sqlx::query_as::<_, (String, String, chrono::DateTime<Utc>)>(
            "SELECT content, role, created_at FROM messages 
             WHERE conversation_id = $1 
             ORDER BY created_at ASC"
        )
        .bind(&conversation_id)
        .fetch_all(&state.db_pool)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("Failed to load conversation history: {}", e);
            Vec::new()
        });
        
        message_count = existing_messages.len();
        
        tracing::info!(
            "Found {} existing messages in database for conversation {}",
            message_count,
            conversation_id
        );
        
        // Add all historical messages - Claude SDK will handle compaction
        for (content, role, _created_at) in &existing_messages {
            let formatted = match role.as_str() {
                "user" => format!("User: {}\n", content),
                "assistant" => format!("Assistant: {}\n", content),
                "system" => format!("System: {}\n", content),
                _ => continue,
            };
            total_chars += formatted.len();
            conversation_text.push_str(&formatted);
        }
        
        tracing::info!(
            "Loaded {} messages for conversation {} ({} chars) - Claude SDK will handle compaction",
            message_count,
            conversation_id,
            total_chars
        );
    }
    
    // Add new message from request (only the last user message if it's new)
    if let Some(last_msg) = chat_request.messages.last() {
        if last_msg.role == MessageRole::User {
            // Check if this message is already in the database (avoid duplicates)
            let is_duplicate = if conversation_id != "new" {
                sqlx::query_scalar::<_, bool>(
                    "SELECT EXISTS(
                        SELECT 1 FROM messages 
                        WHERE conversation_id = $1 
                        AND role = 'user' 
                        AND content = $2 
                        AND created_at > NOW() - INTERVAL '5 seconds'
                    )"
                )
                .bind(&conversation_id)
                .bind(&last_msg.content)
                .fetch_one(&state.db_pool)
                .await
                .unwrap_or(false)
            } else {
                false
            };
            
            if !is_duplicate {
                let new_msg = format!("User: {}\n", last_msg.content);
                total_chars += new_msg.len();
                message_count += 1;
                conversation_text.push_str(&new_msg);
            }
        }
    }
    
    // Calculate context usage (Claude has ~200k token limit, roughly 800k chars)
    const MAX_CONTEXT_CHARS: usize = 800_000;
    let context_percentage = ((total_chars as f64 / MAX_CONTEXT_CHARS as f64) * 100.0).min(100.0);
    
    // Estimate if compaction will be needed
    let needs_compaction = total_chars > MAX_CONTEXT_CHARS / 2; // Over 50% means compaction likely
    
    tracing::info!(
        "Context usage: {}% ({}/{} chars, {} messages){}",
        context_percentage as u32,
        total_chars,
        MAX_CONTEXT_CHARS,
        message_count,
        if needs_compaction { " - compaction will be applied" } else { "" }
    );
    
    // Log first 500 chars of conversation for debugging
    let preview = if conversation_text.len() > 500 {
        format!("{}...", &conversation_text[..500])
    } else {
        conversation_text.clone()
    };
    tracing::info!("Sending to Claude (first 500 chars): {}", preview);

    let options = QueryOptions {
        max_turns: Some(1),
        allowed_tools: None,
        permission_mode: None,
        resume_session_id: None,
        output_format: None,
    };

    // Clone necessary data for the spawned task
    let project_id = chat_request.project_id.clone();
    let conversation_id_clone = conversation_id.clone();
    let db_pool = state.db_pool.clone();
    let active_claude_streams = state.active_claude_streams.clone();
    let context_info = (total_chars, MAX_CONTEXT_CHARS, context_percentage, message_count, needs_compaction);
    
    // Register this stream as active
    {
        let mut streams = active_claude_streams.write().await;
        streams.insert(conversation_id.clone(), StreamingState {
            message_id: assistant_message_id.clone(),
            partial_content: String::new(),
            last_updated: Utc::now(),
            active_tools: Vec::new(),
        });
    }

    // Spawn task to process Claude messages
    tokio::spawn(async move {
        let start_time = std::time::Instant::now();
        let message_id = assistant_message_id;
        let mut tool_usages: Vec<crate::models::tool_usage::ToolUsage> = Vec::new();
        let mut pending_tools: std::collections::HashMap<String, (String, serde_json::Value, std::time::Instant)> = std::collections::HashMap::new();
        let mut assistant_content = String::new();
        
        // Send context usage info
        let (total_chars, max_chars, percentage, message_count, needs_compaction) = context_info;
        broadcast_to_subscribers(
            &project_id,
            &conversation_id_clone,
            ServerMessage::ContextUsage {
                conversation_id: conversation_id_clone.clone(),
                total_chars,
                max_chars,
                percentage: percentage as f32,
                message_count,
                needs_compaction,
            }
        ).await;
        
        // Send start event via WebSocket
        broadcast_to_subscribers(
            &project_id,
            &conversation_id_clone,
            ServerMessage::Start { 
                id: message_id.clone(),
                conversation_id: conversation_id_clone.clone(),
            }
        ).await;
        
        // Execute the Claude query with project context
        match ClaudeManager::query_claude_with_project_and_token(
            client_id, 
            &project_id,
            conversation_text,
            Some(options),
            claude_token
        ).await {
            Ok(mut receiver) => {
                let mut accumulated_text = String::new();
                
                tracing::info!("Starting to receive messages from Claude SDK");
                while let Some(message) = receiver.recv().await {
                    // Log all received messages for debugging
                    tracing::debug!("chat_ws received message type: {:?}", std::mem::discriminant(&message));
                    
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
                                            
                                            // Update active stream with partial content
                                            if let Ok(mut streams) = active_claude_streams.try_write() {
                                                if let Some(stream_state) = streams.get_mut(&conversation_id_clone) {
                                                    stream_state.partial_content = accumulated_text.clone();
                                                    stream_state.last_updated = Utc::now();
                                                }
                                            }
                                            
                                            // Send accumulated text as progress
                                            broadcast_to_subscribers(
                                                &project_id,
                                                &conversation_id_clone,
                                                ServerMessage::Progress { 
                                                    content: accumulated_text.clone(),
                                                    conversation_id: conversation_id_clone.clone()
                                                }
                                            ).await;
                                        }
                                    }
                                    // Handle content_block_delta messages
                                    else if msg_type == "content_block_delta" {
                                        if let Some(delta) = parsed.get("delta") {
                                            if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                                                accumulated_text.push_str(text);
                                                tracing::debug!("Accumulated text from 'content_block_delta': {}", text);
                                                
                                                // Update active stream with partial content
                                                if let Ok(mut streams) = active_claude_streams.try_write() {
                                                    if let Some(stream_state) = streams.get_mut(&conversation_id_clone) {
                                                        stream_state.partial_content = accumulated_text.clone();
                                                        stream_state.last_updated = Utc::now();
                                                    }
                                                }
                                                
                                                // Send accumulated text as progress
                                                broadcast_to_subscribers(
                                                    &project_id,
                                                    &conversation_id_clone,
                                                    ServerMessage::Progress { 
                                                        content: accumulated_text.clone(),
                                                        conversation_id: conversation_id_clone.clone()
                                                    }
                                                ).await;
                                            }
                                        }
                                    }
                                    else if msg_type == "assistant" {
                                        // Handle assistant messages - extract text from message.content[0].text
                                        if let Some(message) = parsed.get("message") {
                                            if let Some(content_array) = message.get("content").and_then(|c| c.as_array()) {
                                                for content_item in content_array {
                                                    if let Some(item_type) = content_item.get("type").and_then(|t| t.as_str()) {
                                                        if item_type == "text" {
                                                            if let Some(text) = content_item.get("text").and_then(|t| t.as_str()) {
                                                                // This is the assistant's text response
                                                                accumulated_text = text.to_string();
                                                                
                                                                // Update active stream
                                                                if let Ok(mut streams) = active_claude_streams.try_write() {
                                                                    if let Some(stream_state) = streams.get_mut(&conversation_id_clone) {
                                                                        stream_state.partial_content = accumulated_text.clone();
                                                                        stream_state.last_updated = Utc::now();
                                                                    }
                                                                }
                                                                
                                                                // Send as progress (full content)
                                                                broadcast_to_subscribers(
                                                                    &project_id,
                                                                    &conversation_id_clone,
                                                                    ServerMessage::Progress { 
                                                                        content: accumulated_text.clone(),
                                                                        conversation_id: conversation_id_clone.clone()
                                                                    }
                                                                ).await;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                // If not JSON, it might be plain text
                                tracing::debug!("Content is not JSON, treating as plain text");
                            }
                        }
                        
                        ClaudeMessage::ToolUse { tool, args, tool_use_id } => {
                            tracing::info!("Received ClaudeMessage::ToolUse for tool: {}", tool);
                            
                            // Track the pending tool usage with its parameters
                            let tool_start = std::time::Instant::now();
                            if let Some(id) = &tool_use_id {
                                pending_tools.insert(id.clone(), (tool.clone(), args.clone(), tool_start));
                            } else {
                                // If no tool_use_id, create one based on the tool name
                                let generated_id = format!("{}_{}", tool, uuid::Uuid::new_v4());
                                pending_tools.insert(generated_id, (tool.clone(), args.clone(), tool_start));
                            }
                            
                            // Update active tools in streaming state
                            {
                                let mut streams = active_claude_streams.write().await;
                                if let Some(stream_state) = streams.get_mut(&conversation_id_clone) {
                                    // Only add tool if it's not already in the list (prevent duplicates)
                                    if !stream_state.active_tools.contains(&tool) {
                                        stream_state.active_tools.push(tool.clone());
                                    }
                                    stream_state.last_updated = Utc::now();
                                }
                            }
                            
                            // Send tool_use via WebSocket
                            tracing::info!("Broadcasting ToolUse message for tool: {}", tool);
                            broadcast_to_subscribers(
                                &project_id,
                                &conversation_id_clone,
                                ServerMessage::ToolUse { 
                                    tool: tool.clone(),
                                    conversation_id: conversation_id_clone.clone()
                                }
                            ).await;
                        }
                        
                        ClaudeMessage::ToolResult { tool, result } => {
                            tracing::info!("Received ToolResult for: {}", tool);
                            
                            // Check if this is a tool_use_id or actual tool name
                            let (_actual_tool_name, _tool_use_id) = if crate::utils::mcp_tools::is_tool_use_id(&tool) {
                                // It's a tool_use_id, try to find the actual tool name from pending tools
                                if let Some((name, params, start_time)) = pending_tools.remove(&tool) {
                                    // Create a complete ToolUsage record
                                    let execution_time = start_time.elapsed().as_millis() as i64;
                                    let tool_usage = crate::models::tool_usage::ToolUsage {
                                        id: uuid::Uuid::new_v4(),
                                        message_id: message_id.clone(),
                                        tool_name: name.clone(),
                                        parameters: Some(params),
                                        output: Some(result.clone()),
                                        execution_time_ms: Some(execution_time),
                                        created_at: Some(chrono::Utc::now().to_rfc3339()),
                                    };
                                    tool_usages.push(tool_usage);
                                    (name, tool.clone())
                                } else {
                                    // No pending tool found, just track the name
                                    (tool.clone(), tool.clone())
                                }
                            } else {
                                // It's already a tool name, create a ToolUsage without matching to pending
                                let tool_usage = crate::models::tool_usage::ToolUsage {
                                    id: uuid::Uuid::new_v4(),
                                    message_id: message_id.clone(),
                                    tool_name: tool.clone(),
                                    parameters: None,
                                    output: Some(result.clone()),
                                    execution_time_ms: None,
                                    created_at: Some(chrono::Utc::now().to_rfc3339()),
                                };
                                tool_usages.push(tool_usage);
                                (tool.clone(), format!("result_{}", uuid::Uuid::new_v4()))
                            };
                            
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
                            
                            // Log tool results for debugging
                            tracing::debug!("Tool {} returned result: {}", tool, result_text);
                        }
                        
                        ClaudeMessage::Result { result } => {
                            // If we get an explicit result, use it instead of accumulated
                            tracing::debug!("Received Result message with content: {}", result);
                            assistant_content = result.clone();
                            accumulated_text.clear();
                            
                            // Send final content via WebSocket
                            broadcast_to_subscribers(
                                &project_id,
                                &conversation_id_clone,
                                ServerMessage::Content { 
                                    content: result,
                                    conversation_id: conversation_id_clone.clone()
                                }
                            ).await;
                            // Don't break here - continue processing messages (like tool usage events)
                            // The channel will close naturally when the SDK is done
                        }
                        
                        ClaudeMessage::Error { error } => {
                            broadcast_to_subscribers(
                                &project_id,
                                &conversation_id_clone,
                                ServerMessage::Error { 
                                    error,
                                    conversation_id: conversation_id_clone.clone()
                                }
                            ).await;
                            break;
                        }
                        
                        ClaudeMessage::AskUser { 
                            prompt_type, 
                            title, 
                            options, 
                            input_type, 
                            placeholder, 
                            tool_use_id 
                        } => {
                            tracing::info!("Received AskUser message with prompt_type: {}", prompt_type);
                            
                            // Convert AskUserOption to serde_json::Value for the ServerMessage
                            let options_value = options.map(|opts| {
                                opts.into_iter()
                                    .map(|opt| serde_json::to_value(opt).unwrap_or(serde_json::Value::Null))
                                    .collect::<Vec<_>>()
                            });
                            
                            broadcast_to_subscribers(
                                &project_id,
                                &conversation_id_clone,
                                ServerMessage::AskUser {
                                    prompt_type,
                                    title,
                                    options: options_value,
                                    input_type,
                                    placeholder,
                                    tool_use_id,
                                    conversation_id: conversation_id_clone.clone(),
                                }
                            ).await;
                        }
                        
                        _ => continue,
                    }
                }
                
                // Use accumulated text if no explicit result was received
                if assistant_content.is_empty() && !accumulated_text.is_empty() {
                    assistant_content = accumulated_text;
                }
                
                // Save final assistant message (even if partial due to abort)
                if !assistant_content.is_empty() {
                    let assistant_message = Message {
                        id: message_id.clone(),
                        content: assistant_content,
                        role: MessageRole::Assistant,
                        created_at: Some(Utc::now().to_rfc3339()),
                        processing_time_ms: Some(start_time.elapsed().as_millis() as i64),
                        file_attachments: None,
                        tool_usages: if tool_usages.is_empty() { None } else { Some(tool_usages.clone()) },
                    };
                    
                    if let Err(e) = save_message(&db_pool, &conversation_id_clone, &assistant_message).await {
                        tracing::error!("Failed to save assistant message: {}", e);
                    }
                }
                
                // Generate title if this is the first exchange and title is not manually set
                if let Ok(is_first_exchange) = sqlx::query_scalar::<_, bool>(
                    "SELECT (COUNT(*) = 2 AND is_title_manually_set = false) 
                     FROM conversations c 
                     JOIN messages m ON m.conversation_id = c.id 
                     WHERE c.id = $1 
                     GROUP BY c.is_title_manually_set"
                )
                .bind(&conversation_id_clone)
                .fetch_optional(&db_pool)
                .await {
                    if is_first_exchange.unwrap_or(false) {
                        // Get the first user message for title generation
                        if let Ok(first_user_message) = sqlx::query_scalar::<_, String>(
                            "SELECT content FROM messages 
                             WHERE conversation_id = $1 AND role = 'user' 
                             ORDER BY created_at ASC LIMIT 1"
                        )
                        .bind(&conversation_id_clone)
                        .fetch_optional(&db_pool)
                        .await {
                            if let Some(user_content) = first_user_message {
                                // Generate a concise title from the user's message
                                let generated_title = generate_conversation_title(&user_content);
                                
                                // Update the conversation title
                                if let Err(e) = sqlx::query(
                                    "UPDATE conversations 
                                     SET title = $1, updated_at = $2 
                                     WHERE id = $3 AND is_title_manually_set = false"
                                )
                                .bind(&generated_title)
                                .bind(Utc::now())
                                .bind(&conversation_id_clone)
                                .execute(&db_pool)
                                .await {
                                    tracing::error!("Failed to update conversation title: {}", e);
                                } else {
                                    tracing::info!("Auto-generated title for conversation {}: {}", conversation_id_clone, generated_title);
                                    
                                    // Broadcast title update via WebSocket
                                    broadcast_to_subscribers(
                                        &project_id,
                                        &conversation_id_clone,
                                        ServerMessage::TitleUpdated { 
                                            conversation_id: conversation_id_clone.clone(),
                                            title: generated_title
                                        }
                                    ).await;
                                }
                            }
                        }
                    }
                }
                
                // Remove from active streams
                {
                    let mut streams = active_claude_streams.write().await;
                    streams.remove(&conversation_id_clone);
                }
                
                // Always send completion event (broadcast handles no connections gracefully)
                {
                    // Send completion event
                    broadcast_to_subscribers(
                        &project_id,
                        &conversation_id_clone,
                        ServerMessage::Complete { 
                            id: message_id,
                            conversation_id: conversation_id_clone.clone(),
                            processing_time_ms: start_time.elapsed().as_millis() as u64,
                            tools_used: tool_usages.iter().map(|tu| tu.tool_name.clone()).collect(),
                        }
                    ).await;
                }
            }
            
            Err(e) => {
                tracing::error!("Failed to query Claude: {}", e);
                
                // Remove from active streams
                {
                    let mut streams = active_claude_streams.write().await;
                    streams.remove(&conversation_id_clone);
                }
                
                broadcast_to_subscribers(
                    &project_id,
                    &conversation_id_clone,
                    ServerMessage::Error { 
                        error: format!("Failed to query Claude: {}", e),
                        conversation_id: conversation_id_clone.clone()
                    }
                ).await;
            }
        }
    });

    // Return success immediately (WebSocket handles the streaming)
    res.render(Json(json!({"success": true, "conversation_id": conversation_id})));
    Ok(())
}