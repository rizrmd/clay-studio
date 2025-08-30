use std::collections::HashMap;

/// MCP Tool Registry - A centralized registry for all MCP tools
/// This allows easy addition/modification of tools without hardcoding
#[derive(Clone)]
#[allow(dead_code)]
pub struct McpTool {
    pub name: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    /// Keywords that appear in the tool result to identify this tool
    pub result_indicators: Vec<&'static str>,
}

/// Get the global registry of all MCP tools
pub fn get_mcp_tools() -> HashMap<String, McpTool> {
    let mut tools = HashMap::new();
    
    // Data Analysis Tools
    tools.insert("mcp__data-analysis__datasource_list".to_string(), McpTool {
        name: "datasource_list",
        display_name: "List Data Sources",
        description: "Lists available data sources",
        result_indicators: vec!["Data sources", "Available data sources"],
    });
    
    tools.insert("mcp__data-analysis__datasource_detail".to_string(), McpTool {
        name: "datasource_detail",
        display_name: "Data Source Details",
        description: "Shows detailed information about a data source",
        result_indicators: vec!["Data Source Details", "Connection Configuration:", "Status:", "Type:"],
    });
    
    tools.insert("mcp__data-analysis__datasource_add".to_string(), McpTool {
        name: "datasource_add",
        display_name: "Add Data Source",
        description: "Adds a new data source",
        result_indicators: vec!["Data source", "added successfully", "Successfully added"],
    });
    
    tools.insert("mcp__data-analysis__datasource_remove".to_string(), McpTool {
        name: "datasource_remove",
        display_name: "Remove Data Source",
        description: "Removes a data source",
        result_indicators: vec!["Data source", "removed successfully", "Successfully removed"],
    });
    
    tools.insert("mcp__data-analysis__datasource_update".to_string(), McpTool {
        name: "datasource_update",
        display_name: "Update Data Source",
        description: "Updates an existing data source configuration",
        result_indicators: vec!["Data source", "updated successfully", "Successfully updated", "Configuration updated"],
    });
    
    tools.insert("mcp__data-analysis__datasource_test".to_string(), McpTool {
        name: "datasource_test",
        display_name: "Test Connection",
        description: "Tests database connection",
        result_indicators: vec!["Connection successful", "Connection test", "Successfully connected"],
    });
    
    tools.insert("mcp__data-analysis__datasource_inspect".to_string(), McpTool {
        name: "datasource_inspect",
        display_name: "Inspect Data Source",
        description: "Inspects database structure",
        result_indicators: vec!["Database compatibility", "MCP Server Error", "Database inspection"],
    });
    
    tools.insert("mcp__data-analysis__data_query".to_string(), McpTool {
        name: "data_query",
        display_name: "Query Data",
        description: "Executes a data query",
        result_indicators: vec!["Query executed on", "Query results", "Rows returned"],
    });
    
    tools.insert("mcp__data-analysis__schema_stats".to_string(), McpTool {
        name: "schema_stats",
        display_name: "Schema Statistics",
        description: "Shows database statistics",
        result_indicators: vec!["Database statistics", "Schema statistics", "Table statistics"],
    });
    
    tools.insert("mcp__data-analysis__schema_search".to_string(), McpTool {
        name: "schema_search",
        display_name: "Search Schema",
        description: "Searches database schema",
        result_indicators: vec!["Tables matching", "Schema for table", "Found tables", "Matching tables"],
    });
    
    tools.insert("mcp__data-analysis__schema_get".to_string(), McpTool {
        name: "schema_get",
        display_name: "Get Schema",
        description: "Gets table schema",
        result_indicators: vec!["columns matching", "Table schema", "Column details"],
    });
    
    // Interaction Tools
    tools.insert("mcp__interaction__ask_user".to_string(), McpTool {
        name: "ask_user",
        display_name: "Interactive Element",
        description: "Creates interactive elements like charts, tables, and user prompts",
        result_indicators: vec!["interaction_type", "Interactive element created", "User interaction"],
    });
    
    // Add more MCP tools here as needed
    // Example for a new tool:
    // tools.insert("mcp__new-server__tool_name".to_string(), McpTool {
    //     name: "tool_name",
    //     display_name: "Tool Display Name",
    //     description: "Tool description",
    //     result_indicators: vec!["keyword1", "keyword2"],
    // });
    
    tools
}

/// Attempts to identify a tool from its result content
pub fn identify_tool_from_result(content: &str) -> Option<String> {
    let tools = get_mcp_tools();
    for (tool_id, tool_info) in tools.iter() {
        // Check if any of the tool's indicators appear in the content
        for indicator in &tool_info.result_indicators {
            if content.contains(indicator) {
                return Some(tool_id.clone());
            }
        }
    }
    None
}

/// Gets tool information by its full ID (e.g., "mcp__data-analysis__datasource_list")
#[allow(dead_code)]
pub fn get_tool_info(tool_id: &str) -> Option<McpTool> {
    let tools = get_mcp_tools();
    tools.get(tool_id).cloned()
}

/// Checks if a string is a tool_use_id (starts with "toolu_")
pub fn is_tool_use_id(s: &str) -> bool {
    s.starts_with("toolu_")
}