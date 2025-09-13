import { useEffect } from "react";
import { useSnapshot } from "valtio";
import { X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { dataBrowserStore, dataBrowserActions } from "@/lib/store/data-browser-store";
import { datasourcesStore } from "@/lib/store/datasources-store";
import { TableList } from "./table-list";
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

  const selectedDatasource = datasourcesSnapshot.datasources.find(
    ds => ds.id === datasourceId
  );

  useEffect(() => {
    if (datasourceId !== dataBrowserSnapshot.selectedDatasourceId) {
      dataBrowserActions.selectDatasource(datasourceId);
    }
  }, [datasourceId, dataBrowserSnapshot.selectedDatasourceId]);

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
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b">
        <div className="flex items-center gap-3">
          <div>
            <h1 className="text-lg font-semibold">{selectedDatasource.name}</h1>
            <p className="text-sm text-muted-foreground">
              {selectedDatasource.source_type.charAt(0).toUpperCase() + 
               selectedDatasource.source_type.slice(1)} Database
            </p>
          </div>
        </div>
        
        {onClose && (
          <Button
            variant="ghost"
            size="sm"
            onClick={handleClose}
            className="h-8 w-8 p-0"
          >
            <X className="h-4 w-4" />
          </Button>
        )}
      </div>

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
      <div className="flex flex-1 overflow-hidden">
        {/* Left Sidebar - Tables */}
        <div className="w-64 border-r flex flex-col">
          <div className="p-3 border-b">
            <h3 className="text-sm font-medium">Tables</h3>
            {dataBrowserSnapshot.tablesLoading && (
              <p className="text-xs text-muted-foreground mt-1">Loading...</p>
            )}
            {!dataBrowserSnapshot.tablesLoading && dataBrowserSnapshot.tables.length === 0 && (
              <p className="text-xs text-muted-foreground mt-1">No tables found</p>
            )}
          </div>
          <div className="flex-1 overflow-y-auto">
            <TableList
              tables={dataBrowserSnapshot.tables}
              selectedTable={dataBrowserSnapshot.selectedTable}
              onTableSelect={dataBrowserActions.selectTable}
              loading={dataBrowserSnapshot.tablesLoading}
            />
          </div>
        </div>

        {/* Right Content Area */}
        <div className="flex-1 flex flex-col">
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
                    Choose from the tables list on the left
                  </p>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}