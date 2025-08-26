pub mod types;
pub mod handlers;

use types::*;
use handlers::McpHandlers;
use sqlx::PgPool;
use std::io::{self, BufRead, BufReader, Write};
use tokio::runtime::Runtime;
use chrono::Utc;

pub struct McpServer {
    #[allow(dead_code)]
    project_id: String,
    #[allow(dead_code)]
    client_id: String,
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
        let db_pool = runtime.block_on(async {
            PgPool::connect(&database_url).await
        })?;
        
        eprintln!(
            "[{}] [INFO] Connected to database successfully", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
        
        let handlers = McpHandlers {
            project_id: project_id.clone(),
            client_id: client_id.clone(),
            db_pool,
        };
        
        Ok(Self {
            project_id,
            client_id,
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
        
        for line in reader.lines() {
            match line {
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
                    io::stdout().flush().unwrap();
                    
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
            },
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
                }).unwrap();
            }
        };
        
        // Route to appropriate handler
        eprintln!(
            "[{}] [DEBUG] Routing method: {}", 
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
            request.method
        );
        let method_start = std::time::Instant::now();
        let result = self.runtime.block_on(async {
            match request.method.as_str() {
                "initialize" => {
                    self.handlers.handle_initialize(request.params).await
                }
                "notifications/initialized" => {
                    // This is a notification from the client that initialization is complete
                    // We just acknowledge it and return an empty result
                    eprintln!(
                        "[{}] [INFO] Client initialization complete", 
                        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                    );
                    Ok(serde_json::json!({}))
                }
                "resources/list" => {
                    self.handlers.handle_resources_list(request.params).await
                }
                "resources/read" => {
                    self.handlers.handle_resources_read(request.params).await
                }
                "tools/list" => {
                    self.handlers.handle_tools_list(request.params).await
                }
                "tools/call" => {
                    self.handlers.handle_tools_call(request.params).await
                }
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
            },
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
            },
        };
        
        serde_json::to_string(&response).unwrap()
    }
}

// Public function to run the MCP server
#[allow(dead_code)]
pub fn run(project_id: String, client_id: String) {
    match McpServer::new(project_id, client_id) {
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