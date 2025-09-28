use super::analysis::get_analysis_tools;
use super::interaction::get_interaction_tools;
use super::operation_impl::get_operation_tools;

/// Get all available MCP tool names for Claude CLI allowed tools configuration
pub fn get_all_available_mcp_tools() -> Vec<String> {
    let mut tools = Vec::new();
    
    // Get all analysis tools and prefix them
    for tool in get_analysis_tools() {
        tools.push(format!("mcp__analysis__{}", tool.name));
    }
    
    // Get all operation tools and prefix them
    for tool in get_operation_tools() {
        tools.push(format!("mcp__operation__{}", tool.name));
    }
    
    // Get all interaction tools and prefix them
    for tool in get_interaction_tools() {
        tools.push(format!("mcp__interaction__{}", tool.name));
    }
    
    tools
}