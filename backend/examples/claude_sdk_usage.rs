// Example usage of the Claude SDK module
// Run with: cargo run --example claude_sdk_usage

use clay_studio_backend::claude::{ClaudeManager, QueryOptions, ClaudeMessage};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .init();

    // Example 1: Simple query without options
    simple_query_example().await?;
    
    // Example 2: Query with custom options
    query_with_options_example().await?;
    
    // Example 3: Streaming query with message handling
    streaming_query_example().await?;
    
    Ok(())
}

async fn simple_query_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Simple Query Example ===");
    
    // Use an existing client ID (you'd get this from your database)
    let client_id = Uuid::new_v4();
    
    // First ensure the client is set up (in production, this would be done once)
    // ClaudeManager::setup_client(client_id, None).await?;
    
    // Execute a simple query
    match ClaudeManager::query_claude_simple(
        client_id,
        "What is 2 + 2?".to_string()
    ).await {
        Ok(result) => println!("Result: {}", result),
        Err(e) => println!("Error: {}", e),
    }
    
    Ok(())
}

async fn query_with_options_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Query with Options Example ===");
    
    let client_id = Uuid::new_v4();
    
    // Configure advanced options
    let options = QueryOptions {
        system_prompt: Some("You are a helpful coding assistant.".to_string()),
        max_turns: Some(3),
        allowed_tools: Some(vec![
            "Read".to_string(),
            "Write".to_string(),
            "Bash".to_string()
        ]),
        permission_mode: Some("read_write".to_string()),
        resume_session_id: None,
        output_format: Some("json".to_string()),
    };
    
    let mut receiver = ClaudeManager::query_claude(
        client_id,
        "Write a simple Python hello world script".to_string(),
        Some(options)
    ).await?;
    
    // Process messages
    while let Some(message) = receiver.recv().await {
        match message {
            ClaudeMessage::Result { result } => {
                println!("Final Result: {}", result);
                break;
            }
            ClaudeMessage::Progress { content } => {
                println!("Progress: {}", content);
            }
            ClaudeMessage::ToolUse { tool, args } => {
                println!("Tool Used: {} with args: {}", tool, args);
            }
            ClaudeMessage::Error { error } => {
                println!("Error: {}", error);
                break;
            }
            ClaudeMessage::Start { session_id } => {
                println!("Session Started: {}", session_id);
            }
        }
    }
    
    Ok(())
}

async fn streaming_query_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Streaming Query Example ===");
    
    let client_id = Uuid::new_v4();
    
    // Create a complex multi-turn conversation
    let prompt = r#"
    1. First, check what files are in the current directory
    2. Create a new file called "test.txt" with "Hello, World!" content
    3. Read the file back to confirm it was created
    "#;
    
    let options = QueryOptions {
        system_prompt: Some("You are a file system assistant.".to_string()),
        max_turns: Some(5),
        allowed_tools: Some(vec![
            "Read".to_string(),
            "Write".to_string(),
            "LS".to_string()
        ]),
        permission_mode: None,
        resume_session_id: None,
        output_format: None,
    };
    
    let mut receiver = ClaudeManager::query_claude(
        client_id,
        prompt.to_string(),
        Some(options)
    ).await?;
    
    // Stream and process all messages
    println!("Streaming responses:");
    while let Some(message) = receiver.recv().await {
        match message {
            ClaudeMessage::Start { session_id } => {
                println!("[START] Session ID: {}", session_id);
            }
            ClaudeMessage::Progress { content } => {
                println!("[PROGRESS] {}", content);
            }
            ClaudeMessage::ToolUse { tool, args } => {
                println!("[TOOL] {} - Args: {}", tool, serde_json::to_string_pretty(&args)?);
            }
            ClaudeMessage::Result { result } => {
                println!("[RESULT] {}", result);
                break;
            }
            ClaudeMessage::Error { error } => {
                println!("[ERROR] {}", error);
                break;
            }
        }
    }
    
    Ok(())
}