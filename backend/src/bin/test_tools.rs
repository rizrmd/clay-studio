use clay_studio_backend::core::mcp::handlers::tools;

fn main() {
    let all_tools = tools::get_all_available_mcp_tools();
    
    println!("All MCP tools ({} total):", all_tools.len());
    for tool in &all_tools {
        println!("  {}", tool);
    }
    
    println!("\n=== Checking for show_table and show_chart ===");
    let has_show_table = all_tools.iter().any(|t| t == "mcp__interaction__show_table");
    let has_show_chart = all_tools.iter().any(|t| t == "mcp__interaction__show_chart");
    
    println!("mcp__interaction__show_table present: {}", has_show_table);
    println!("mcp__interaction__show_chart present: {}", has_show_chart);
}