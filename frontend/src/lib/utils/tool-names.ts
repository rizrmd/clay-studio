/**
 * Maps tool IDs to friendly display names
 */
export function getFriendlyToolName(toolName: string): string {
  // Handle MCP tools
  if (toolName.startsWith("mcp__")) {
    const parts = toolName.split("__");
    if (parts.length >= 3) {
      // For mcp__operation__datasource_create: category=operation, subcategory=datasource, action=create
      // For mcp__analysis__create: category=analysis, action=create
      const category = parts[1];
      const rest = parts.slice(2);

      // Map common MCP operations
      const mappings: Record<string, string> = {
        // operation__schema__* tools
        "operation__schema__search": "Search Schema",
        "operation__schema__get": "Get Schema",
        // operation__datasource__* tools
        "operation__datasource__query": "Query Datasource",
        "operation__datasource__list": "List Datasources",
        "operation__datasource__get": "Get Datasource",
        "operation__datasource__create": "Create Datasource",
        // interaction__* tools
        "interaction__export_excel": "Export to Excel",
        "interaction__show_chart": "Show Chart",
        "interaction__show_table": "Show Table",
        // analysis__* tools (no operation prefix)
        "analysis__create": "Create Analysis",
        "analysis__list": "List Analyses",
        "analysis__run": "Run Analysis",
        "analysis__show_chart": "Show Chart",
        "analysis__show_table": "Show Table",
      };

      const key = `${category}__${rest.join("__")}`;
      return mappings[key] || formatToolName(rest.join("_"));
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