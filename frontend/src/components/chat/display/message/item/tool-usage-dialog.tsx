import * as React from "react";
import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { Code2, Clock, FileText, Play } from "lucide-react";
import { api } from "@/lib/utils/api";

interface ToolUsageDetails {
  id: string;
  message_id: string;
  tool_name: string;
  tool_use_id?: string;
  parameters?: any;
  output?: any;
  execution_time_ms?: number;
  createdAt?: string;
}

interface ToolUsageDialogProps {
  toolUsageId: string;
  children: React.ReactNode;
}

export function ToolUsageDialog({
  toolUsageId,
  children,
}: ToolUsageDialogProps) {
  const [toolUsage, setToolUsage] = useState<ToolUsageDetails | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState(false);

  const fetchToolUsageDetails = async () => {
    if (toolUsage) return; // Already loaded

    console.log("Fetching tool usage details for ID:", toolUsageId);
    setIsLoading(true);
    setError(null);

    try {
      console.log("Making API call to:", `/tool-usages/${toolUsageId}`);
      const response = await api.get(`/tool-usages/${toolUsageId}`);
      console.log("Full API response:", response);
      console.log("Tool usage data:", response);
      setToolUsage(response);
    } catch (err) {
      console.error("Failed to fetch tool usage details:", err);
      console.error("Error details:", {
        message: err instanceof Error ? err.message : "Unknown error",
        stack: err instanceof Error ? err.stack : undefined,
        response: (err as any)?.response,
      });
      setError(
        err instanceof Error
          ? err.message
          : "Failed to fetch tool usage details"
      );
    } finally {
      setIsLoading(false);
    }
  };

  const handleOpenChange = (newOpen: boolean) => {
    setOpen(newOpen);
    if (newOpen) {
      fetchToolUsageDetails();
    }
  };

  const parseMcpToolResult = (text: string): any => {
    try {
      if (text.includes("[Resource from interaction at mcp://tool-result/")) {
        const match = text.match(/\[Resource from interaction at mcp:\/\/tool-result\/[^\]]+\]\s*(.+)/s);
        if (match && match[1]) {
          const jsonText = match[1].trim();
          return JSON.parse(jsonText);
        }
      }
      return null;
    } catch {
      return null;
    }
  };

  const formatOutput = (output: any): string => {
    if (!output) return "null";

    // Check for array format with MCP tool result
    if (Array.isArray(output) && output.length > 0) {
      const firstItem = output[0];
      if (firstItem && typeof firstItem === "object" && firstItem.text && firstItem.type === "text") {
        const parsedMcp = parseMcpToolResult(firstItem.text);
        if (parsedMcp) {
          return JSON.stringify(parsedMcp, null, 2);
        }
      }
    }

    return JSON.stringify(output, null, 2);
  };

  const formatJson = (obj: any) => {
    if (!obj) return "null";
    return JSON.stringify(obj, null, 2);
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger asChild>{children}</DialogTrigger>
      <DialogContent className="min-w-[95vw] max-h-[90vh] text-xs">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Code2 className="h-5 w-5" />
            Tool Usage Details
          </DialogTitle>
        </DialogHeader>

        {isLoading && (
          <div className="flex items-center justify-center py-8">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
            <span className="ml-2">Loading tool usage details...</span>
          </div>
        )}

        {error && (
          <div className="flex items-center justify-center py-8">
            <div className="text-red-500">Error: {error}</div>
          </div>
        )}

        {toolUsage && (
          <div className="space-y-4">
            {/* Tool Info */}
            <div className="flex items-center gap-4 p-4 border rounded-lg bg-muted/50">
              <Play className="h-5 w-5 text-green-500" />
              <div className="flex-1">
                <div className="font-mono text-sm">{toolUsage.tool_name}</div>
                <div className="text-muted-foreground">ID: {toolUsage.id}</div>
              </div>
              {toolUsage.execution_time_ms && (
                <Badge variant="secondary" className="flex items-center gap-1">
                  <Clock className="h-3 w-3" />
                  {toolUsage.execution_time_ms}ms
                </Badge>
              )}
            </div>

            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
              {/* Parameters */}
              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <FileText className="h-4 w-4" />
                  <h3 className="font-semibold">Parameters</h3>
                </div>
                <div className="border rounded-lg">
                  <ScrollArea className="h-60">
                    <pre className="p-4 text-xs font-mono bg-muted/30 rounded-lg whitespace-pre-wrap">
                      {formatJson(toolUsage.parameters)}
                    </pre>
                  </ScrollArea>
                </div>
              </div>

              {/* Output */}
              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <FileText className="h-4 w-4" />
                  <h3 className="font-semibold">Output</h3>
                </div>
                <div className="border rounded-lg">
                  <ScrollArea className="h-60">
                    <pre className="p-4 text-xs font-mono bg-muted/30 rounded-lg whitespace-pre-wrap">
                      {formatOutput(toolUsage.output)}
                    </pre>
                  </ScrollArea>
                </div>
              </div>
            </div>

            <Separator />

            {/* Metadata */}
            <div className="text-xs text-muted-foreground space-y-1">
              <div>Message ID: {toolUsage.message_id}</div>
              {toolUsage.tool_use_id && (
                <div>Tool Use ID: {toolUsage.tool_use_id}</div>
              )}
              {toolUsage.createdAt && (
                <div>
                  Created: {new Date(toolUsage.createdAt).toLocaleString()}
                </div>
              )}
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
