use clay_studio_backend::core::mcp::handlers::tools;

fn main() {
    // Get all MCP tools
    let all_tools = tools::get_all_available_mcp_tools();
    
    println!("Total MCP tools registered: {}", all_tools.len());
    
    // Check for show_table and show_chart
    let has_show_table = all_tools.iter().any(|t| t.contains("show_table"));
    let has_show_chart = all_tools.iter().any(|t| t.contains("show_chart"));
    
    println!("\nInteraction tools:");
    for tool in &all_tools {
        if tool.starts_with("mcp__interaction__") {
            println!("  - {}", tool);
        }
    }
    
    println!("\nshow_table found: {}", has_show_table);
    println!("show_chart found: {}", has_show_chart);
    
    // Get interaction tools directly
    println!("\n\nDirect from get_interaction_tools():");
    let interaction_tools = tools::interaction::get_interaction_tools();
    for tool in &interaction_tools {
        println!("  - {}", tool.name);
    }
}