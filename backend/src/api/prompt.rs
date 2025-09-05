use crate::core::claude::{ClaudeMessage, ClaudeSDK, QueryRequest};
use crate::utils::AppError;
use crate::utils::AppState;
use salvo::prelude::*;
use salvo::sse::{self, SseEvent};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct PromptRequest {
    pub prompt: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub max_turns: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum PromptStreamMessage {
    #[serde(rename = "start")]
    Start { id: String },
    #[serde(rename = "progress")]
    Progress { content: serde_json::Value },
    #[serde(rename = "tool_use")]
    ToolUse { tool: String },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool: String,
        result: serde_json::Value,
    },
    #[serde(rename = "content")]
    Content { content: String },
    #[serde(rename = "complete")]
    Complete {
        id: String,
        processing_time_ms: i64,
        tools_used: Vec<String>,
    },
    #[serde(rename = "error")]
    Error { error: String },
}

#[handler]
pub async fn handle_prompt_stream(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = depot.obtain::<AppState>().unwrap();
    let prompt_request: PromptRequest = req
        .parse_json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;

    // Validate that prompt is provided
    if prompt_request.prompt.trim().is_empty() {
        return Err(AppError::BadRequest("No prompt provided".to_string()));
    }

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
        (
            id,
            token.ok_or_else(|| {
                AppError::ServiceUnavailable("No Claude token available".to_string())
            })?,
        )
    } else {
        return Err(AppError::ServiceUnavailable(
            "No active Claude client available. Please set up a client first.".to_string(),
        ));
    };

    tracing::info!("Using client {} for one-shot prompt request", client_id);

    // Create a channel for SSE events
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<SseEvent, std::io::Error>>(100);

    // Clone necessary data for the spawned task
    let _db_pool = state.db_pool.clone();

    // Spawn task to process Claude messages
    tokio::spawn(async move {
        let start_time = std::time::Instant::now();
        let prompt_id = Uuid::new_v4().to_string();
        let mut tools_used = Vec::new();

        // Send start event
        if let Ok(event) = SseEvent::default()
            .name("message")
            .json(PromptStreamMessage::Start {
                id: prompt_id.clone(),
            })
        {
            let _ = tx.send(Ok(event)).await;
        }

        // Create Claude SDK instance
        let claude_sdk = ClaudeSDK::new(client_id, Some(claude_token));

        // Create query request
        let query_request = QueryRequest {
            prompt: prompt_request.prompt,
            options: None,
        };

        // Execute the Claude query
        let mut assistant_content = String::new();

        match claude_sdk.query(query_request).await {
            Ok(mut receiver) => {
                // Process streaming messages
                let mut accumulated_text = String::new();

                while let Some(message) = receiver.recv().await {
                    match message {
                        ClaudeMessage::Progress { content } => {
                            // Send progress to frontend without any parsing
                            if let Ok(event) = SseEvent::default()
                                .name("message")
                                .json(PromptStreamMessage::Progress { content })
                            {
                                let _ = tx.send(Ok(event)).await;
                            }
                        }
                        ClaudeMessage::ToolUse {
                            tool,
                            args: _,
                            tool_use_id: _,
                        } => {
                            tools_used.push(tool.clone());

                            if let Ok(event) = SseEvent::default()
                                .name("message")
                                .json(PromptStreamMessage::ToolUse { tool })
                            {
                                let _ = tx.send(Ok(event)).await;
                            }
                        }
                        ClaudeMessage::ToolResult { tool, result } => {
                            // Send tool result to frontend via SSE immediately
                            if let Ok(event) = SseEvent::default().name("message").json(
                                PromptStreamMessage::ToolResult {
                                    tool: tool.clone(),
                                    result: result.clone(),
                                },
                            ) {
                                let _ = tx.send(Ok(event)).await;
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
                                .json(PromptStreamMessage::Content { content: result })
                            {
                                let _ = tx.send(Ok(event)).await;
                            }
                        }
                        ClaudeMessage::Error { error } => {
                            if let Ok(event) = SseEvent::default()
                                .name("message")
                                .json(PromptStreamMessage::Error { error })
                            {
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

                // Calculate processing time
                let processing_time_ms = start_time.elapsed().as_millis() as i64;

                tracing::info!("One-shot prompt completed. Content length: {}, Processing time: {}ms, Tools used: {:?}", 
                    assistant_content.len(), processing_time_ms, tools_used);

                // Send completion event
                if let Ok(event) =
                    SseEvent::default()
                        .name("message")
                        .json(PromptStreamMessage::Complete {
                            id: prompt_id,
                            processing_time_ms,
                            tools_used,
                        })
                {
                    let _ = tx.send(Ok(event)).await;
                }
            }
            Err(e) => {
                tracing::error!("Failed to query Claude: {}", e);
                if let Ok(event) =
                    SseEvent::default()
                        .name("message")
                        .json(PromptStreamMessage::Error {
                            error: format!("Failed to query Claude: {}", e),
                        })
                {
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
