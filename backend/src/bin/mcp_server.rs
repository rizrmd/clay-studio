// Entry point for the MCP server binary
// This creates a separate executable that Claude CLI will spawn

use std::io::Write;


fn main() {
    // Set up global panic handler to prevent -32000 errors
    std::panic::set_hook(Box::new(|panic_info| {
        let panic_msg = if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else {
            "Unknown panic occurred".to_string()
        };
        
        let location = if let Some(location) = panic_info.location() {
            format!(" at {}:{}:{}", location.file(), location.line(), location.column())
        } else {
            " at unknown location".to_string()
        };
        
        eprintln!(
            "[{}] [FATAL] Panic occurred{}: {}", 
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            location,
            panic_msg
        );
        
        // Try to send a proper JSON-RPC error response before exiting
        let error_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": null,
            "error": {
                "code": -32603,
                "message": format!("Internal server error: {}", panic_msg),
                "data": format!("Panic occurred{}", location)
            }
        });
        
        if let Ok(error_json) = serde_json::to_string(&error_response) {
            println!("{}", error_json);
            let _ = std::io::stdout().flush();
        }
        
        std::process::exit(1);
    }));
    
    // Load environment variables from backend/.env file if present
    let backend_env_path = std::env::current_exe()
        .ok()
        .and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
        .and_then(|target_path| {
            // Navigate from target/debug to backend directory
            target_path.parent()
                .and_then(|p| p.parent())  // Go up from target/debug to backend
                .map(|p| p.join(".env"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("backend/.env"));
    
    let _ = dotenv::from_path(&backend_env_path);
    
    // Set up basic logging to stderr (so it doesn't interfere with stdout JSON-RPC)
    let start_time = chrono::Utc::now();
    eprintln!(
        "[{}] [INFO] MCP Server v0.1.0 starting...", 
        start_time.format("%Y-%m-%d %H:%M:%S UTC")
    );
    
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut project_id = String::from("default");
    let mut client_id = String::from("unknown");
    
    // Simple argument parsing
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--project-id" => {
                if i + 1 < args.len() {
                    project_id = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!(
                        "[{}] [ERROR] --project-id requires a value", 
                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                    );
                    std::process::exit(1);
                }
            }
            "--client-id" => {
                if i + 1 < args.len() {
                    client_id = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!(
                        "[{}] [ERROR] --client-id requires a value", 
                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                    );
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {
                eprintln!(
                    "[{}] [ERROR] Unknown argument: {}", 
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                    args[i]
                );
                print_help();
                std::process::exit(1);
            }
        }
    }
    
    // Also check environment variables as fallback
    if let Ok(env_project_id) = std::env::var("PROJECT_ID") {
        if project_id == "default" {
            project_id = env_project_id;
        }
    }
    
    if let Ok(env_client_id) = std::env::var("CLIENT_ID") {
        if client_id == "unknown" {
            client_id = env_client_id;
        }
    }
    
    eprintln!(
        "[{}] [INFO] Configuration:", 
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );
    eprintln!("  Project ID: {}", project_id);
    eprintln!("  Client ID: {}", client_id);
    
    // Check for required DATABASE_URL
    if std::env::var("DATABASE_URL").is_err() {
        eprintln!(
            "[{}] [FATAL] DATABASE_URL environment variable is required", 
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
        eprintln!("Please set DATABASE_URL to your PostgreSQL connection string");
        std::process::exit(1);
    }
    
    // Run the MCP server
    // Note: We need to access the mcp module from the library
    // This will be compiled as part of the same crate
    clay_studio_backend::core::mcp::run(project_id, client_id);
}

fn print_help() {
    eprintln!("Clay Studio MCP Server");
    eprintln!();
    eprintln!("Usage: mcp_server [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --project-id <ID>   Project ID to serve data for");
    eprintln!("  --client-id <ID>    Client ID for authentication");
    eprintln!("  -h, --help          Show this help message");
    eprintln!();
    eprintln!("Environment variables:");
    eprintln!("  DATABASE_URL        PostgreSQL connection string (required)");
    eprintln!("  PROJECT_ID          Project ID (used if --project-id not provided)");
    eprintln!("  CLIENT_ID           Client ID (used if --client-id not provided)");
    eprintln!();
    eprintln!("This server implements the Model Context Protocol (MCP) for Claude.");
    eprintln!("It provides access to data sources stored in PostgreSQL.");
}