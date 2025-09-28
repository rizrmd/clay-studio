pub mod handlers;
pub mod types;
pub mod response;

use chrono::Utc;
use handlers::McpHandlers;
use salvo::prelude::*;
use serde_json::json;
use sqlx::PgPool;
use std::io::{self, BufRead, BufReader, Write};
use tokio::runtime::Runtime;
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

        let db_pool = runtime.block_on(async { PgPool::connect(&database_url).await })?;

        let handlers = McpHandlers {
            project_id: project_id.clone(),
            client_id: client_id.clone(),
            server_type: "operation".to_string(),
            db_pool,
        };

        Ok(Self {
            project_id,
            client_id,
            server_type: "operation".to_string(),
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
        let _start_time = Utc::now();

        let runtime = Runtime::new()?;

        // Get database URL from environment
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL environment variable not set")?;

        let db_pool = runtime.block_on(async { PgPool::connect(&database_url).await })?;

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
        let line_iter = reader.lines();
        for line_result in line_iter {
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
    _project_id: String,
    _client_id: String,
    _server_type: String,
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

    let router = Router::new()
        .push(Router::with_path("/operation/{client_id}/{project_id}").post(handle_mcp_request).get(handle_sse_connection))
        .push(Router::with_path("/analysis/{client_id}/{project_id}").post(handle_mcp_request).get(handle_sse_connection))
        .push(Router::with_path("/interaction/{client_id}/{project_id}").post(handle_mcp_request).get(handle_sse_connection))
        .hoop(DbMiddleware { db_pool })
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

struct DbMiddleware {
    db_pool: PgPool,
}

#[async_trait::async_trait]
impl Handler for DbMiddleware {
    async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
        depot.insert("db_pool", self.db_pool.clone());
        ctrl.call_next(req, depot, res).await;
    }
}

struct CorsMiddleware;

#[async_trait::async_trait]
impl Handler for CorsMiddleware {
    async fn handle(&self, _req: &mut Request, _depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
        if let Ok(value) = "*".parse() {
            res.headers_mut().insert("Access-Control-Allow-Origin", value);
        }
        if let Ok(value) = "POST, GET, OPTIONS".parse() {
            res.headers_mut().insert("Access-Control-Allow-Methods", value);
        }
        if let Ok(value) = "Content-Type".parse() {
            res.headers_mut().insert("Access-Control-Allow-Headers", value);
        }
        ctrl.call_next(_req, _depot, res).await;
    }
}

#[handler]
async fn handle_mcp_request(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    let db_pool = match depot.get::<PgPool>("db_pool") {
        Ok(pool) => pool.clone(),
        Err(_) => {
            tracing::error!("Database pool not found in depot");
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(json!({
                "error": "Internal server error: Database pool not available"
            })));
            return;
        }
    };
    
    // Extract client_id and project_id from URL parameters
    let client_id = req.param::<String>("client_id").unwrap_or_else(|| "unknown".to_string());
    let project_id = req.param::<String>("project_id").unwrap_or_else(|| "default".to_string());
    
    // Determine server type from URL path
    let server_type = if req.uri().path().starts_with("/operation") {
        "operation".to_string()
    } else if req.uri().path().starts_with("/analysis") {
        "analysis".to_string()
    } else if req.uri().path().starts_with("/interaction") {
        "interaction".to_string()
    } else {
        "operation".to_string()
    };
    
    // Create handlers for this specific request
    let handlers = McpHandlers {
        project_id: project_id.clone(),
        client_id: client_id.clone(),
        server_type: server_type.clone(),
        db_pool,
    };
    
    eprintln!(
        "[{}] [INFO] Processing MCP request for server_type: {}, client_id: {}, project_id: {}",
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        server_type,
        client_id,
        project_id
    );
    
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

    let request_text = String::from_utf8_lossy(request_body);
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

    // Handle the request
    let result = match json_request.method.as_str() {
        "initialize" => handlers.handle_initialize(json_request.params).await,
        "notifications/initialized" => {
            eprintln!(
                "[{}] [INFO] Client initialization complete - MCP server fully ready",
                Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
            );
            Ok(serde_json::json!({}))
        }
        "resources/list" => handlers.handle_resources_list(json_request.params).await,
        "resources/read" => handlers.handle_resources_read(json_request.params).await,
        "tools/list" => handlers.handle_tools_list(json_request.params).await,
        "tools/call" => handlers.handle_tools_call(json_request.params).await,
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
async fn handle_sse_connection(req: &mut Request, _depot: &mut Depot, res: &mut Response) {
    use salvo::sse::{self as sse, SseEvent};
    use futures_util::stream;
    use std::convert::Infallible;
    
    // Extract client_id and project_id from URL parameters for logging
    let client_id = req.param::<String>("client_id").unwrap_or_else(|| "unknown".to_string());
    let project_id = req.param::<String>("project_id").unwrap_or_else(|| "default".to_string());
    
    // Determine server type from URL path
    let server_type = if req.uri().path().starts_with("/operation") {
        "operation"
    } else if req.uri().path().starts_with("/analysis") {
        "analysis"
    } else if req.uri().path().starts_with("/interaction") {
        "interaction"
    } else {
        "operation"
    };
    
    eprintln!(
        "[{}] [INFO] SSE connection established for server_type: {}, client_id: {}, project_id: {}",
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        server_type,
        client_id,
        project_id
    );
    
    let event_stream = stream::iter(vec![
        Ok::<_, Infallible>(
            SseEvent::default()
                .text(format!("MCP {} SSE connection established for client {} project {}", server_type, client_id, project_id))
                .name("connected")
        )
    ]);
    
    sse::stream(res, event_stream);
}
