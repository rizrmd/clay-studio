# MCP Server Configuration

The Clay Studio project includes an MCP (Model Context Protocol) server that provides database access and analysis tools.

## Development Configuration

For development, the MCP server is configured to use the debug binary:
- Path: `/Users/riz/Developer/clay-studio/backend/target/debug/mcp_server`
- Built automatically by `bun dev`
- Includes enhanced logging with timestamps and detailed debugging information

## Production Configuration

For production, update the MCP configuration to use the release binary:
- Path: `/Users/riz/Developer/clay-studio/backend/target/release/mcp_server`
- Built by `bun build`
- Optimized for performance

## MCP Configuration File

The configuration is stored in:
```
.clients/{client-id}/{project-id}/.claude/mcp_servers.json
```

## Logging Features

The MCP server includes comprehensive logging:
- **Structured timestamps**: `[YYYY-MM-DD HH:MM:SS UTC]`
- **Log levels**: INFO, DEBUG, ERROR, WARNING, FATAL, REQUEST, RESPONSE
- **Performance metrics**: Request/response timing, database query duration
- **Database operations**: Connection status, query execution, tool calls
- **Error tracking**: Detailed error messages with context

## Example Log Output

```
[2025-08-26 09:45:28 UTC] [INFO] MCP Server v0.1.0 starting...
[2025-08-26 09:45:28 UTC] [INFO] Configuration:
  Project ID: 202579e9-3da8-4c45-8d45-9003bebd4a30
  Client ID: 77d5ecd4-6f00-44c5-9d35-a38ddfdbb9c5
[2025-08-26 09:45:28 UTC] [INFO] Connected to database successfully
[2025-08-26 09:45:28 UTC] [REQUEST] Received: {"jsonrpc": "2.0", "method": "initialize"...}
[2025-08-26 09:45:28 UTC] [RESPONSE] Sent (took 15ms): {"jsonrpc": "2.0", "result": {...}}
```

## Available Tools

The MCP server provides these database analysis tools:
- `datasource_list` - List all data sources
- `datasource_inspect` - Analyze database structure
- `datasource_add` - Add new data sources
- `datasource_test` - Test database connections
- `schema_get` - Get table schemas
- `schema_search` - Search for tables by pattern
- `data_query` - Execute SELECT queries

## Troubleshooting

1. **Connection Issues**: Check that the database URL is correct and accessible
2. **Binary Not Found**: Ensure the MCP server binary is built (`bun dev` or `bun build`)
3. **Permission Errors**: Verify the binary has execute permissions
4. **Database Errors**: Check the logs for detailed database connection issues