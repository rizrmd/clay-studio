pub mod handlers;
pub mod types;

use chrono::Utc;
use handlers::McpHandlers;
use salvo::prelude::*;
use sqlx::PgPool;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use types::*;

pub struct McpServer {
    #[allow(dead_code)]
    project_id: String,
    #[allow(dead_code)]
    client_id: String,
    #[allow(dead_code)]
    server_type: String,
    runtime: Runtime,
    handlers: McpHandlers,
}

impl McpServer {
    #[allow(dead_code)]
    pub fn new(project_id: String, client_id: String) -> Result<Self, Box<dyn std::error::Error>> {
        let start_time = Utc::now();
        eprintln!(
            "[{}] [INFO] MCP Server starting for project: {}, client: {}",
            start_time.format("%Y-%m-%d %H:%M:%S UTC"),
            project_id,
            client_id
        );

        let runtime = Runtime::new()?;

        // Get database URL from environment
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL environment variable not set")?;

        // Create database connection pool
        eprintln!(
            "[{}] [INFO] Connecting to database...",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
        let db_pool = runtime.block_on(async { PgPool::connect(&database_url).await })?;

        eprintln!(
            "[{}] [INFO] Connected to database successfully",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        let handlers = McpHandlers {
            project_id: project_id.clone(),
            client_id: client_id.clone(),
            server_type: "data-analysis".to_string(),
            db_pool,
        };

        Ok(Self {
            project_id,
            client_id,
            server_type: "data-analysis".to_string(),
            runtime,
            handlers,
        })
    }

    #[allow(dead_code)]
    pub fn new_with_type(
        project_id: String,
        client_id: String,
        server_type: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let start_time = Utc::now();
        eprintln!(
            "[{}] [INFO] MCP Server ({}) starting for project: {}, client: {}",
            start_time.format("%Y-%m-%d %H:%M:%S UTC"),
            server_type,
            project_id,
            client_id
        );

        let runtime = Runtime::new()?;

        // Get database URL from environment
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL environment variable not set")?;

        // Create database connection pool
        eprintln!(
            "[{}] [INFO] Connecting to database...",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
        let db_pool = runtime.block_on(async { PgPool::connect(&database_url).await })?;

        eprintln!(
            "[{}] [INFO] Connected to database successfully",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        let handlers = McpHandlers {
            project_id: project_id.clone(),
            client_id: client_id.clone(),
            server_type: server_type.clone(),
            db_pool,
        };

        Ok(Self {
            project_id,
            client_id,
            server_type,
            runtime,
            handlers,
        })
    }

    #[allow(dead_code)]
    pub fn run(&mut self) {
        eprintln!(
            "[{}] [INFO] MCP Server ready, waiting for JSON-RPC requests on stdin...",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());

        // Set up timeout to avoid hanging indefinitely
        let mut line_iter = reader.lines();
        while let Some(line_result) = line_iter.next() {
            match line_result {
                Ok(line) => {
                    if line.trim().is_empty() {
                        continue;
                    }

                    let request_time = Utc::now();
                    eprintln!(
                        "[{}] [REQUEST] Received: {}",
                        request_time.format("%Y-%m-%d %H:%M:%S UTC"),
                        line
                    );

                    // Parse and handle the request
                    let start_processing = std::time::Instant::now();
                    let response = self.handle_request(line);
                    let processing_duration = start_processing.elapsed();

                    // Send response
                    println!("{}", response);
                    if let Err(e) = io::stdout().flush() {
                        eprintln!(
                            "[{}] [ERROR] Failed to flush stdout: {}",
                            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                            e
                        );
                        break;
                    }

                    eprintln!(
                        "[{}] [RESPONSE] Sent (took {}ms): {}",
                        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                        processing_duration.as_millis(),
                        response
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[{}] [ERROR] Error reading stdin: {}",
                        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                        e
                    );
                    break;
                }
            }
        }

        eprintln!(
            "[{}] [INFO] MCP Server shutting down",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
    }

    #[allow(dead_code)]
    fn handle_request(&mut self, line: String) -> String {
        // Parse JSON-RPC request
        let request: JsonRpcRequest = match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(req) => {
                eprintln!(
                    "[{}] [DEBUG] Parsed request - method: {}, id: {:?}",
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                    req.method,
                    req.id
                );
                req
            }
            Err(e) => {
                eprintln!(
                    "[{}] [ERROR] JSON-RPC parse error: {}",
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                    e
                );
                return serde_json::to_string(&JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                }).unwrap_or_else(|e| {
                    eprintln!(
                        "[{}] [ERROR] Failed to serialize error response: {}", 
                        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                        e
                    );
                    r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error and serialization failed"}}"#.to_string()
                });
            }
        };

        // Route to appropriate handler
        eprintln!(
            "[{}] [DEBUG] Routing method: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            request.method
        );
        let method_start = std::time::Instant::now();

        // Clone request_id to avoid borrow issues
        let request_id = request.id.clone();

        // Execute the method with async error handling (no panic catching for async operations)
        let result = self.runtime.block_on(async {
            match request.method.as_str() {
                "initialize" => self.handlers.handle_initialize(request.params).await,
                "notifications/initialized" => {
                    // This is a notification from the client that initialization is complete
                    // We just acknowledge it and return an empty result
                    eprintln!(
                        "[{}] [INFO] Client initialization complete - MCP server fully ready",
                        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                    );
                    Ok(serde_json::json!({}))
                }
                "resources/list" => self.handlers.handle_resources_list(request.params).await,
                "resources/read" => self.handlers.handle_resources_read(request.params).await,
                "tools/list" => self.handlers.handle_tools_list(request.params).await,
                "tools/call" => self.handlers.handle_tools_call(request.params).await,
                _ => {
                    eprintln!(
                        "[{}] [ERROR] Method not found: {}",
                        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                        request.method
                    );
                    Err(JsonRpcError {
                        code: METHOD_NOT_FOUND,
                        message: format!("Method not found: {}", request.method),
                        data: None,
                    })
                }
            }
        });
        let method_duration = method_start.elapsed();

        // Build response
        let response = match result {
            Ok(value) => {
                eprintln!(
                    "[{}] [DEBUG] Method {} completed successfully in {}ms",
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                    request.method,
                    method_duration.as_millis()
                );
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(value),
                    error: None,
                }
            }
            Err(error) => {
                eprintln!(
                    "[{}] [ERROR] Method {} failed in {}ms: {} (code: {})",
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                    request.method,
                    method_duration.as_millis(),
                    error.message,
                    error.code
                );
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(error),
                }
            }
        };

        serde_json::to_string(&response).unwrap_or_else(|e| {
            eprintln!(
                "[{}] [ERROR] Failed to serialize response: {}", 
                Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                e
            );
            format!(
                r#"{{"jsonrpc":"2.0","id":{},"error":{{"code":-32603,"message":"Failed to serialize response: {}"}}}}"#,
                request_id.map(|id| id.to_string()).unwrap_or_else(|| "null".to_string()),
                e
            )
        })
    }
}

// Public function to run the MCP server with specific type
#[allow(dead_code)]
pub fn run_with_type(project_id: String, client_id: String, server_type: String) {
    match McpServer::new_with_type(project_id, client_id, server_type) {
        Ok(mut server) => {
            server.run();
        }
        Err(e) => {
            eprintln!(
                "[{}] [FATAL] Failed to start MCP server: {}",
                Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                e
            );
            std::process::exit(1);
        }
    }
}

// Public function to run the MCP server with HTTP transport
#[allow(dead_code)]
pub fn run_with_http(project_id: String, client_id: String, server_type: String, port: u16) {
    let runtime = Runtime::new().unwrap_or_else(|e| {
        eprintln!(
            "[{}] [FATAL] Failed to create Tokio runtime: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            e
        );
        std::process::exit(1);
    });

    runtime.block_on(async {
        if let Err(e) = run_http_server(project_id, client_id, server_type, port).await {
            eprintln!(
                "[{}] [FATAL] HTTP MCP server failed: {}",
                Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                e
            );
            std::process::exit(1);
        }
    });
}

async fn run_http_server(
    project_id: String,
    client_id: String,
    server_type: String,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create database connection pool
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL environment variable not set")?;
    
    eprintln!(
        "[{}] [INFO] Connecting to database...",
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
    let db_pool = PgPool::connect(&database_url).await?;
    
    eprintln!(
        "[{}] [INFO] Connected to database successfully",
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );

    let handlers = Arc::new(Mutex::new(McpHandlers {
        project_id: project_id.clone(),
        client_id: client_id.clone(),
        server_type: server_type.clone(),
        db_pool,
    }));

    let router = Router::new()
        .push(Router::with_path("/mcp").post(handle_mcp_request))
        .push(Router::with_path("/mcp/sse").get(handle_sse_connection))
        .hoop(McpHandlerMiddleware { handlers: handlers.clone() })
        .hoop(CorsMiddleware);

    let acceptor = TcpListener::new(format!("0.0.0.0:{}", port)).bind().await;
    
    eprintln!(
        "[{}] [INFO] MCP HTTP server listening on port {}",
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        port
    );
    
    Server::new(acceptor).serve(router).await;
    
    Ok(())
}

struct McpHandlerMiddleware {
    handlers: Arc<Mutex<McpHandlers>>,
}

#[async_trait::async_trait]
impl Handler for McpHandlerMiddleware {
    async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
        depot.insert("mcp_handlers", self.handlers.clone());
        ctrl.call_next(req, depot, res).await;
    }
}

struct CorsMiddleware;

#[async_trait::async_trait]
impl Handler for CorsMiddleware {
    async fn handle(&self, _req: &mut Request, _depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
        res.headers_mut().insert("Access-Control-Allow-Origin", "*".parse().unwrap());
        res.headers_mut().insert("Access-Control-Allow-Methods", "POST, GET, OPTIONS".parse().unwrap());
        res.headers_mut().insert("Access-Control-Allow-Headers", "Content-Type".parse().unwrap());
        ctrl.call_next(_req, _depot, res).await;
    }
}

#[handler]
async fn handle_mcp_request(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    let handlers = depot.get::<Arc<Mutex<McpHandlers>>>("mcp_handlers").unwrap();
    
    // Parse JSON-RPC request
    let request_body = match req.payload().await {
        Ok(body) => body,
        Err(e) => {
            let error_response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: Some(JsonRpcError {
                    code: -32700,
                    message: format!("Failed to read request body: {}", e),
                    data: None,
                }),
            };
            res.render(Json(error_response));
            return;
        }
    };

