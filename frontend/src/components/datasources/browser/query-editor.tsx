import { useState } from "react";
import { Play, History, Download } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { useSnapshot } from "valtio";
import { dataBrowserStore, dataBrowserActions } from "@/lib/store/data-browser-store";
import { DataGrid } from "./data-grid";

interface QueryEditorProps {
  className?: string;
}

export function QueryEditor({ className }: QueryEditorProps) {
  const dataBrowserSnapshot = useSnapshot(dataBrowserStore);
  const [localQuery, setLocalQuery] = useState(dataBrowserSnapshot.currentQuery);

  const handleQueryChange = (value: string) => {
    setLocalQuery(value);
    dataBrowserActions.setQuery(value);
  };

  const handleExecute = () => {
    if (!localQuery.trim()) return;
    dataBrowserActions.executeQuery();
  };

  const handleHistorySelect = (query: string) => {
    handleQueryChange(query);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      handleExecute();
    }
  };

  return (
    <div className={`flex flex-col h-full ${className}`}>
      {/* Query Input */}
      <div className="border-b p-4">
        <div className="flex items-start gap-2">
          <div className="flex-1">
            <Textarea
              value={localQuery}
              onChange={(e) => handleQueryChange(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Enter your SQL query here...&#10;Tip: Press Ctrl+Enter (or Cmd+Enter) to execute"
              className="min-h-[120px] font-mono text-sm resize-none"
            />
          </div>
          
          <div className="flex flex-col gap-2">
            <Button
              onClick={handleExecute}
              disabled={!localQuery.trim() || dataBrowserSnapshot.queryLoading}
              size="sm"
              className="whitespace-nowrap"
            >
              <Play className="h-4 w-4 mr-1" />
              {dataBrowserSnapshot.queryLoading ? "Running..." : "Execute"}
            </Button>

            {dataBrowserSnapshot.queryHistory.length > 0 && (
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="outline" size="sm">
                    <History className="h-4 w-4 mr-1" />
                    History
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-80">
                  {dataBrowserSnapshot.queryHistory.map((query, index) => (
                    <DropdownMenuItem
                      key={index}
                      onClick={() => handleHistorySelect(query)}
                      className="font-mono text-xs"
                    >
                      <div className="truncate" title={query}>
                        {query.length > 60 ? `${query.substring(0, 60)}...` : query}
                      </div>
                    </DropdownMenuItem>
                  ))}
                </DropdownMenuContent>
              </DropdownMenu>
            )}
          </div>
        </div>

        {/* Quick Info */}
        <div className="mt-2 text-xs text-muted-foreground">
          <span>Press Ctrl+Enter (or Cmd+Enter) to execute query</span>
          {dataBrowserSnapshot.selectedDatasourceId && (
            <span className="ml-4">
              Connected to: {dataBrowserSnapshot.selectedDatasourceId}
            </span>
          )}
        </div>
      </div>

      {/* Results */}
      <div className="flex-1 overflow-hidden">
        {dataBrowserSnapshot.queryLoading ? (
          <div className="flex items-center justify-center h-full">
            <div className="text-center">
              <div className="animate-spin h-6 w-6 border-2 border-primary border-t-transparent rounded-full mx-auto mb-2" />
              <p className="text-sm text-muted-foreground">Executing query...</p>
            </div>
          </div>
        ) : dataBrowserSnapshot.queryResults ? (
          <div className="h-full">
            <div className="border-b px-4 py-2 bg-muted/50">
              <div className="flex items-center justify-between">
                <h4 className="text-sm font-medium">Query Results</h4>
                <div className="flex items-center gap-2">
                  <span className="text-xs text-muted-foreground">
                    {dataBrowserSnapshot.queryResults.row_count} rows
                  </span>
                  <Button variant="outline" size="sm" className="h-7">
                    <Download className="h-3 w-3 mr-1" />
                    Export
                  </Button>
                </div>
              </div>
            </div>
            <div className="h-[calc(100%-3rem)]">
              <DataGrid
                data={dataBrowserSnapshot.queryResults}
                loading={false}
                currentPage={1}
                pageSize={dataBrowserSnapshot.queryResults.row_count}
                totalRows={dataBrowserSnapshot.queryResults.row_count}
                sortColumn={null}
                sortDirection="asc"
                onPageChange={() => {}}
                onPageSizeChange={() => {}}
                onSort={() => {}}
              />
            </div>
          </div>
        ) : (
          <div className="flex items-center justify-center h-full">
            <div className="text-center">
              <p className="text-muted-foreground">Enter a query and click Execute to see results</p>
              <p className="text-sm text-muted-foreground mt-1">
                Or use Ctrl+Enter (Cmd+Enter) to run
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}