import {
  Wrench,
  Loader2,
  Database,
  BarChart3,
  FileSearch,
  Code,
  Terminal,
  Globe,
  CheckCircle,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { css } from "goober";

// Function to parse MCP tool names and convert to friendly display names
function parseMcpToolName(toolName: string): {
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
      const serverName = parts[1]; // e.g., "data-analysis"
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
        datasource_list: {
          name: "List Data Sources",
          description: "Listing data sources",
          done: "Data sources listed",
          icon: Database,
          color: "text-blue-600 bg-blue-50 border-blue-200",
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
        data_query: {
          name: "Execute SQL Query",
          description: "Running SQL query",
          done: "Query executed",
          icon: Terminal,
          color: "text-blue-600 bg-blue-50 border-blue-200",
        },
        schema_get: {
          name: "Get Table Schema",
          description: "Getting table schema",
          done: "Schema retrieved",
          icon: FileSearch,
          color: "text-indigo-600 bg-indigo-50 border-indigo-200",
        },
        schema_search: {
          name: "Search Schema",
          description: "Searching database schema",
          done: "Schema search completed",
          icon: FileSearch,
          color: "text-yellow-600 bg-yellow-50 border-yellow-200",
        },
        schema_get_related: {
          name: "Find Related Tables",
          description: "Finding related tables",
          done: "Related tables found",
          icon: Database,
          color: "text-green-600 bg-green-50 border-green-200",
        },
        schema_stats: {
          name: "Database Statistics",
          description: "Getting database statistics",
          done: "Statistics retrieved",
          icon: BarChart3,
          color: "text-purple-600 bg-purple-50 border-purple-200",
        },
      };

      if (actionMappings[toolAction]) {
        const mapping = actionMappings[toolAction];
        return {
          friendlyName: mapping.name,
          icon: mapping.icon,
          color: mapping.color,
          done: mapping.done,
          description: mapping.description,
        };
      }

      // Fallback for unknown MCP tools - make it readable
      const readableAction = toolAction
        .replace(/_/g, " ")
        .replace(/\b\w/g, (l) => l.toUpperCase());
      const readableServer = serverName
        .replace(/-/g, " ")
        .replace(/\b\w/g, (l) => l.toUpperCase());
      return {
        friendlyName: `${readableServer}: ${readableAction}`,
        icon: Wrench,
        color: "text-gray-600 bg-gray-50 border-gray-200",
        description: `Using ${readableAction.toLowerCase()}`,
      };
    }
  }

  // Handle common Claude Code tools
  const claudeCodeTools: Record<
    string,
    {
      name: string;
      description: string;
      icon: React.ElementType;
      done?: string;
      color: string;
    }
  > = {
    bash: {
      name: "Terminal Command",
      description: "Running command",
      icon: Terminal,
      color: "text-green-600 bg-green-50 border-green-200",
    },
    str_replace_editor: {
      name: "Code Editor",
      description: "Editing file",
      icon: Code,
      color: "text-blue-600 bg-blue-50 border-blue-200",
    },
    web_search: {
      name: "Web Search",
      description: "Searching web",
      icon: Globe,
      color: "text-purple-600 bg-purple-50 border-purple-200",
    },
    grep: {
      name: "Search Files",
      description: "Searching files",
      icon: FileSearch,
      color: "text-yellow-600 bg-yellow-50 border-yellow-200",
    },
    read: {
      name: "Read File",
      description: "Reading file",
      icon: FileSearch,
      color: "text-blue-600 bg-blue-50 border-blue-200",
    },
    write: {
      name: "Write File",
      description: "Writing file",
      icon: Code,
      color: "text-green-600 bg-green-50 border-green-200",
    },
    edit: {
      name: "Edit File",
      description: "Editing file",
      icon: Code,
      color: "text-blue-600 bg-blue-50 border-blue-200",
    },
  };

  if (claudeCodeTools[toolName]) {
    const tool = claudeCodeTools[toolName];
    return {
      friendlyName: tool.name,
      icon: tool.icon,
      color: tool.color,
      description: tool.description,
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

interface ToolCallIndicatorProps {
  tools: string[];
  className?: string;
  variant?: "compact" | "full";
  isCompleted?: boolean; // Whether these are completed tools vs active ones
}

export function ToolCallIndicator({
  tools,
  className = "",
  variant = "full",
  isCompleted = false,
}: ToolCallIndicatorProps) {
  if (tools.length === 0) return null;

  if (variant === "compact") {
    if (isCompleted) {
      const firstTool = tools?.[0] ? parseMcpToolName(tools[0]) : null;
      const Icon = firstTool?.icon as any;
      return (
        <div
          className={cn(
            "flex items-center gap-2",
            css`
              svg {
                width: 13px;
                height: 13px;
              }
            `,
            className
          )}
        >
          <div className="text-xs text-green-600 font-medium">
            {tools.length === 1 ? (
              <>
                {firstTool && (
                  <div className={cn("flex gap-1 items-center")}>
                    <Icon />
                    {firstTool?.done || firstTool.friendlyName}
                  </div>
                )}
              </>
            ) : (
              <div
                className={cn(
                  "flex gap-1 items-center",
                  css`
                    svg {
                      width: 13px;
                    }
                  `
                )}
              >
                <CheckCircle className="h-3 w-3 text-green-600" />
                Used {tools.length} tool{tools.length > 1 ? "s" : ""}
              </div>
            )}
          </div>
        </div>
      );
    }

    return (
      <div className={cn("flex items-center gap-2", className)}>
        <Loader2 className="h-3 w-3 text-green-600 animate-spin" />
        <span className="text-xs text-green-600 font-medium">
          Using {tools.length} tool{tools.length > 1 ? "s" : ""}
        </span>
      </div>
    );
  }

  return (
    <div className={cn("space-y-1", className)}>
      {[tools[tools.length - 1]].map((tool, index) => {
        const parsedTool = parseMcpToolName(tool);
        const Icon = parsedTool.icon;

        return (
          <Badge
            key={`${tool}-${index}`}
            variant="outline"
            className={cn(
              "flex items-center gap-1.5 text-xs px-2 py-1",
              isCompleted ? "" : "animate-pulse",
              parsedTool.color
            )}
          >
            <Icon className="h-3 w-3" />
            {isCompleted ? (
              <CheckCircle className="h-3 w-3 text-green-600" />
            ) : (
              <Loader2 className="h-3 w-3 animate-spin" />
            )}
            <span>
              {isCompleted
                ? parsedTool.friendlyName
                : `${parsedTool.description}...`}
            </span>
          </Badge>
        );
      })}
    </div>
  );
}
