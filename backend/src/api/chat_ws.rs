use crate::models::{Message, MessageRole};
use crate::utils::{AppState, StreamingState};
use crate::utils::AppError;
use crate::core::claude::{ClaudeManager, QueryOptions, ClaudeMessage};
use chrono::Utc;
use sqlx::PgPool;

use crate::api::websocket::{broadcast_to_subscribers, ServerMessage};
use uuid;




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
                "INSERT INTO tool_usages (id, message_id, tool_name, tool_use_id, parameters, output, execution_time_ms, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                 ON CONFLICT (id) DO UPDATE SET
                    tool_use_id = EXCLUDED.tool_use_id,
                    parameters = EXCLUDED.parameters,
                    output = EXCLUDED.output,
                    execution_time_ms = EXCLUDED.execution_time_ms"
            )
            .bind(&tool_usage.id)
            .bind(&message.id)
            .bind(&tool_usage.tool_name)
            .bind(&tool_usage.tool_use_id)
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

// WebSocket-only message handler (replaces SSE streaming)
pub async fn handle_chat_message_ws(
    project_id: String,
    conversation_id: String,
    content: String,
    _uploaded_file_paths: Vec<String>,
    client_id_str: String,
    state: AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    
    tracing::info!("handle_chat_message_ws started: project={}, conversation={}, client={}", 
                   project_id, conversation_id, client_id_str);
    
    let db_pool = state.db_pool.clone();
    let active_claude_streams = state.active_claude_streams.clone();
    
    // Handle "new" conversation ID by creating in database and getting the ID
    let actual_conversation_id = if conversation_id == "new" {
        tracing::info!("Creating new conversation in database");
        let new_id = uuid::Uuid::new_v4().to_string();
        
        sqlx::query(
            "INSERT INTO conversations (id, project_id, title, created_at, is_title_manually_set) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(&new_id)
        .bind(&project_id)
        .bind("New Conversation")
        .bind(Utc::now())
        .bind(false)
        .execute(&db_pool)
        .await
        .map_err(|e| format!("Failed to create conversation: {}", e))?;
        
        tracing::info!("Created new conversation with ID: {}", new_id);
        
        // Send redirect message to frontend
        broadcast_to_subscribers(
            &project_id,
            "new", // Send to the original "new" subscription
            ServerMessage::ConversationRedirect {
                old_conversation_id: "new".to_string(),
                new_conversation_id: new_id.clone(),
            }
        ).await;
        
        new_id
    } else {
        conversation_id.clone()
    };
    
    let conversation_id_clone = actual_conversation_id.clone();
    
    // Insert user message first
    tracing::info!("Creating user message");
    let user_message = Message {
        id: uuid::Uuid::new_v4().to_string(),
        content: content.clone(),
        role: MessageRole::User,
        created_at: Some(Utc::now().to_rfc3339()),
        processing_time_ms: None,
        file_attachments: None, // WebSocket doesn't handle file attachments in this way
        tool_usages: None,
    };
    
    tracing::info!("Saving user message");
    if let Err(e) = save_message(&db_pool, &actual_conversation_id, &user_message).await {
        tracing::error!("Failed to save user message: {}", e);
        return Err(e.into());
    }
    tracing::info!("User message saved successfully");
    
    // Update the conversation cache with the new user message
    state.update_conversation_cache(&actual_conversation_id, user_message.clone()).await;
    
    // Get conversation history from cache (fast) or database (slow)
    tracing::info!("Getting conversation history for context");
    let cached_messages = state.get_conversation_messages(&actual_conversation_id)
        .await
        .map_err(|e| format!("Failed to get conversation history: {}", e))?;
    
    // Build the full prompt with conversation history
    let mut full_prompt = String::new();
    let mut history_count = 0;
    if !cached_messages.is_empty() {
        full_prompt.push_str("Previous conversation:\n\n");
        for msg in cached_messages.iter() {
            // Skip the current message we just added
            if msg.role == crate::models::MessageRole::User && msg.content == content {
                continue;
            }
            
            match msg.role {
                crate::models::MessageRole::User => {
                    full_prompt.push_str(&format!("User: {}\n\n", msg.content));
                    history_count += 1;
                },
                crate::models::MessageRole::Assistant => {
                    full_prompt.push_str(&format!("Assistant: {}\n", msg.content));
                    
                    // Include tool usages if present (already in cached message)
                    if let Some(ref tool_usages) = msg.tool_usages {
                        if !tool_usages.is_empty() {
                            full_prompt.push_str("\n[Tool Usage Details]:\n");
                            for tool in tool_usages {
                                full_prompt.push_str(&format!("- Tool: {}\n", tool.tool_name));
                                if let Some(ref params) = tool.parameters {
                                    full_prompt.push_str(&format!("  Parameters: {}\n", params));
                                }
                                if let Some(ref out) = tool.output {
                                    // Convert JSON to string and truncate if very long
                                    let output_str = if out.is_string() {
                                        out.as_str().unwrap_or("").to_string()
                                    } else {
                                        serde_json::to_string(&out).unwrap_or_else(|_| out.to_string())
                                    };
                                    
                                    let truncated_output = if output_str.len() > 500 {
                                        format!("{}... [truncated]", output_str.chars().take(497).collect::<String>())
                                    } else {
                                        output_str
                                    };
                                    full_prompt.push_str(&format!("  Output: {}\n", truncated_output));
                                }
                            }
                        }
                    }
                    full_prompt.push_str("\n");
                    history_count += 1;
                },
                _ => {}
            }
        }
        full_prompt.push_str("Current message:\n");
    }
    full_prompt.push_str(&content);
    
    tracing::info!("Built prompt with {} historical messages for conversation {}, total length: {} chars", 
                   history_count, actual_conversation_id, full_prompt.len());
    tracing::debug!("Full prompt preview (first 500 chars): {}", full_prompt.chars().take(500).collect::<String>());
    
    // Warn if prompt is getting too long (approaching OS limits)
    if full_prompt.len() > 100_000 {
        tracing::warn!("Prompt length ({} chars) may approach OS command line limits. Consider truncating older messages.", full_prompt.len());
    }
    
    let start_time = std::time::Instant::now();
    let message_id = uuid::Uuid::new_v4();
    
    // Track this conversation as actively streaming
    {
        let mut streams = active_claude_streams.write().await;
        streams.insert(conversation_id_clone.clone(), StreamingState {
            message_id: message_id.to_string(),
            partial_content: String::new(),
            active_tools: Vec::new(),
            progress_events: Vec::new(),
        });
    }
    
    // Send start event
    broadcast_to_subscribers(
        &project_id,
        &conversation_id_clone,
        ServerMessage::Start { 
            id: message_id.to_string(),
            conversation_id: conversation_id_clone.clone(),
        }
    ).await;
    
    // Execute the Claude query with project context
    tracing::info!("Parsing client ID and starting Claude query");
    let client_id = client_id_str.parse::<uuid::Uuid>().map_err(|e| format!("Invalid client ID: {}", e))?;
    tracing::info!("Client ID parsed successfully: {}", client_id);
    
    match ClaudeManager::query_claude_with_project_and_db(
        client_id,
        &project_id,
        full_prompt,
        Some(QueryOptions::default()),
        &db_pool,
    ).await {
        Ok(mut receiver) => {
            tracing::info!("Claude query successful, starting message loop");
            let _accumulated_text = String::new();
            let mut tool_usages = Vec::new();
            let mut pending_tools = std::collections::HashMap::new();
            let mut assistant_content = String::new();
            
            while let Some(message) = receiver.recv().await {
                tracing::info!("Received Claude message: {:?}", std::mem::discriminant(&message));
                
                match message {
                    ClaudeMessage::Progress { content } => {
                        // Store progress event for replay on reconnection
                        {
                            let mut streams = active_claude_streams.write().await;
                            if let Some(stream_state) = streams.get_mut(&conversation_id_clone) {
                                stream_state.progress_events.push(
                                    serde_json::json!({
                                        "type": "progress",
                                        "content": content
                                    })
                                );
                            }
                        }
                        
                        // Send progress via WebSocket without any parsing
                        broadcast_to_subscribers(
                            &project_id,
                            &conversation_id_clone,
                            ServerMessage::Progress { 
                                content,
                                conversation_id: conversation_id_clone.clone()
                            }
                        ).await;
                    }
                    
                    ClaudeMessage::ToolUse { tool, args, tool_use_id } => {
                        let tool_usage_id = uuid::Uuid::new_v4();
                        let lookup_key = tool_use_id.clone().unwrap_or_else(|| tool.clone());
                        pending_tools.insert(lookup_key.clone(), (tool.clone(), args.clone(), std::time::Instant::now(), tool_usage_id));
                        
                        // Store tool use event for replay on reconnection
                        {
                            let mut streams = active_claude_streams.write().await;
                            if let Some(stream_state) = streams.get_mut(&conversation_id_clone) {
                                stream_state.progress_events.push(
                                    serde_json::json!({
                                        "type": "tool_use",
                                        "tool": tool,
                                        "tool_usage_id": tool_usage_id.to_string()
                                    })
                                );
                            }
                        }
                        
                        // Send ToolUse via WebSocket
                        broadcast_to_subscribers(
                            &project_id,
                            &conversation_id_clone,
                            ServerMessage::ToolUse { 
                                tool: tool.clone(),
                                tool_usage_id: tool_usage_id.to_string(),
                                conversation_id: conversation_id_clone.clone()
                            }
                        ).await;
                    }
                    
                    ClaudeMessage::ToolResult { tool, result } => {
                        // Handle tool completion and send ToolComplete event
                        if let Some((name, params, start_time, tool_usage_id)) = pending_tools.remove(&tool) {
                            let execution_time = start_time.elapsed().as_millis() as u64;
                            
                            // Store tool complete event for replay on reconnection
                            {
                                let mut streams = active_claude_streams.write().await;
                                if let Some(stream_state) = streams.get_mut(&conversation_id_clone) {
                                    stream_state.progress_events.push(
                                        serde_json::json!({
                                            "type": "tool_complete",
                                            "tool": name,
                                            "tool_usage_id": tool_usage_id.to_string(),
                                            "execution_time_ms": execution_time as i64,
                                            "output": result
                                        })
                                    );
                                }
                            }
                            
                            // Send ToolComplete event
                            broadcast_to_subscribers(
                                &project_id,
                                &conversation_id_clone,
                                ServerMessage::ToolComplete {
                                    tool: name.clone(),
                                    tool_usage_id: tool_usage_id.to_string(),
                                    execution_time_ms: execution_time as i64,
                                    output: Some(result.clone()),
                                    conversation_id: conversation_id_clone.clone(),
                                }
                            ).await;
                            
                            // Add to tool_usages for final message
                            let tool_usage = crate::models::tool_usage::ToolUsage {
                                id: tool_usage_id,
                                message_id: message_id.to_string(),
                                tool_name: name.clone(),
                                tool_use_id: Some(tool.clone()),
                                parameters: Some(params),
                                output: Some(result.clone()),
                                execution_time_ms: Some(execution_time as i64),
                                created_at: None,
                            };
                            tool_usages.push(tool_usage);
                        }
                    }
                    
                    
                    ClaudeMessage::Result { result } => {
                        tracing::info!("Processing Result message: {} chars", result.len());
                        tracing::debug!("Result content preview: {}", result.chars().take(100).collect::<String>());
                        
                        // Parse JSON content if it's a JSON string
                        let actual_content = if result.starts_with('{') && result.ends_with('}') {
                            // Try to parse as JSON to extract readable content
                            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&result) {
                                // For Claude messages, extract the actual text from nested structure
                                if let Some(message_obj) = json_value.get("message") {
                                    if let Some(content_array) = message_obj.get("content").and_then(|v| v.as_array()) {
                                        if let Some(first_content) = content_array.get(0) {
                                            if let Some(text) = first_content.get("text").and_then(|v| v.as_str()) {
                                                text.to_string()
                                            } else {
                                                serde_json::to_string_pretty(&json_value).unwrap_or(result)
                                            }
                                        } else {
                                            serde_json::to_string_pretty(&json_value).unwrap_or(result)
                                        }
                                    } else {
                                        serde_json::to_string_pretty(&json_value).unwrap_or(result)
                                    }
                                } else if let Some(content_text) = json_value.get("content").and_then(|v| v.as_str()) {
                                    // Fallback for simpler JSON structure
                                    content_text.to_string()
                                } else {
                                    // If no extractable content, pretty print the JSON
                                    serde_json::to_string_pretty(&json_value).unwrap_or(result)
                                }
                            } else {
                                result
                            }
                        } else {
                            result
                        };
                        
                        assistant_content = actual_content.clone();
                        broadcast_to_subscribers(
                            &project_id,
                            &conversation_id_clone,
                            ServerMessage::Content { 
                                content: actual_content,
                                conversation_id: conversation_id_clone.clone()
                            }
                        ).await;
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
                        return Err("Claude SDK error".into());
                    }
                    
                    _ => continue,
                }
            }
            
            // Save final assistant message
            if !assistant_content.is_empty() {
                let assistant_message = Message {
                    id: message_id.to_string(),
                    content: assistant_content,
                    role: MessageRole::Assistant,
                    created_at: Some(Utc::now().to_rfc3339()),
                    processing_time_ms: Some(start_time.elapsed().as_millis() as i64),
                    file_attachments: None,
                    tool_usages: if tool_usages.is_empty() { None } else { Some(tool_usages.clone()) },
                };
                
                if let Err(e) = save_message(&db_pool, &actual_conversation_id, &assistant_message).await {
                    tracing::error!("Failed to save assistant message: {}", e);
                } else {
                    // Update the conversation cache with the new assistant message
                    state.update_conversation_cache(&actual_conversation_id, assistant_message).await;
                }
            }
            
            // Generate title if this is the first exchange
            let message_count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM messages WHERE conversation_id = $1"
            )
            .bind(&actual_conversation_id)
            .fetch_one(&db_pool)
            .await
            .unwrap_or(0);
            
            if message_count == 2 { // User + Assistant message
                let title = if content.len() > 50 {
                    format!("{}...", content.chars().take(47).collect::<String>().trim())
                } else if content.is_empty() {
                    "New Conversation".to_string()
                } else {
                    content.clone()
                };
                
                let _ = sqlx::query(
                    "UPDATE conversations SET title = $1 WHERE id = $2 AND is_title_manually_set = false"
                )
                .bind(&title)
                .bind(&actual_conversation_id)
                .execute(&db_pool)
                .await;
            }
            
            // Send completion event
            broadcast_to_subscribers(
                &project_id,
                &conversation_id_clone,
                ServerMessage::Complete { 
                    id: message_id.to_string(),
                    conversation_id: conversation_id_clone.clone(),
                    processing_time_ms: start_time.elapsed().as_millis() as u64,
                    tool_usages: if tool_usages.is_empty() { None } else { Some(tool_usages.clone()) },
                }
            ).await;
        }
        
        Err(e) => {
            let error_msg = e.to_string();
            tracing::error!("Failed to query Claude: {}", error_msg);
            
            // Send appropriate error message to client
            broadcast_to_subscribers(
                &project_id,
                &conversation_id_clone,
                ServerMessage::Error { 
                    error: error_msg.clone(),
                    conversation_id: conversation_id_clone.clone()
                }
            ).await;
            return Err(e.into());
        }
    }
    
    // Clear progress events from active streams (keep the state for reference)
    {
        let mut streams = active_claude_streams.write().await;
        if let Some(stream_state) = streams.get_mut(&conversation_id_clone) {
            let event_count = stream_state.progress_events.len();
            // Clear progress events as the message is complete and they're no longer needed
            stream_state.progress_events.clear();
            tracing::info!("Cleared {} progress events for completed message in conversation {}", 
                event_count, conversation_id_clone);
        }
    }
    
    Ok(())
}