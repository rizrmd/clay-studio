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
import { Separator } from "@/components/ui/separator";
import { Code2, Clock, FileText, Play, Copy, Check, ChevronLeft, ChevronRight } from "lucide-react";
import { api } from "@/lib/utils/api";
import { Button } from "@/components/ui/button";
import { parseMcpToolResult } from "../../../tool/tool-call-utils";

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
  onNavigate?: (direction: 'prev' | 'next') => void;
  hasNext?: boolean;
  hasPrev?: boolean;
}

export function ToolUsageDialog({
  toolUsageId,
  children,
  onNavigate,
  hasNext = false,
  hasPrev = false,
}: ToolUsageDialogProps) {
  const [toolUsage, setToolUsage] = useState<ToolUsageDetails | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState(false);
  const [parametersCopied, setParametersCopied] = useState(false);
  const [outputCopied, setOutputCopied] = useState(false);

  const fetchToolUsageDetails = async () => {
    if (toolUsage) return; // Already loaded

    setIsLoading(true);
    setError(null);

    try {
      const response = await api.get(`/tool-usages/${toolUsageId}`);
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

  // Clear tool usage when toolUsageId changes
  React.useEffect(() => {
    setToolUsage(null);
    setError(null);
  }, [toolUsageId]);

  const handleOpenChange = (newOpen: boolean) => {
    setOpen(newOpen);
    if (newOpen) {
      fetchToolUsageDetails();
    }
  };

  const formatOutput = (output: any): string => {
    if (!output) return "null";

    // Check for array format with MCP tool result
    if (Array.isArray(output) && output.length > 0) {
      const firstItem = output[0];
      if (
        firstItem &&
        typeof firstItem === "object" &&
        firstItem.text &&
        firstItem.type === "text"
      ) {
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

  const copyToClipboard = async (text: string, type: 'parameters' | 'output') => {
    try {
      await navigator.clipboard.writeText(text);
      if (type === 'parameters') {
        setParametersCopied(true);
        setTimeout(() => setParametersCopied(false), 2000);
      } else {
        setOutputCopied(true);
        setTimeout(() => setOutputCopied(false), 2000);
      }
    } catch (err) {
      console.error('Failed to copy to clipboard:', err);
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger asChild>{children}</DialogTrigger>
      <DialogContent className="min-w-[95vw] min-h-[90vh] text-xs flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Code2 className="h-5 w-5" />
              Tool Usage Details
            </div>
            {onNavigate && (hasPrev || hasNext) && (
              <div className="flex items-center gap-1">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => onNavigate('prev')}
                  disabled={!hasPrev}
                  className="h-8 w-8 p-0"
                >
                  <ChevronLeft className="h-4 w-4" />
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => onNavigate('next')}
                  disabled={!hasNext}
                  className="h-8 w-8 p-0"
                >
                  <ChevronRight className="h-4 w-4" />
                </Button>
              </div>
            )}
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
          <div className="space-y-4 flex-1 flex flex-col">
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

            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 flex-1">
              {/* Parameters */}
              <div className="space-y-2 flex flex-col">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <FileText className="h-4 w-4" />
                    <h3 className="font-semibold">Parameters</h3>
                  </div>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => copyToClipboard(formatJson(toolUsage.parameters), 'parameters')}
                    className="h-8 px-2"
                  >
                    {parametersCopied ? (
                      <Check className="h-3 w-3" />
                    ) : (
                      <Copy className="h-3 w-3" />
                    )}
                  </Button>
                </div>
                <div className="border  relative overflow-auto flex-1 bg-muted/30 rounded-lg">
                  <pre className="p-4 absolute inset-0 text-xs font-mono whitespace-pre-wrap">
                    {formatJson(toolUsage.parameters)}
                  </pre>
                </div>
              </div>

              {/* Output */}
              <div className="space-y-2 flex flex-col">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <FileText className="h-4 w-4" />
                    <h3 className="font-semibold">Output</h3>
                  </div>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => copyToClipboard(formatOutput(toolUsage.output), 'output')}
                    className="h-8 px-2"
                  >
                    {outputCopied ? (
                      <Check className="h-3 w-3" />
                    ) : (
                      <Copy className="h-3 w-3" />
                    )}
                  </Button>
                </div>
                <div className="border relative overflow-auto flex-1 bg-muted/30 rounded-lg ">
                  <pre className="p-4 absolute inset-0 text-xs font-mono whitespace-pre-wrap">
                    {formatOutput(toolUsage.output)}
                  </pre>
                </div>
              </div>
            </div>

            <Separator />

            {/* Metadata */}
            <div className="text-xs text-muted-foreground gap-x-7 flex-wrap flex flex-row justify-center items-center">
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
