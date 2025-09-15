import { useEffect, useMemo } from "react";
import { Play, History, Download } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { useSnapshot } from "valtio";
import { queryEditorStore, queryEditorActions } from "@/lib/store/query-editor-store";
import { DataSheetGrid, Column, textColumn, keyColumn } from "react-datasheet-grid";

interface QueryEditorProps {
  className?: string;
  datasourceId?: string;
}

export function QueryEditor({ className, datasourceId }: QueryEditorProps) {
  const queryEditorSnapshot = useSnapshot(queryEditorStore);

  // Set datasourceId when provided
  useEffect(() => {
    if (datasourceId) {
      queryEditorActions.setDatasourceId(datasourceId);
    }
  }, [datasourceId]);

  // Generate columns for DataSheetGrid
  const columns = useMemo((): Column[] => {
    if (!queryEditorSnapshot.queryResults || !queryEditorSnapshot.queryResults.columns.length) {
      return [];
    }

    return queryEditorSnapshot.queryResults.columns.map((columnName) => ({
      ...keyColumn(columnName, textColumn),
      title: columnName,
    }));
  }, [queryEditorSnapshot.queryResults]);

  // Transform data for DataSheetGrid
  const gridData = useMemo(() => {
    if (!queryEditorSnapshot.queryResults) return [];
    
    const { columns, rows } = queryEditorSnapshot.queryResults;
    return rows.map(row => {
      const rowObject: Record<string, any> = {};
      columns.forEach((columnName, index) => {
        rowObject[columnName] = row[index];
      });
      return rowObject;
    });
  }, [queryEditorSnapshot.queryResults]);

  const handleQueryChange = (value: string) => {
    queryEditorActions.setQuery(value);
  };

  const handleExecute = () => {
    if (!queryEditorSnapshot.query.trim()) return;
    queryEditorActions.executeQuery();
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
              value={queryEditorSnapshot.query}
              onChange={(e) => handleQueryChange(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Enter your SQL query here...&#10;Tip: Press Ctrl+Enter (or Cmd+Enter) to execute"
              className="min-h-[120px] font-mono text-sm resize-none"
            />
          </div>
          
          <div className="flex flex-col gap-2">
            <Button
              onClick={handleExecute}
              disabled={!queryEditorSnapshot.query.trim() || queryEditorSnapshot.queryLoading}
              size="sm"
              className="whitespace-nowrap"
            >
              <Play className="h-4 w-4 mr-1" />
              {queryEditorSnapshot.queryLoading ? "Running..." : "Execute"}
            </Button>

            {queryEditorSnapshot.queryHistory.length > 0 && (
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="outline" size="sm">
                    <History className="h-4 w-4 mr-1" />
                    History
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-80">
                  {queryEditorSnapshot.queryHistory.map((query, index) => (
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
          {queryEditorSnapshot.datasourceId && (
            <span className="ml-4">
              Connected to: {queryEditorSnapshot.datasourceId}
            </span>
          )}
        </div>
      </div>

      {/* Results */}
      <div className="flex-1 overflow-hidden">
        {queryEditorSnapshot.error ? (
          <div className="flex items-center justify-center h-full">
            <div className="text-center p-4">
              <div className="bg-red-50 border border-red-200 rounded-lg p-4 max-w-lg">
                <h4 className="text-sm font-medium text-red-800 mb-2">Query Error</h4>
                <p className="text-sm text-red-700">{queryEditorSnapshot.error}</p>
                <Button 
                  variant="outline" 
                  size="sm" 
                  className="mt-2" 
                  onClick={() => queryEditorActions.clearError()}
                >
                  Dismiss
                </Button>
              </div>
            </div>
          </div>
        ) : queryEditorSnapshot.queryLoading ? (
          <div className="flex items-center justify-center h-full">
            <div className="text-center">
              <div className="animate-spin h-6 w-6 border-2 border-primary border-t-transparent rounded-full mx-auto mb-2" />
              <p className="text-sm text-muted-foreground">Executing query...</p>
            </div>
          </div>
        ) : queryEditorSnapshot.queryResults ? (
          <div className="h-full">
            <div className="border-b px-4 py-2 bg-muted/50">
              <div className="flex items-center justify-between">
                <h4 className="text-sm font-medium">Query Results</h4>
                <div className="flex items-center gap-2">
                  <span className="text-xs text-muted-foreground">
                    {queryEditorSnapshot.queryResults.row_count} rows (unlimited)
                  </span>
                  <Button variant="outline" size="sm" className="h-7">
                    <Download className="h-3 w-3 mr-1" />
                    Export
                  </Button>
                </div>
              </div>
            </div>
            <div className="h-[calc(100%-3rem)] p-4">
              {gridData.length > 0 ? (
                <DataSheetGrid
                  value={gridData}
                  onChange={() => {}} // Read-only for query results
                  columns={columns}
                  height={500}
                  className="border rounded-md"
                />
              ) : (
                <div className="flex items-center justify-center h-full">
                  <p className="text-muted-foreground">No results to display</p>
                </div>
              )}
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