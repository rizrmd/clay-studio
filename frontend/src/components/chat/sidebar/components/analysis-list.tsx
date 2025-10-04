import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
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
import { Analysis, analysisActions } from "@/lib/store/analysis-store";
import { analysisApi } from "@/lib/api/analysis";
import { tabsActions, tabsStore } from "@/lib/store/tabs-store";
import { useSnapshot } from "valtio";
import { useNavigate } from "react-router-dom";

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
  projectId,
}: AnalysisListProps) {
  const tabsSnapshot = useSnapshot(tabsStore);
  const navigate = useNavigate();

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

  const handleDelete = async (analysisId: string, e: React.MouseEvent) => {
    e.stopPropagation();

    if (!confirm('Are you sure you want to delete this analysis?')) return;

    try {
      // Check if we're currently viewing this analysis
      const isCurrentlyViewing = tabsSnapshot.tabs.some(
        t => t.type === 'analysis' &&
             t.metadata.analysisId === analysisId &&
             t.id === tabsSnapshot.activeTabId
      );

      await analysisApi.deleteAnalysis(analysisId);

      // Remove from store
      analysisActions.removeAnalysis(analysisId);

      // Close the tab if it's open
      const analysisTab = tabsSnapshot.tabs.find(
        t => t.type === 'analysis' && t.metadata.analysisId === analysisId
      );
      if (analysisTab) {
        tabsActions.removeTab(analysisTab.id);
      }

      // If we were viewing this analysis, navigate away
      if (isCurrentlyViewing && projectId) {
        navigate(`/p/${projectId}`);
      }
    } catch (error) {
      console.error('Failed to delete analysis:', error);
      alert('Failed to delete analysis');
    }
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
                  activeAnalysisId === analysis.id && "bg-muted border-blue-700/30"
                )}
                onClick={() => onAnalysisClick(analysis.id)}
              >
                <div className="flex items-start gap-2 overflow-hidden group-hover:pr-8">
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
                </div>

                {/* Actions dropdown */}
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
                        onClick={(e) => handleDelete(analysis.id, e)}
                        className="text-destructive"
                      >
                        <Trash2 className="h-4 w-4 mr-2" />
                        Delete
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}