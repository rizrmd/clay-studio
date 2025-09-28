use clay_studio_backend::core::mcp::handlers::tools::get_all_available_mcp_tools;

fn main() {
    let tools = get_all_available_mcp_tools();
    println!("Total MCP tools: {}", tools.len());
    println!("\nAll registered MCP tools:");
    for tool in &tools {
        println!("  - {}", tool);
    }
    
    // Check for context_create
    let has_op_context = tools.iter().any(|t| t == "mcp__operation__context_create");
    let has_int_context = tools.iter().any(|t| t == "mcp__interaction__context_create");
    
    println!("\nmcp__operation__context_create exists: {}", has_op_context);
    println!("mcp__interaction__context_create exists: {}", has_int_context);
}