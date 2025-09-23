/// Get all available MCP tool names for Claude CLI allowed tools configuration
pub fn get_all_available_mcp_tools() -> Vec<String> {
    let mut tools = Vec::new();

    // Data analysis tools
    let data_analysis_tools = vec![
        // Datasource tools
        "datasource_add",
        "datasource_list", 
        "datasource_remove",
        "datasource_update",
        "connection_test",
        "datasource_detail",
        "datasource_query",
        "datasource_inspect",
        // Schema tools
        "schema_get",
        "schema_search",
        "schema_related",
        "schema_stats",
        // Analysis tools
        "analysis_create",
        "analysis_list",
        "analysis_get",
        "analysis_update",
        "analysis_delete",
        "analysis_run",
        "analysis_validate",
        // Job management tools
        "job_list",
        "job_get",
        "job_cancel",
        "job_result",
        // Schedule management tools
        "schedule_create",
        "schedule_list",
        "schedule_update",
        "schedule_delete",
        // Monitoring tools
        "monitoring_status",
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