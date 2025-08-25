pub mod types;
pub mod handlers;

use types::*;
use handlers::McpHandlers;
use sqlx::PgPool;
use std::io::{self, BufRead, BufReader, Write};
use tokio::runtime::Runtime;

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
        eprintln!("MCP Server starting for project: {}, client: {}", project_id, client_id);
        
        let runtime = Runtime::new()?;
        
        // Get database URL from environment
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL environment variable not set")?;
        
        // Create database connection pool
        let db_pool = runtime.block_on(async {
            PgPool::connect(&database_url).await
        })?;
        
        eprintln!("Connected to database successfully");
        
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
        eprintln!("MCP Server ready, waiting for JSON-RPC requests on stdin...");
        
        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());
        
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    
                    eprintln!("Received request: {}", line);
                    
                    // Parse and handle the request
                    let response = self.handle_request(line);
                    
                    // Send response
                    println!("{}", response);
                    io::stdout().flush().unwrap();
                    
                    eprintln!("Sent response: {}", response);
                }
                Err(e) => {
                    eprintln!("Error reading stdin: {}", e);
                    break;
                }
            }
        }
        
        eprintln!("MCP Server shutting down");
    }
    
    #[allow(dead_code)]
    fn handle_request(&mut self, line: String) -> String {
        // Parse JSON-RPC request
        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
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
        let result = self.runtime.block_on(async {
            match request.method.as_str() {
                "initialize" => {
                    self.handlers.handle_initialize(request.params).await
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
                    Err(JsonRpcError {
                        code: METHOD_NOT_FOUND,
                        message: format!("Method not found: {}", request.method),
                        data: None,
                    })
                }
            }
        });
        
        // Build response
        let response = match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(value),
                error: None,
            },
            Err(error) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(error),
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
            eprintln!("Failed to start MCP server: {}", e);
            std::process::exit(1);
        }
    }
}