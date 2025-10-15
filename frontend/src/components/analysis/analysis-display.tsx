"use client";

import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Play,
  Square,
  RefreshCw,
  Clock,
  CheckCircle,
  XCircle,
  Eye,
  Download,
  Maximize2,
  Minimize2
} from "lucide-react";
import { mcpAnalysisApi, type McpAnalysisJob } from "@/lib/api/analysis";

interface AnalysisDisplayProps {
  analysisId: string;
  title?: string;
  description?: string;
  parameters?: any;
}

const statusConfig = {
  pending: {
    icon: Clock,
    label: "Pending",
    color: "bg-yellow-100 text-yellow-800 border-yellow-200",
    variant: "outline" as const,
  },
  running: {
    icon: RefreshCw,
    label: "Running",
    color: "bg-blue-100 text-blue-800 border-blue-200",
    variant: "outline" as const,
  },
  completed: {
    icon: CheckCircle,
    label: "Completed",
    color: "bg-green-100 text-green-800 border-green-200",
    variant: "default" as const,
  },
  failed: {
    icon: XCircle,
    label: "Failed",
    color: "bg-red-100 text-red-800 border-red-200",
    variant: "destructive" as const,
  },
  cancelled: {
    icon: Square,
    label: "Cancelled",
    color: "bg-gray-100 text-gray-800 border-gray-200",
    variant: "outline" as const,
  },
};

function formatDuration(startTime?: string, endTime?: string): string {
  if (!startTime) return "—";

  const start = new Date(startTime);
  const end = endTime ? new Date(endTime) : new Date();
  const durationMs = end.getTime() - start.getTime();

  if (durationMs < 1000) return `${durationMs}ms`;
  if (durationMs < 60000) return `${(durationMs / 1000).toFixed(1)}s`;
  return `${(durationMs / 60000).toFixed(1)}m`;
}

function formatTimestamp(timestamp?: string): string {
  if (!timestamp) return "—";
  return new Date(timestamp).toLocaleString();
}

