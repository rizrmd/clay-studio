import { useEffect } from "react";
import { useSnapshot } from "valtio";
import { useSearchParams } from "react-router-dom";
import { X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { dataBrowserStore, dataBrowserActions } from "@/lib/store/data-browser-store";
import { datasourcesStore } from "@/lib/store/datasources-store";
import { DataGrid } from "./data-grid";
import { cn } from "@/lib/utils";

interface DataBrowserProps {
  datasourceId: string;
  onClose?: () => void;
  className?: string;
}

export function DataBrowser({ datasourceId, onClose, className }: DataBrowserProps) {
  const dataBrowserSnapshot = useSnapshot(dataBrowserStore);
  const datasourcesSnapshot = useSnapshot(datasourcesStore);
  const [searchParams] = useSearchParams();

  const selectedDatasource = datasourcesSnapshot.datasources.find(
    ds => ds.id === datasourceId
  );

  const tableFromUrl = searchParams.get('table');

  useEffect(() => {
    if (datasourceId !== dataBrowserSnapshot.selectedDatasourceId) {
      dataBrowserActions.selectDatasource(datasourceId);
    }
  }, [datasourceId, dataBrowserSnapshot.selectedDatasourceId]);

  // Handle table selection from URL parameter
  useEffect(() => {
    if (tableFromUrl && dataBrowserSnapshot.tables.length > 0) {
      const tableExists = dataBrowserSnapshot.tables.includes(tableFromUrl);
      if (tableExists && dataBrowserSnapshot.selectedTable !== tableFromUrl) {
        dataBrowserActions.selectTable(tableFromUrl);
      }
    }
  }, [tableFromUrl, dataBrowserSnapshot.tables, dataBrowserSnapshot.selectedTable]);

  const handleClose = () => {
    dataBrowserActions.reset();
    onClose?.();
  };

  if (!selectedDatasource) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <p className="text-muted-foreground">Datasource not found</p>
          {onClose && (
            <Button variant="outline" onClick={handleClose} className="mt-2">
              Close
            </Button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className={cn("flex flex-col h-full bg-background", className)}>
      {/* Error Display */}
      {dataBrowserSnapshot.error && (
        <div className="mx-4 mt-4 p-3 bg-red-50 border border-red-200 rounded-md">
          <div className="flex items-center justify-between">
            <p className="text-sm text-red-800">{dataBrowserSnapshot.error}</p>
            <Button
              variant="ghost"
              size="sm"
              onClick={dataBrowserActions.clearError}
              className="h-6 w-6 p-0"
            >
              <X className="h-3 w-3" />
            </Button>
          </div>
        </div>
      )}

      {/* Main Content */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Tab Navigation */}
        <div className="border-b">
          <div className="flex">
            <button
              className={cn(
                "px-4 py-2 text-sm font-medium border-b-2 transition-colors",
                dataBrowserSnapshot.selectedTable
                  ? "border-primary text-primary"
                  : "border-transparent text-muted-foreground hover:text-foreground"
              )}
              disabled={!dataBrowserSnapshot.selectedTable}
            >
              Data ({dataBrowserSnapshot.selectedTable || "Select table"})
            </button>
            <button
              className={cn(
                "px-4 py-2 text-sm font-medium border-b-2 transition-colors",
                "border-transparent text-muted-foreground hover:text-foreground"
              )}
            >
              Query
            </button>
          </div>
        </div>

        {/* Content Area */}
        <div className="flex-1 overflow-hidden">
          {dataBrowserSnapshot.selectedTable ? (
            <DataGrid
              data={dataBrowserSnapshot.tableData}
              loading={dataBrowserSnapshot.dataLoading}
              currentPage={dataBrowserSnapshot.currentPage}
              pageSize={dataBrowserSnapshot.pageSize}
              totalRows={dataBrowserSnapshot.totalRows}
              sortColumn={dataBrowserSnapshot.sortColumn}
              sortDirection={dataBrowserSnapshot.sortDirection}
              onPageChange={dataBrowserActions.setPage}
              onPageSizeChange={dataBrowserActions.setPageSize}
              onSort={dataBrowserActions.setSorting}
            />
          ) : (
            <div className="flex items-center justify-center h-full">
              <div className="text-center">
                <p className="text-muted-foreground">Select a table to view data</p>
                <p className="text-sm text-muted-foreground mt-1">
                  Choose a table from the sidebar to view its data
                </p>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}