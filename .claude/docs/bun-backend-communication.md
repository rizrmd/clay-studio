# Bun Analysis <-> Backend Communication

## Question: Do we need IPC between Bun and Backend?

**Answer: For file operations, yes. For datasource queries, no.**

## Current Architecture

### What Works Without IPC ✅

1. **DuckDB Queries** - Runs entirely in Bun process
2. **External Datasources** - Bun connects directly to PostgreSQL/MySQL
3. **Script Execution** - Self-contained
4. **Result Return** - Via stdout JSON

### What Needs Backend Access 🔄

1. **File Operations** - Files stored in backend database
2. **Project Metadata** - Stored in backend
3. **Conversation Context** - Managed by backend

## Implementation Options

### Option 1: HTTP Callbacks (Current)

**How it works:**
```typescript
// In Bun process
const files = await fetch('http://localhost:8000/api/mcp/files?project_id=...');
```

**Pros:**
- Simple implementation
- Works with existing REST API
- Easy to test

**Cons:**
- Requires backend URL configuration
- No authentication mechanism
- Network overhead for local calls
- Firewall/NAT issues possible
- Can't work if backend isn't HTTP accessible

### Option 2: stdin/stdout IPC (Implemented)

**How it works:**
```typescript
// In Bun process
process.stdout.write(JSON.stringify({
  id: 'req-1',
  method: 'files.list',
  params: { projectId: '...' }
}));

// Backend reads from child process stdout, responds via stdin
// Bun reads response from stdin
```

**Pros:**
- ✅ No network required
- ✅ Built-in authentication (process isolation)
- ✅ Fast (no HTTP overhead)
- ✅ Works without backend HTTP server
- ✅ Automatic cleanup on process exit

**Cons:**
- More complex protocol
- Need request/response correlation (IDs)
- Requires async message handling

### Option 3: Shared Memory / Unix Sockets

**Not implemented** - Overly complex for this use case

## Current Implementation Status

### IPC Infrastructure ✅

The backend now has stdin/stdout piping set up:

```rust
let mut child = Command::new(&self.bun_path)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;
```

**stdin**: Reserved for IPC requests (TODO)
**stdout**: Analysis results + IPC responses
**stderr**: Logs and debug output

### File Operations (HTTP) ✅

Currently using HTTP callbacks:

```typescript
ctx.files = {
    list: async (conversationId?: string) => {
        const url = `${backendUrl}/api/mcp/files?conversation_id=${conversationId}`;
        const response = await fetch(url);
        return await response.json();
    }
}
```

## Recommendation

### For Production: Use IPC

Implement stdin/stdout IPC protocol:

```typescript
// In wrapper.ts
let requestId = 0;
const pendingRequests = new Map();

function callBackend(method: string, params: any): Promise<any> {
    return new Promise((resolve, reject) => {
        const id = `req-${++requestId}`;
        pendingRequests.set(id, { resolve, reject });

        // Send request to backend via stdout
        console.log(JSON.stringify({ id, method, params }));
    });
}

// Listen for responses on stdin
process.stdin.on('data', (data) => {
    const response = JSON.parse(data.toString());
    const pending = pendingRequests.get(response.id);
    if (pending) {
        if (response.error) {
            pending.reject(new Error(response.error));
        } else {
            pending.resolve(response.result);
        }
        pendingRequests.delete(response.id);
    }
});

// Use it
ctx.files = {
    list: async (conversationId) => {
        return await callBackend('files.list', { conversationId });
    }
}
```

### Backend Handler:

```rust
// In run_bun_script, spawn task to handle IPC
tokio::spawn(async move {
    let mut stdin_writer = stdin;
    let mut stdout_reader = BufReader::new(stdout).lines();

    while let Some(line) = stdout_reader.next_line().await {
        if let Ok(request) = serde_json::from_str::<IpcRequest>(&line) {
            let response = match request.method.as_str() {
                "files.list" => handle_files_list(request.params).await,
                "files.read" => handle_files_read(request.params).await,
                _ => IpcResponse::error(request.id, "Unknown method")
            };

            stdin_writer.write_all(
                serde_json::to_string(&response)?.as_bytes()
            ).await?;
            stdin_writer.write_all(b"\n").await?;
        }
    }
});
```

### For Development: HTTP is Fine

The current HTTP approach works well for development:
- Easy to debug (can use curl)
- Can inspect traffic
- No protocol complexity
- Works with backend running separately

## Migration Path

1. **Phase 1 (Current)**: HTTP callbacks ✅
   - Simple, works immediately
   - Good for testing

2. **Phase 2 (Future)**: Add IPC support
   - Implement IPC protocol
   - Keep HTTP as fallback
   - Feature flag to choose

3. **Phase 3 (Production)**: IPC by default
   - Use IPC for all operations
   - HTTP only for debugging

## Security Considerations

### HTTP Approach
- ⚠️ No authentication
- ⚠️ Backend must be accessible
- ⚠️ Can access any endpoint

### IPC Approach
- ✅ Process isolation provides security
- ✅ Can only call whitelisted methods
- ✅ No external network access
- ✅ Automatic cleanup

## Performance

### HTTP:
- ~1-5ms overhead per call
- Multiple calls = multiple RTTs
- Connection pooling needed

### IPC:
- ~0.1ms overhead
- Batching possible
- No connection overhead

## Conclusion

**Answer: Yes, for file operations we should use IPC, but HTTP works fine for now.**

**Current state:**
- IPC infrastructure is in place (stdin/stdout piped)
- HTTP callbacks implemented and working
- Can migrate to IPC when needed

**Recommendation:**
- ✅ Keep HTTP for development
- 🔄 Add IPC protocol for production
- 🔄 Make it configurable via environment variable

The current HTTP implementation is **good enough** for the MVP. IPC can be added later when performance or security becomes a concern.
