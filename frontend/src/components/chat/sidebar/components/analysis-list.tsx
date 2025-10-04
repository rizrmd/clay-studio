import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Play,
  Code,
  Database,
  FileText,
  MoreHorizontal,
  Trash2,
  BarChart,
  Loader2,
} from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Analysis } from "@/lib/store/analysis-store";
import { sidebarActions, sidebarStore } from "@/lib/store/chat/sidebar-store";
import { useSnapshot } from "valtio";

interface AnalysisListProps {
  analyses: Analysis[];
  onAnalysisClick: (analysisId: string) => void;
  onRunAnalysis: (analysisId: string) => void;
  onAddNew?: () => void;
  activeAnalysisId?: string;
  projectId?: string;
}

export function AnalysisList({
  analyses = [],
  onAnalysisClick,
  onRunAnalysis,
  activeAnalysisId,
}: AnalysisListProps) {
  const sidebarSnapshot = useSnapshot(sidebarStore);

  const getTypeIcon = (type?: "sql" | "python" | "r" | string) => {
    if (!type) {
      return <Code className="h-4 w-4 text-muted-foreground" />;
    }

    switch (type.toLowerCase()) {
      case "sql":
        return <Database className="h-4 w-4 text-blue-500" />;
      case "python":
        return <Code className="h-4 w-4 text-green-500" />;
      case "r":
        return <FileText className="h-4 w-4 text-purple-500" />;
      case "javascript":
      case "js":
        return <Code className="h-4 w-4 text-yellow-500" />;
      default:
        return <Code className="h-4 w-4 text-muted-foreground" />;
    }
  };

  const handleDeleteSingle = async (analysisId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    sidebarActions.enterAnalysisDeleteMode(analysisId);
  };

  return (
    <div className="flex flex-col h-full">
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
          <div className="px-2">
            {analyses.map((analysis) => (
              <div
                key={analysis.id}
                className={cn(
                  "block w-full group text-left p-2 rounded-md hover:bg-muted border border-transparent transition-colors mb-1 cursor-pointer relative",
                  activeAnalysisId === analysis.id && "bg-muted border-blue-700/30",
                  sidebarSnapshot.isAnalysisDeleteMode &&
                    sidebarSnapshot.selectedAnalyses.includes(analysis.id) &&
                    "bg-red-50 dark:bg-red-900/20 border-red-500/30",
                  sidebarSnapshot.isAnalysisDeleteMode &&
                    "hover:bg-red-50 dark:hover:bg-red-900/10"
                )}
                onClick={() => {
                  if (sidebarSnapshot.isAnalysisDeleteMode) {
                    sidebarActions.toggleAnalysisSelection(analysis.id);
                  } else {
                    onAnalysisClick(analysis.id);
                  }
                }}
              >
                <div className={cn(
                  "flex items-start gap-2 overflow-hidden",
                  !sidebarSnapshot.isAnalysisDeleteMode && "group-hover:pr-8"
                )}>
                  {/* Icon */}
                  <div className="pt-1">
                    {analysis.status === 'running' ? (
                      <Loader2 className="h-4 w-4 animate-spin text-blue-500" />
                    ) : (
                      getTypeIcon(analysis.type)
                    )}
                  </div>

                  {/* Content */}
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <p className="text-sm font-medium truncate">
                        {analysis.name || "Untitled Analysis"}
                      </p>
                      {analysis.status === 'running' && (
                        <span className="text-xs text-blue-500 font-medium">Running</span>
                      )}
                    </div>
                    <p className="text-xs text-muted-foreground truncate">
                      {analysis.description || "No description"}
                    </p>
                    {analysis.created_at && (
                      <p className="text-xs text-muted-foreground mt-0.5">
                        {new Date(analysis.created_at).toLocaleDateString()}
                      </p>
                    )}
                  </div>

                  {/* Checkbox on the right in delete mode */}
                  {sidebarSnapshot.isAnalysisDeleteMode && (
                    <div className="pt-1 flex-shrink-0">
                      <Checkbox
                        checked={sidebarSnapshot.selectedAnalyses.includes(analysis.id)}
                      />
                    </div>
                  )}
                </div>

                {/* Actions dropdown - hidden in delete mode */}
                {!sidebarSnapshot.isAnalysisDeleteMode && (
                  <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-6 w-6 p-0"
                          onClick={(e) => e.stopPropagation()}
                        >
                          <MoreHorizontal className="h-3 w-3" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem
                          onClick={(e) => {
                            e.stopPropagation();
                            onRunAnalysis(analysis.id);
                          }}
                        >
                          <Play className="h-4 w-4 mr-2" />
                          Run Analysis
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onClick={(e) => handleDeleteSingle(analysis.id, e)}
                          className="text-red-600 focus:text-red-600"
                        >
                          <Trash2 className="h-4 w-4 mr-2" />
                          Delete
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}