export function AnalysisDisplay({
  analysisId,
  title,
  description,
  parameters,
}: AnalysisDisplayProps) {
  const [currentJob, setCurrentJob] = useState<McpAnalysisJob | null>(null);
  const [isMaximized, setIsMaximized] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Poll for job status updates
  useEffect(() => {
    if (!currentJob?.job_id) return;

    const pollInterval = setInterval(async () => {
      try {
        const updatedJob = await mcpAnalysisApi.getJob(currentJob.job_id);
        setCurrentJob(updatedJob);

        // Stop polling if job is completed, failed, or cancelled
        if (updatedJob.status === 'completed' || updatedJob.status === 'failed' || updatedJob.status === 'cancelled') {
          clearInterval(pollInterval);
        }
      } catch (err) {
        console.error("Failed to poll job status:", err);
        clearInterval(pollInterval);
      }
    }, 2000);

    return () => clearInterval(pollInterval);
  }, [currentJob?.job_id, currentJob?.status]);

  const runAnalysis = async () => {
    setIsLoading(true);
    setError(null);

    try {
      const response = await mcpAnalysisApi.runAnalysis({
        analysis_id: analysisId,
        parameters: parameters || {},
      });

      if (response.success) {
        // The job was submitted successfully, we need to get the job ID
        // This might require an additional API call or be returned in the response
        // For now, we'll fetch the latest jobs to find our new job
        const jobsResponse = await mcpAnalysisApi.listJobs({
          analysis_id: analysisId,
          limit: 1
        });

        if (jobsResponse.jobs.length > 0) {
          setCurrentJob(jobsResponse.jobs[0]);
        }
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to run analysis");
    } finally {
      setIsLoading(false);
    }
  };

  const cancelAnalysis = async () => {
    if (!currentJob?.job_id) return;

    try {
      await mcpAnalysisApi.cancelJob(currentJob.job_id);
      // Update the local state to reflect cancellation
      setCurrentJob(prev => prev ? {
        ...prev,
        status: 'cancelled',
        completed_at: new Date().toISOString(),
        error_message: "Cancelled by user"
      } : null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to cancel analysis");
    }
  };

  const downloadResults = () => {
    if (!currentJob?.result) return;

    const dataStr = JSON.stringify(currentJob.result, null, 2);
    const dataBlob = new Blob([dataStr], { type: 'application/json' });
    const url = URL.createObjectURL(dataBlob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `analysis-${currentJob.job_id}.json`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  };

  const StatusIcon = currentJob ? statusConfig[currentJob.status].icon : Clock;
  const statusConfigData = currentJob ? statusConfig[currentJob.status] : statusConfig.pending;

  const content = (
    <Card className="w-full">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <CardTitle className="text-lg flex items-center gap-2">
              <StatusIcon className="h-5 w-5" />
              {title || "Analysis"}
            </CardTitle>
            {description && (
              <CardDescription>{description}</CardDescription>
            )}
          </div>
          <div className="flex items-center gap-2">
            <Badge variant={statusConfigData.variant} className={statusConfigData.color}>
              {statusConfigData.label}
            </Badge>
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              onClick={() => setIsMaximized(!isMaximized)}
              title={isMaximized ? "Minimize" : "Maximize"}
            >
              {isMaximized ? <Minimize2 className="h-4 w-4" /> : <Maximize2 className="h-4 w-4" />}
            </Button>
          </div>
        </div>
      </CardHeader>

      <CardContent className="space-y-4">
        {/* Action Buttons */}
        <div className="flex items-center gap-2">
          {!currentJob || currentJob.status === 'completed' || currentJob.status === 'failed' || currentJob.status === 'cancelled' ? (
            <Button
              onClick={runAnalysis}
              disabled={isLoading}
              className="flex items-center gap-2"
            >
              {isLoading ? (
                <RefreshCw className="h-4 w-4 animate-spin" />
              ) : (
                <Play className="h-4 w-4" />
              )}
              {currentJob ? "Run Again" : "Run Analysis"}
            </Button>
          ) : (
            <Button
              onClick={cancelAnalysis}
              variant="destructive"
              className="flex items-center gap-2"
            >
              <Square className="h-4 w-4" />
              Cancel
            </Button>
          )}

          {currentJob?.result && (
            <Button
              variant="outline"
              onClick={downloadResults}
              className="flex items-center gap-2"
            >
              <Download className="h-4 w-4" />
              Download
            </Button>
          )}
        </div>

        {error && (
          <div className="flex items-center gap-2 p-3 bg-red-50 border border-red-200 rounded-md">
            <XCircle className="h-4 w-4 text-red-600" />
            <span className="text-sm text-red-700">{error}</span>
          </div>
        )}

        {currentJob && (
          <Tabs defaultValue="status" className="w-full">
            <TabsList className="grid w-full grid-cols-3">
              <TabsTrigger value="status">Status</TabsTrigger>
              <TabsTrigger value="details">Details</TabsTrigger>
              <TabsTrigger value="results">Results</TabsTrigger>
            </TabsList>

            <TabsContent value="status" className="space-y-4">
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="font-medium">Created:</span>
                  <div className="text-muted-foreground">{formatTimestamp(currentJob.created_at)}</div>
                </div>
                <div>
                  <span className="font-medium">Started:</span>
                  <div className="text-muted-foreground">{formatTimestamp(currentJob.started_at)}</div>
                </div>
                <div>
                  <span className="font-medium">Duration:</span>
                  <div className="text-muted-foreground">
                    {formatDuration(currentJob.started_at, currentJob.completed_at)}
                  </div>
                </div>
                <div>
                  <span className="font-medium">Job ID:</span>
                  <div className="text-muted-foreground font-mono text-xs">{currentJob.job_id}</div>
                </div>
              </div>

              {currentJob.status === 'running' && (
                <div className="flex items-center gap-2 text-blue-600">
                  <RefreshCw className="h-4 w-4 animate-spin" />
                  <span className="text-sm">Analysis is running...</span>
                </div>
              )}
            </TabsContent>

            <TabsContent value="details" className="space-y-4">
              <div className="space-y-2">
                <div className="text-sm">
                  <span className="font-medium">Analysis ID:</span>
                  <div className="text-muted-foreground font-mono text-xs">{currentJob.analysis_id}</div>
                </div>

                {parameters && Object.keys(parameters).length > 0 && (
                  <div className="text-sm">
                    <span className="font-medium">Parameters:</span>
                    <ScrollArea className="mt-1 h-24 rounded-md border p-2">
                      <pre className="text-xs text-muted-foreground">
                        {JSON.stringify(parameters, null, 2)}
                      </pre>
                    </ScrollArea>
                  </div>
                )}

                {currentJob.error_message && (
                  <div className="text-sm">
                    <span className="font-medium text-red-600">Error:</span>
                    <div className="text-red-600 text-xs mt-1 p-2 bg-red-50 rounded border">
                      {currentJob.error_message}
                    </div>
                  </div>
                )}
              </div>
            </TabsContent>

            <TabsContent value="results" className="space-y-4">
              {currentJob.result ? (
                <div className="space-y-2">
                  <div className="flex items-center gap-2">
                    <CheckCircle className="h-4 w-4 text-green-600" />
                    <span className="text-sm font-medium">Analysis completed successfully</span>
                  </div>

                  <ScrollArea className="h-64 rounded-md border p-3">
                    <pre className="text-xs text-muted-foreground">
                      {JSON.stringify(currentJob.result, null, 2)}
                    </pre>
                  </ScrollArea>
                </div>
              ) : (
                <div className="flex items-center justify-center h-32 text-muted-foreground">
                  <div className="text-center">
                    <Eye className="h-8 w-8 mx-auto mb-2 opacity-50" />
                    <p className="text-sm">No results available yet</p>
                  </div>
                </div>
              )}
            </TabsContent>
          </Tabs>
        )}
      </CardContent>
    </Card>
  );

  if (isMaximized) {
    return (
      <div className="fixed inset-0 z-[100] flex flex-col bg-background p-4">
        <div className="flex-1">
          {content}
        </div>
      </div>
    );
  }

  return content;
}