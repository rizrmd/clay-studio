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
            "INSERT INTO conversations (id, project_id, title, created_at) VALUES ($1, $2, $3, $4)"
        )
        .bind(conversation_id)
        .bind(project_id)
        .bind("New Conversation")
        .bind(Utc::now())
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

    // Prepare conversation string for Claude (same format as chat.rs)
    let mut conversation_text = String::new();
    
    // Add system message
    conversation_text.push_str(&format!("System: You are Claude Code, Anthropic's CLI tool for developers. Project: {}\n", chat_request.project_id));
    
    // Add all messages to conversation
    for msg in &chat_request.messages {
        match msg.role {
            MessageRole::User => conversation_text.push_str(&format!("User: {}\n", msg.content)),
            MessageRole::Assistant => conversation_text.push_str(&format!("Assistant: {}\n", msg.content)),
            MessageRole::System => conversation_text.push_str(&format!("System: {}\n", msg.content)),
        }
    }

    let options = QueryOptions {
        system_prompt: Some("You are Clay Studio, an AI data analysis assistant.".to_string()),
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
                                                                tracing::info!("Broadcasting assistant content: {} chars", accumulated_text.len());
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