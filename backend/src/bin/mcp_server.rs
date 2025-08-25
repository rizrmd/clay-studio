// Entry point for the MCP server binary
// This creates a separate executable that Claude CLI will spawn

fn main() {
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
    
    if let Err(e) = dotenv::from_path(&backend_env_path) {
        // Try current directory as fallback
        if let Err(e2) = dotenv::dotenv() {
            eprintln!("Note: .env file not found at {:?} or current directory: {} / {}", backend_env_path, e, e2);
        }
    }
    
    // Set up basic logging to stderr (so it doesn't interfere with stdout JSON-RPC)
    eprintln!("MCP Server v0.1.0 starting...");
    
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
                    eprintln!("Error: --project-id requires a value");
                    std::process::exit(1);
                }
            }
            "--client-id" => {
                if i + 1 < args.len() {
                    client_id = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --client-id requires a value");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
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
    
    eprintln!("Configuration:");
    eprintln!("  Project ID: {}", project_id);
    eprintln!("  Client ID: {}", client_id);
    
    // Check for required DATABASE_URL
    if std::env::var("DATABASE_URL").is_err() {
        eprintln!("Error: DATABASE_URL environment variable is required");
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