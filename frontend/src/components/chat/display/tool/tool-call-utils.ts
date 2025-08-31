import {
  Wrench,
  Database,
  BarChart3,
  FileSearch,
  Code,
  Terminal,
  Globe,
  ListTodo,
  MessageSquare,
} from "lucide-react";

// Function to parse MCP tool names and convert to friendly display names
export function parseMcpToolName(toolName: string): {
  friendlyName: string;
  icon: React.ElementType;
  color: string;
  description: string;
  done?: string;
} {
  // Handle MCP tool format: mcp__server-name__tool-name
  if (toolName.startsWith("mcp__")) {
    const parts = toolName.split("__");
    if (parts.length >= 3) {
      // const serverName = parts[1]; // e.g., "data-analysis"
      const toolAction = parts[2]; // e.g., "datasource_list"

      // Convert tool action to friendly name
      const actionMappings: Record<
        string,
        {
          name: string;
          description: string;
          done?: string;
          icon: React.ElementType;
          color: string;
        }
      > = {
        // Data source management
        datasource_list: {
          name: "List Data Sources",
          description: "Listing data sources",
          done: "Data sources listed",
          icon: Database,
          color: "text-blue-600 bg-blue-50 border-blue-200",
        },
        datasource_detail: {
          name: "Data Source Details",
          description: "Getting datasource details",
          done: "Details retrieved",
          icon: FileSearch,
          color: "text-indigo-600 bg-indigo-50 border-indigo-200",
        },
        datasource_add: {
          name: "Add Data Source",
          description: "Adding data source",
          done: "Data source added",
          icon: Database,
          color: "text-green-600 bg-green-50 border-green-200",
        },
        datasource_remove: {
          name: "Remove Data Source",
          description: "Removing data source",
          done: "Data source removed",
          icon: Database,
          color: "text-red-600 bg-red-50 border-red-200",
        },
        datasource_test: {
          name: "Test Connection",
          description: "Testing connection",
          done: "Connection tested",
          icon: Database,
          color: "text-yellow-600 bg-yellow-50 border-yellow-200",
        },
        datasource_inspect: {
          name: "Inspect Datasource",
          description: "Inspecting datasource",
          done: "Datasource inspected",
          icon: FileSearch,
          color: "text-purple-600 bg-purple-50 border-purple-200",
        },
        
        // Data operations
        data_query: {
          name: "Query Data",
          description: "Querying data",
          done: "Query completed",
          icon: Database,
          color: "text-blue-600 bg-blue-50 border-blue-200",
        },
        data_analyze: {
          name: "Analyze Data",
          description: "Analyzing data",
          done: "Analysis completed",
          icon: BarChart3,
          color: "text-indigo-600 bg-indigo-50 border-indigo-200",
        },
        data_preview: {
          name: "Preview Data",
          description: "Previewing data",
          done: "Data previewed",
          icon: FileSearch,
          color: "text-cyan-600 bg-cyan-50 border-cyan-200",
        },
        
        // Schema operations
        schema_stats: {
          name: "Schema Statistics",
          description: "Getting schema statistics",
          done: "Statistics retrieved",
          icon: BarChart3,
          color: "text-purple-600 bg-purple-50 border-purple-200",
        },
        schema_search: {
          name: "Search Schema",
          description: "Searching schema",
          done: "Schema searched",
          icon: FileSearch,
          color: "text-teal-600 bg-teal-50 border-teal-200",
        },
        schema_get: {
          name: "Get Schema",
          description: "Getting table schema",
          done: "Schema retrieved",
          icon: Database,
          color: "text-orange-600 bg-orange-50 border-orange-200",
        },
        
        // Interaction tools
        ask_user: {
          name: "Interactive Element",
          description: "Creating interaction",
          done: "Interaction created",
          icon: MessageSquare,
          color: "text-violet-600 bg-violet-50 border-violet-200",
        },
        show_table: {
          name: "Display Table",
          description: "Displaying data table",
          done: "Table displayed",
          icon: Database,
          color: "text-emerald-600 bg-emerald-50 border-emerald-200",
        },
      };

      // Check if we have a mapping for this action
      if (actionMappings[toolAction]) {
        const mapping = actionMappings[toolAction];
        return {
          friendlyName: mapping.name,
          icon: mapping.icon,
          color: mapping.color,
          description: mapping.description,
          done: mapping.done,
        };
      }

      // Default mapping for unknown MCP tools
      const friendlyAction = toolAction
        .replace(/_/g, " ")
        .split(" ")
        .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
        .join(" ");

      return {
        friendlyName: friendlyAction,
        icon: Database,
        color: "text-blue-600 bg-blue-50 border-blue-200",
        description: `Running ${friendlyAction}`,
        done: `${friendlyAction} completed`,
      };
    }
  }

  // Handle Claude Code tools
  const claudeCodeTools: Record<
    string,
    {
      name: string;
      description: string;
      done?: string;
      icon: React.ElementType;
      color: string;
    }
  > = {
    run_command: {
      name: "Run Command",
      description: "Running command",
      icon: Terminal,
      color: "text-orange-600 bg-orange-50 border-orange-200",
    },
    read_file: {
      name: "Read File",
      description: "Reading file",
      icon: FileSearch,
      color: "text-emerald-600 bg-emerald-50 border-emerald-200",
    },
    write_file: {
      name: "Write File",
      description: "Writing file",
      icon: Code,
      color: "text-violet-600 bg-violet-50 border-violet-200",
    },
    search_files: {
      name: "Search Files",
      description: "Searching files",
      icon: FileSearch,
      color: "text-teal-600 bg-teal-50 border-teal-200",
    },
    web_search: {
      name: "Web Search",
      description: "Searching the web",
      icon: Globe,
      color: "text-sky-600 bg-sky-50 border-sky-200",
    },
    TodoWrite: {
      name: "Task List",
      description: "Managing tasks",
      done: "Tasks updated",
      icon: ListTodo,
      color: "text-purple-600 bg-purple-50 border-purple-200",
    },
  };

  if (claudeCodeTools[toolName]) {
    const tool = claudeCodeTools[toolName];
    return {
      friendlyName: tool.name,
      icon: tool.icon,
      color: tool.color,
      description: tool.description,
      done: tool.done,
    };
  }

  // Fallback for any other tool format
  return {
    friendlyName: toolName,
    icon: Wrench,
    color: "text-gray-600 bg-gray-50 border-gray-200",
    description: `Using ${toolName}`,
  };
}