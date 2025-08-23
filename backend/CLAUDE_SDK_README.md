# Claude Code SDK Integration for Rust Backend

This module provides a modular, reusable Rust backend for integrating with the Claude Code SDK, converting the CLI-based approach into a programmatic SDK interface.

## Architecture

The implementation consists of three main components:

### 1. ClaudeSDK - Core SDK Client
- Manages OAuth authentication tokens
- Executes queries using the JavaScript SDK via Bun runtime
- Streams responses back through Tokio channels
- Handles multiple message types (Start, Progress, ToolUse, Result, Error)

### 2. ClaudeSetup - Environment Setup
- Handles initial client setup and authentication
- Downloads and installs Bun runtime
- Installs @anthropic-ai/claude-code package
- Manages OAuth token authentication flow

### 3. ClaudeManager - Centralized Management
- Provides static methods for easy access
- Manages client and SDK instances
- Handles token updates and authentication state

## Key Features

### SDK Query Options
Matches the JavaScript SDK structure:
- `system_prompt`: Define the agent's role and behavior
- `max_turns`: Limit conversation turns
- `allowed_tools`: Specify permitted tools (Read, Write, Bash, etc.)
- `permission_mode`: Control modification capabilities
- `resume_session_id`: Continue previous conversations
- `output_format`: Request specific output formats (e.g., JSON)

### Message Types
- `Start`: Session initialization with ID
- `Progress`: Intermediate progress updates
- `ToolUse`: Tool invocation details
- `Result`: Final query result
- `Error`: Error messages

## Usage Examples

### Simple Query
```rust
use claude::{ClaudeManager};

let result = ClaudeManager::query_claude_simple(
    client_id,
    "What is the capital of France?".to_string()
).await?;
```

### Query with Options
```rust
use claude::{ClaudeManager, QueryOptions};

let options = QueryOptions {
    system_prompt: Some("You are a helpful coding assistant.".to_string()),
    max_turns: Some(3),
    allowed_tools: Some(vec!["Read".to_string(), "Write".to_string()]),
    ..Default::default()
};

let mut receiver = ClaudeManager::query_claude(
    client_id,
    "Help me write a Python script".to_string(),
    Some(options)
).await?;

while let Some(message) = receiver.recv().await {
    match message {
        ClaudeMessage::Result { result } => {
            println!("Result: {}", result);
            break;
        }
        ClaudeMessage::ToolUse { tool, args } => {
            println!("Tool used: {}", tool);
        }
        _ => continue,
    }
}
```

### Streaming Responses
```rust
let mut receiver = ClaudeManager::query_claude(
    client_id,
    prompt,
    options
).await?;

while let Some(message) = receiver.recv().await {
    match message {
        ClaudeMessage::Progress { content } => {
            // Stream progress to client
            send_to_client(content);
        }
        ClaudeMessage::Result { result } => {
            // Send final result
            send_final_result(result);
            break;
        }
        _ => continue,
    }
}
```

## Integration with Chat Handler

The chat handler in `handlers/chat.rs` demonstrates how to integrate the SDK:

1. Retrieves an authenticated client from the database
2. Builds conversation context from message history
3. Configures query options
4. Executes the query and processes streaming responses
5. Returns the final result with metadata

## Setup Requirements

1. **Environment Variables**:
   - `CLIENTS_DIR`: Directory for client installations (default: `../.clients`)
   - `HOME`: Home directory for Bun installation

2. **Database**:
   - Clients table with `id`, `claude_token`, and `install_path` columns
   - Active client with valid OAuth token

3. **Runtime Dependencies**:
   - Bun JavaScript runtime (automatically installed)
   - @anthropic-ai/claude-code npm package (automatically installed)

## API Endpoints

The SDK is exposed through the chat endpoint:
- `POST /api/chat` - Send messages and receive Claude responses

## Error Handling

The implementation includes comprehensive error handling:
- OAuth token validation
- Process spawn failures
- Timeout handling for long-running queries
- Graceful fallbacks for service unavailability

## Security Considerations

- OAuth tokens are stored securely in the database
- Each client has isolated environment
- Temporary script files are cleaned up after execution
- Process isolation prevents cross-client interference

## Future Enhancements

Potential improvements:
- WebSocket support for real-time streaming
- Session persistence and resumption
- Custom tool integration via MCP
- Response caching
- Rate limiting and quota management
- Multi-model support