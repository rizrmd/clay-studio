/// Get all available MCP tool names for Claude CLI allowed tools configuration
pub fn get_all_available_mcp_tools() -> Vec<String> {
    let mut tools = Vec::new();

    // Data analysis tools
    let data_analysis_tools = vec![
        "datasource_add",
        "datasource_list", 
        "datasource_remove",
        "datasource_update",
        "connection_test",
        "datasource_detail",
        "datasource_query",
        "datasource_inspect",
        "schema_get",
        "schema_search",
        "schema_related",
        "schema_stats",
    ];

    // Add data analysis tools with mcp prefix  
    for tool in data_analysis_tools {
        tools.push(format!("mcp__data-analysis__{}", tool));
    }

    // Interaction tools
    let interaction_tools = vec![
        "ask_user",
        "export_excel", 
        "show_table",
        "show_chart",
        "file_list",
        "file_read",
        "file_search",
        "file_metadata",
    ];

    // Add interaction tools with mcp prefix
    for tool in interaction_tools {
        tools.push(format!("mcp__interaction__{}", tool));
    }

    // Add standard web tools
    tools.extend(vec![
        "WebSearch".to_string(),
        "WebFetch".to_string(),
    ]);

    tools
}