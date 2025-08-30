import { Loader2, CheckCircle, Clock } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { cn } from "@/lib/utils";
import { css } from "goober";
import { ToolUsagePopover } from "./tool-usage";
import { parseMcpToolName } from "./tool-call-utils";

interface ToolCallIndicatorProps {
  tools: string[];
  className?: string;
  variant?: "compact" | "full";
  isCompleted?: boolean; // Whether these are completed tools vs active ones
  messageId?: string; // Message ID for fetching tool usage details
  toolUsages?: any[]; // Tool usage data for the message
}

export function ToolCallIndicator({
  tools,
  className = "",
  variant = "full",
  isCompleted = false,
  messageId,
  toolUsages,
}: ToolCallIndicatorProps) {
  
  const formatExecutionTime = (ms?: number) => {
    if (!ms) return null;
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  const getToolExecutionTime = (toolName: string) => {
    if (!toolUsages || toolUsages.length === 0) {
      return null;
    }
    
    const usage = toolUsages.find(tu => tu.tool_name === toolName);
    
    // Check for execution time in the usage data
    const executionTime = usage?.execution_time_ms;
    
    if (executionTime !== undefined && executionTime !== null) {
      return formatExecutionTime(executionTime);
    }
    return null;
  };
  
  if (!tools || tools.length === 0) return null;

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
                {firstTool && messageId ? (
                  <ToolUsagePopover messageId={messageId} toolName={tools[0]} toolUsages={toolUsages}>
                    <div
                      className={cn(
                        "flex gap-1 items-center cursor-pointer hover:opacity-80 transition-opacity"
                      )}
                    >
                      <Icon />
                      {firstTool?.done || firstTool.friendlyName}
                      {getToolExecutionTime(tools[0]) && (
                        <span className="text-muted-foreground ml-1">
                          ({getToolExecutionTime(tools[0])})
                        </span>
                      )}
                    </div>
                  </ToolUsagePopover>
                ) : firstTool ? (
                  <div className={cn("flex gap-1 items-center")}>
                    <Icon />
                    {firstTool?.done || firstTool.friendlyName}
                    {getToolExecutionTime(tools[0]) && (
                      <span className="text-muted-foreground ml-1">
                        ({getToolExecutionTime(tools[0])})
                      </span>
                    )}
                  </div>
                ) : null}
              </>
            ) : (
              <Popover>
                <PopoverTrigger asChild>
                  <div
                    className={cn(
                      "flex gap-1 items-center cursor-pointer hover:opacity-80 transition-opacity",
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
                </PopoverTrigger>
                <PopoverContent className="w-auto min-w-[250px] p-3">
                  <div className="space-y-2">
                    <div className="text-sm font-medium mb-2">Tools Used:</div>
                    {tools.map((tool, index) => {
                      const parsedTool = parseMcpToolName(tool);
                      const Icon = parsedTool.icon;
                      const executionTime = getToolExecutionTime(tool);
                      const content = (
                        <div
                          key={`${tool}-${index}`}
                          className="flex items-center justify-between gap-2 text-sm w-full"
                        >
                          <div className="flex items-center gap-2">
                            <Icon className="h-4 w-4 text-muted-foreground" />
                            <span>{parsedTool.friendlyName}</span>
                          </div>
                          {executionTime && (
                            <div className="flex items-center gap-1 text-xs text-muted-foreground ml-auto">
                              <Clock className="h-3 w-3" />
                              <span>{executionTime}</span>
                            </div>
                          )}
                        </div>
                      );

                      // Wrap in ToolUsagePopover if we have a messageId
                      if (messageId && isCompleted) {
                        return (
                          <ToolUsagePopover
                            key={`${tool}-${index}`}
                            messageId={messageId}
                            toolName={tool}
                            toolUsages={toolUsages}
                          >
                            <div className="cursor-pointer hover:bg-accent rounded px-1 -mx-1 transition-colors">
                              {content}
                            </div>
                          </ToolUsagePopover>
                        );
                      }

                      return content;
                    })}
                  </div>
                </PopoverContent>
              </Popover>
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
        const executionTime = getToolExecutionTime(tool);

        return (
          <Badge
            key={`${tool}-${index}`}
            variant="outline"
            className={cn(
              "flex items-center gap-1.5 text-xs px-2 py-1",
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
            {isCompleted && executionTime && (
              <span className="text-muted-foreground flex items-center gap-1">
                <Clock className="h-3 w-3" />
                {executionTime}
              </span>
            )}
          </Badge>
        );
      })}
    </div>
  );
}
