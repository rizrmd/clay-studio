import React, { useState, useEffect } from "react";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Badge } from "@/components/ui/badge";
import { Loader2, Clock, AlertCircle, CheckCircle } from "lucide-react";
import { useToolUsage } from "@/hooks/use-tool-usage";
import { ToolUsage } from "@/types/chat";
import { ScrollArea } from "@/components/ui/scroll-area";
import { parseMcpToolName } from "./tool-call-utils";

interface ToolUsagePopoverProps {
  messageId: string;
  toolName: string;
  children: React.ReactNode;
  toolUsages?: ToolUsage[];
}

export function ToolUsagePopover({
  messageId: _messageId,
  toolName,
  children,
  toolUsages,
}: ToolUsagePopoverProps) {
  const [toolUsage, setToolUsage] = useState<ToolUsage | null>(null);
  const [isOpen, setIsOpen] = useState(false);
  const [hasFetched, setHasFetched] = useState(false);
  const { loading, error } = useToolUsage();

  useEffect(() => {
    if (isOpen && !hasFetched) {
      // Check if we have tool_usages passed as props
      // This is more efficient than making an API call
      if (toolUsages) {
        const usage = toolUsages.find((tu: ToolUsage) => tu.tool_name === toolName);
        setToolUsage(usage || null);
      }
      setHasFetched(true);
    }
  }, [isOpen, hasFetched, toolUsages, toolName]);

  const renderComplexValue = (value: any): React.ReactNode => {
    if (value === null || value === undefined) {
      return <span className="text-muted-foreground italic">null</span>;
    }

    if (typeof value === "boolean") {
      return (
        <span className={value ? "text-green-600" : "text-red-600"}>
          {String(value)}
        </span>
      );
    }

    if (typeof value === "number") {
      return <span className="text-blue-600">{String(value)}</span>;
    }

    if (typeof value === "string") {
      return <span>{value}</span>;
    }

    if (Array.isArray(value)) {
      // For small arrays, show inline
      if (
        value.length <= 3 &&
        value.every((v) => typeof v !== "object" || v === null)
      ) {
        return <span>[{value.map((v) => String(v)).join(", ")}]</span>;
      }

      // For larger arrays or arrays with objects, render as nested table
      if (
        value.length > 0 &&
        typeof value[0] === "object" &&
        value[0] !== null
      ) {
        return (
          <div className="mt-2">
            <div className="text-xs text-muted-foreground mb-1">
              Array ({value.length} items):
            </div>
            {renderJsonAsTable(value)}
          </div>
        );
      }

      return (
        <div className="max-h-20 overflow-y-auto">
          <pre className="text-xs">{JSON.stringify(value, null, 2)}</pre>
        </div>
      );
    }

    if (typeof value === "object") {
      // For small objects, show key count
      const keyCount = Object.keys(value).length;
      if (keyCount <= 5) {
        return (
          <div className="mt-2">
            <div className="text-xs text-muted-foreground mb-1">
              Object ({keyCount} keys):
            </div>
            {renderJsonAsTable(value)}
          </div>
        );
      }

      return (
        <div className="max-h-20 overflow-y-auto">
          <pre className="text-xs">{JSON.stringify(value, null, 2)}</pre>
        </div>
      );
    }

    return <span>{String(value)}</span>;
  };

  const renderJsonAsTable = (json: any) => {
    if (!json) return <span className="text-muted-foreground">None</span>;

    try {
      const data = typeof json === "string" ? JSON.parse(json) : json;

      // If it's an array of objects, render as table
      if (
        Array.isArray(data) &&
        data.length > 0 &&
        typeof data[0] === "object" &&
        data[0] !== null
      ) {
        // Get all unique keys from all objects
        const allKeys = new Set<string>();
        data.forEach((item) => {
          if (typeof item === "object" && item !== null) {
            Object.keys(item).forEach((key) => allKeys.add(key));
          }
        });
        const keys = Array.from(allKeys);

        return (
          <div className="overflow-x-auto">
            <table className="min-w-full text-xs border border-border rounded bg-white">
              <thead>
                <tr className="border-b border-border bg-muted/20">
                  {keys.map((key) => (
                    <th key={key} className="px-2 py-1 text-left font-medium">
                      {key}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {data.map((row: any, index: number) => (
                  <tr key={index} className="border-b border-border/50">
                    {keys.map((key) => (
                      <td key={key} className="px-2 py-1 max-w-48 align-top">
                        {renderComplexValue(row[key])}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        );
      }

      // If it's a single object, render as key-value table
      if (typeof data === "object" && data !== null && !Array.isArray(data)) {
        const entries = Object.entries(data);
        return (
          <div className="overflow-x-auto">
            <table className="min-w-full text-xs border border-border rounded bg-white">
              <thead>
                <tr className="border-b border-border bg-muted/20">
                  <th className="px-2 py-1 text-left font-medium">Key</th>
                  <th className="px-2 py-1 text-left font-medium">Value</th>
                </tr>
              </thead>
              <tbody>
                {entries.map(([key, value]) => (
                  <tr key={key} className="border-b border-border/50">
                    <td className="px-2 py-1 font-medium align-top">{key}</td>
                    <td className="px-2 py-1 max-w-96 align-top">
                      {renderComplexValue(value)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        );
      }

      // Fallback to formatted JSON
      return (
        <pre className="text-xs overflow-x-auto whitespace-pre-wrap break-words font-mono">
          {JSON.stringify(data, null, 2)}
        </pre>
      );
    } catch {
      return (
        <pre className="text-xs overflow-x-auto whitespace-pre-wrap break-words font-mono">
          {String(json)}
        </pre>
      );
    }
  };

  const renderOutput = (output: any) => {
    if (!output)
      return <span className="text-muted-foreground">No output</span>;

    try {
      // Handle new status-wrapped format
      if (typeof output === "object" && output.status) {
        const status = output.status;
        const result = output.result;

        if (status === "executing") {
          return (
            <div className="flex items-center gap-2 text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              <span>Executing...</span>
            </div>
          );
        }

        if (status === "success" && result) {
          return renderJsonAsTable(result);
        }

        if (status === "completed") {
          return (
            <div className="text-muted-foreground text-[10px] font-mono overflow-auto">
              {typeof result === "string" ? result : "Tool execution completed"}
            </div>
          );
        }

        // Fallback to showing the whole object if status format is unexpected
        return renderJsonAsTable(output);
      }

      if (typeof output === "string") {
        // Try to parse as JSON first
        try {
          const parsed = JSON.parse(output);
          return renderJsonAsTable(parsed);
        } catch {
          // Return as string if not JSON
          return (
            <pre className="text-xs overflow-x-auto whitespace-pre-wrap break-words font-mono">
              {output}
            </pre>
          );
        }
      }

      // Handle objects (legacy format)
      if (typeof output === "object") {
        return renderJsonAsTable(output);
      }

      return (
        <pre className="text-xs overflow-x-auto whitespace-pre-wrap break-words font-mono">
          {String(output)}
        </pre>
      );
    } catch {
      return (
        <pre className="text-xs overflow-x-auto whitespace-pre-wrap break-words font-mono">
          {String(output)}
        </pre>
      );
    }
  };

  const formatExecutionTime = (ms?: number) => {
    if (!ms) return "N/A";
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  const toolInfo = parseMcpToolName(toolName);
  const Icon = toolInfo.icon;

  return (
    <Popover open={isOpen} onOpenChange={setIsOpen}>
      <PopoverTrigger asChild>{children}</PopoverTrigger>
      <PopoverContent
        className="w-[500px] max-h-[600px] p-0"
        align="start"
      >
        <div className="flex items-center justify-between border-b px-4 py-3">
          <div className="flex items-center gap-2">
            <Icon className="h-4 w-4 text-muted-foreground" />
            <h3 className="font-medium">{toolInfo.friendlyName}</h3>
          </div>
          <div className="flex items-center gap-2">
            {toolUsage && (
              <Badge variant="outline" className="flex items-center gap-1">
                <CheckCircle className="h-3 w-3 text-green-600" />
                Completed
              </Badge>
            )}
          </div>
        </div>

        <ScrollArea className="max-h-[500px]">
          <div className="p-4 space-y-4">
            {loading && !hasFetched && (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              </div>
            )}

            {error && (
              <div className="flex items-center gap-2 text-destructive text-sm">
                <AlertCircle className="h-4 w-4" />
                <span>{error}</span>
              </div>
            )}

            {!loading && !error && !toolUsage && hasFetched && (
              <div className="text-sm text-muted-foreground text-center py-4">
                Tool was executed successfully.
              </div>
            )}

            {toolUsage && (
              <>
                {/* Parameters Section */}
                {!!toolUsage.parameters &&
                  Object.keys(toolUsage.parameters).length > 0 && (
                    <div className="space-y-2">
                      <h4 className="text-sm font-medium text-muted-foreground">
                        Parameters
                      </h4>
                      <div className="bg-muted/50 rounded-lg p-3">
                        {renderJsonAsTable(toolUsage.parameters)}
                      </div>
                    </div>
                  )}

                {/* Output Section */}
                <div className="space-y-2">
                  <h4 className="text-sm font-medium text-muted-foreground items-center flex gap-2">
                    Output{" "}
                    {toolUsage.output?.status === "completed" && (
                      <CheckCircle className="h-4 w-4 inline mr-2 text-green-600" />
                    )}
                  </h4>
                  <div className="bg-muted/50 rounded-lg p-3 max-h-60 overflow-auto">
                    {renderOutput(toolUsage.output)}
                  </div>
                </div>

                {/* Metadata */}
                {(toolUsage.createdAt ||
                  toolUsage.execution_time_ms !== undefined) && (
                  <div className="flex items-center justify-between text-xs text-muted-foreground pt-2 border-t">
                    {toolUsage.createdAt && (
                      <span>
                        {new Date(toolUsage.createdAt).toLocaleString(
                          undefined,
                          {
                            hour12: false,
                            year: "numeric",
                            month: "2-digit",
                            day: "2-digit",
                            hour: "2-digit",
                            minute: "2-digit",
                            second: "2-digit",
                          }
                        )}
                      </span>
                    )}
                    {toolUsage.execution_time_ms !== undefined && (
                      <div className="flex items-center gap-1">
                        <Clock className="h-3 w-3" />
                        <span>
                          {formatExecutionTime(toolUsage.execution_time_ms)}
                        </span>
                      </div>
                    )}
                  </div>
                )}
              </>
            )}
          </div>
        </ScrollArea>
      </PopoverContent>
    </Popover>
  );
}
