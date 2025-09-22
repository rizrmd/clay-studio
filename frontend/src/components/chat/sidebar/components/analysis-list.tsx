import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import {
  Play,
  Clock,
  CheckCircle,
  XCircle,
  Code,
  Database,
  FileText,
  MoreVertical,
  BarChart,
} from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Analysis } from "@/lib/store/analysis-store";

interface AnalysisListProps {
  analyses: Analysis[];
  onAnalysisClick: (analysisId: string) => void;
  onRunAnalysis: (analysisId: string) => void;
  onAddNew?: () => void;
  activeAnalysisId?: string;
}

export function AnalysisList({
  analyses = [],
  onAnalysisClick,
  onRunAnalysis,
  activeAnalysisId,
}: AnalysisListProps) {
  const getTypeIcon = (type: "sql" | "python" | "r") => {
    switch (type) {
      case "sql":
        return <Database className="w-4 h-4 text-muted-foreground" />;
      case "python":
        return <Code className="w-4 h-4 text-muted-foreground" />;
      case "r":
        return <FileText className="w-4 h-4 text-muted-foreground" />;
      default:
        return <Code className="w-4 h-4 text-muted-foreground" />;
    }
  };

  const getStatusIcon = (status: Analysis["status"]) => {
    switch (status) {
      case "running":
        return <Play className="h-3 w-3" />;
      case "completed":
        return <CheckCircle className="h-3 w-3 text-green-500" />;
      case "failed":
        return <XCircle className="h-3 w-3 text-red-500" />;
      default:
        return <Clock className="h-3 w-3" />;
    }
  };

  const getStatusColor = (status: Analysis["status"]) => {
    switch (status) {
      case "running":
        return "bg-blue-100 text-blue-700";
      case "completed":
        return "bg-green-100 text-green-700";
      case "failed":
        return "bg-red-100 text-red-700";
      default:
        return "bg-gray-100 text-gray-700";
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Analysis list */}
      <div className="flex-1 overflow-y-auto">
        {analyses.length === 0 ? (
          <div className="p-4 text-center">
            <BarChart className="h-8 w-8 mx-auto mb-3 text-muted-foreground" />
            <p className="text-sm font-medium mb-1">No analyses yet</p>
            <p className="text-xs text-muted-foreground text-center">
              Use chat to create new analyses
            </p>
          </div>
        ) : (
          <div className="p-2 space-y-1">
            {analyses.map((analysis) => (
              <Card
                key={analysis.id}
                className={cn(
                  "p-2 cursor-pointer transition-all hover:bg-accent/50",
                  activeAnalysisId === analysis.id && "bg-accent border-accent"
                )}
                onClick={() => onAnalysisClick(analysis.id)}
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      {getTypeIcon(analysis.type)}
                      <span className="text-sm font-medium truncate">
                        {analysis.name}
                      </span>
                    </div>

                    {analysis.description && (
                      <p className="text-xs text-muted-foreground line-clamp-2">
                        {analysis.description}
                      </p>
                    )}

                    <div className="flex items-center gap-2 mt-2">
                      <Badge
                        variant="secondary"
                        className={cn(
                          "text-xs",
                          getStatusColor(analysis.status)
                        )}
                      >
                        {getStatusIcon(analysis.status)}
                        <span className="ml-1 capitalize">
                          {analysis.status}
                        </span>
                      </Badge>

                      {analysis.last_job &&
                        analysis.last_job.execution_time_ms && (
                          <span className="text-xs text-muted-foreground">
                            {Math.round(
                              analysis.last_job.execution_time_ms / 1000
                            )}
                            s
                          </span>
                        )}
                    </div>
                  </div>

                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-6 w-6 p-0 opacity-0 group-hover:opacity-100 hover:bg-accent"
                      >
                        <MoreVertical className="h-3 w-3" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end" className="w-48">
                      <DropdownMenuItem
                        onClick={() => onRunAnalysis(analysis.id)}
                      >
                        Run Analysis
                      </DropdownMenuItem>
                      <DropdownMenuItem>View Results</DropdownMenuItem>
                      <DropdownMenuItem>Schedule</DropdownMenuItem>
                      <DropdownMenuItem>Export Results</DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              </Card>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