    let request_text = String::from_utf8_lossy(&request_body);
    let json_request: JsonRpcRequest = match serde_json::from_str(&request_text) {
        Ok(req) => req,
        Err(e) => {
            let error_response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: Some(JsonRpcError {
                    code: -32700,
                    message: format!("Parse error: {}", e),
                    data: None,
                }),
            };
            res.render(Json(error_response));
            return;
        }
    };

    // Handle the request using the same logic as stdio
    let handlers_guard = handlers.lock().await;
    let result = match json_request.method.as_str() {
        "initialize" => handlers_guard.handle_initialize(json_request.params).await,
        "notifications/initialized" => {
            eprintln!(
                "[{}] [INFO] Client initialization complete - MCP server fully ready",
                Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
            );
            Ok(serde_json::json!({}))
        }
        "resources/list" => handlers_guard.handle_resources_list(json_request.params).await,
        "resources/read" => handlers_guard.handle_resources_read(json_request.params).await,
        "tools/list" => handlers_guard.handle_tools_list(json_request.params).await,
        "tools/call" => handlers_guard.handle_tools_call(json_request.params).await,
        _ => Err(JsonRpcError {
            code: METHOD_NOT_FOUND,
            message: format!("Method not found: {}", json_request.method),
            data: None,
        }),
    };

    let response = match result {
        Ok(value) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json_request.id,
            result: Some(value),
            error: None,
        },
        Err(error) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json_request.id,
            result: None,
            error: Some(error),
        },
    };

    res.render(Json(response));
}

#[handler]
async fn handle_sse_connection(_req: &mut Request, _depot: &mut Depot, res: &mut Response) {
    use salvo::sse::{self as sse, SseEvent};
    use futures_util::stream;
    use std::convert::Infallible;
    
    let event_stream = stream::iter(vec![
        Ok::<_, Infallible>(
            SseEvent::default()
                .text("MCP SSE connection established")
                .name("connected")
        )
    ]);
    
    sse::stream(res, event_stream);
}
