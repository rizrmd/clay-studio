"use client";

import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Play,
  Square,
  RefreshCw,
  Clock,
  CheckCircle,
  XCircle,
  Download
} from "lucide-react";
import { mcpAnalysisApi, type McpAnalysisJob } from "@/lib/api/analysis";

interface AnalysisDisplayProps {
  analysisId: string;
  title?: string;
  description?: string;
  parameters?: any;
}

const statusConfig = {
  pending: { icon: Clock, color: "bg-yellow-100 text-yellow-800" },
  running: { icon: RefreshCw, color: "bg-blue-100 text-blue-800" },
  completed: { icon: CheckCircle, color: "bg-green-100 text-green-800" },
  failed: { icon: XCircle, color: "bg-red-100 text-red-800" },
  cancelled: { icon: Square, color: "bg-gray-100 text-gray-800" },
};

export function AnalysisDisplay({
  analysisId,
  title,
  description,
  parameters,
}: AnalysisDisplayProps) {
  const [currentJob, setCurrentJob] = useState<McpAnalysisJob | null>(null);
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
        if (['completed', 'failed', 'cancelled'].includes(updatedJob.status)) {
          clearInterval(pollInterval);
        }
      } catch (err) {
        clearInterval(pollInterval);
      }
    }, 2000);

    return () => clearInterval(pollInterval);
  }, [currentJob?.job_id, currentJob?.status]);

  const runAnalysis = async () => {
    setIsLoading(true);
    setError(null);

    try {
      await mcpAnalysisApi.runAnalysis({
        analysis_id: analysisId,
        parameters: parameters || {},
      });

      // Get the latest job
      const jobsResponse = await mcpAnalysisApi.listJobs({
        analysis_id: analysisId,
        limit: 1
      });

      if (jobsResponse.jobs.length > 0) {
        setCurrentJob(jobsResponse.jobs[0]);
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
    link.click();
    URL.revokeObjectURL(url);
  };

  const StatusIcon = currentJob ? statusConfig[currentJob.status].icon : Clock;
  const statusColor = currentJob ? statusConfig[currentJob.status].color : statusConfig.pending.color;
  const statusLabel = currentJob ? currentJob.status.charAt(0).toUpperCase() + currentJob.status.slice(1) : "Pending";

  return (
    <div className="border rounded-lg p-4 bg-white">
      {/* Header */}
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <StatusIcon className="h-4 w-4" />
          <span className="font-medium">{title || "Analysis"}</span>
        </div>
        <Badge className={statusColor}>
          {statusLabel}
        </Badge>
      </div>

      {description && (
        <p className="text-sm text-gray-600 mb-3">{description}</p>
      )}

      {/* Action Buttons */}
      <div className="flex items-center gap-2 mb-3">
        {!currentJob || ['completed', 'failed', 'cancelled'].includes(currentJob.status) ? (
          <Button
            onClick={runAnalysis}
            disabled={isLoading}
            size="sm"
            className="flex items-center gap-2"
          >
            {isLoading ? (
              <RefreshCw className="h-3 w-3 animate-spin" />
            ) : (
              <Play className="h-3 w-3" />
            )}
            {currentJob ? "Run Again" : "Run"}
          </Button>
        ) : (
          <Button
            onClick={cancelAnalysis}
            variant="destructive"
            size="sm"
            className="flex items-center gap-2"
          >
            <Square className="h-3 w-3" />
            Cancel
          </Button>
        )}

        {currentJob?.result && (
          <Button
            variant="outline"
            onClick={downloadResults}
            size="sm"
            className="flex items-center gap-2"
          >
            <Download className="h-3 w-3" />
            Download
          </Button>
        )}
      </div>

      {/* Error Display */}
      {error && (
        <div className="flex items-center gap-2 p-2 bg-red-50 border border-red-200 rounded text-sm text-red-700 mb-3">
          <XCircle className="h-3 w-3" />
          {error}
        </div>
      )}

      {/* Analysis Parameters/Filters */}
      {(parameters && Object.keys(parameters).length > 0) && (
        <div className="text-xs text-gray-500 space-y-1 mb-3">
          <div className="font-medium text-gray-700">Parameters:</div>
          {Object.entries(parameters).map(([key, value]) => (
            <div key={key} className="flex gap-2">
              <span className="font-medium">{key}:</span>
              <span>{typeof value === 'object' ? JSON.stringify(value) : String(value)}</span>
            </div>
          ))}
        </div>
      )}

      {/* Job Status */}
      {currentJob && (
        <div className="text-xs text-gray-500 space-y-1">
          <div>Job ID: {currentJob.job_id}</div>
          {currentJob.status === 'running' && (
            <div className="flex items-center gap-1 text-blue-600">
              <RefreshCw className="h-3 w-3 animate-spin" />
              Running...
            </div>
          )}
          {currentJob.status === 'completed' && currentJob.result && (
            <div className="text-green-600">✓ Analysis completed</div>
          )}
          {currentJob.status === 'failed' && currentJob.error_message && (
            <div className="text-red-600">✗ {currentJob.error_message}</div>
          )}
        </div>
      )}
    </div>
  );
}