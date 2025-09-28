/**
 * Maps tool IDs to friendly display names
 */
export function getFriendlyToolName(toolName: string): string {
  // Handle MCP tools
  if (toolName.startsWith("mcp__")) {
    const parts = toolName.split("__");
    if (parts.length >= 3) {
      const category = parts[1];
      const action = parts.slice(2).join("_");
      
      // Map common MCP operations
      const mappings: Record<string, string> = {
        "operation_schema_search": "Search Schema",
        "operation_schema_get": "Get Schema Details",
        "operation_datasource_query": "Query Database",
        "operation_datasource_list": "List Data Sources",
        "operation_datasource_get": "Get Data Source",
        "interaction_export_excel": "Export to Excel",
        "interaction_show_chart": "Show Chart",
        "interaction_show_table": "Show Table",
      };
      
      const key = `${category}_${action}`;
      return mappings[key] || formatToolName(action);
    }
  }
  
  // Handle other common tools
  const commonTools: Record<string, string> = {
    "TodoWrite": "Update Todo List",
    "WebSearch": "Search Web",
    "WebFetch": "Fetch Web Content",
    "Read": "Read File",
    "Write": "Write File",
    "Edit": "Edit File",
    "Bash": "Run Command",
  };
  
  return commonTools[toolName] || formatToolName(toolName);
}

/**
 * Formats a tool name by converting snake_case or camelCase to Title Case
 */
function formatToolName(name: string): string {
  // Convert snake_case to space-separated
  let formatted = name.replace(/_/g, " ");
  
  // Convert camelCase to space-separated
  formatted = formatted.replace(/([a-z])([A-Z])/g, "$1 $2");
  
  // Capitalize each word
  return formatted
    .split(" ")
    .map(word => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
    .join(" ");
}