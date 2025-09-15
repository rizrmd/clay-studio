import { useEffect, useRef, useCallback } from "react";
import { useSnapshot } from "valtio";
import { useSearchParams, useParams } from "react-router-dom";
import { X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { datasourcesApi } from "@/lib/api/datasources";
import {
  dataBrowserStore,
  dataBrowserActions,
} from "@/lib/store/data-browser-store";
import {
  datasourcesStore,
  datasourcesActions,
} from "@/lib/store/datasources-store";
import { TableStructureView } from "./table-structure";
import { cn } from "@/lib/utils";
import { useLocalStore } from "./hooks/use-local-store";
import { useServerHandlers } from "./hooks/use-server-handlers";
import { TabNavigation } from "./components/tab-navigation";
import { DataView } from "./components/data-view";
import { EditToolbar } from "./components/edit-toolbar";
import {
  DatasourcesLoadingState,
  DatasourcesErrorState,
  DatasourceNotFoundState,
  StructureLoadingState,
  NoTableSelectedState,
} from "./components/loading-states";

interface DataBrowserProps {
  datasourceId: string;
  onClose?: () => void;
  className?: string;
  mode?: "data" | "structure";
}


export function DataBrowser({
  datasourceId,
  onClose,
  className,
  mode = "data",
}: DataBrowserProps) {
  // Create local store for component state
  const { localStore, localSnapshot } = useLocalStore(mode);

  // Global stores
  const dataBrowserSnapshot = useSnapshot(dataBrowserStore);
  const datasourcesSnapshot = useSnapshot(datasourcesStore);
  const [searchParams] = useSearchParams();
  const { projectId } = useParams<{ projectId: string }>();

  const selectedDatasource = datasourcesSnapshot.datasources.find(
    (ds) => ds.id === datasourceId
  );

  const tableFromUrl = searchParams.get("table");

  // Handle delete selected rows
  const handleDeleteSelectedRows = useCallback(async () => {
    const selectedRowIds = Object.keys(localSnapshot.selectedRows);
    if (selectedRowIds.length === 0) return;
    
    try {
      // Call the delete API
      const result = await datasourcesApi.deleteRows(
        dataBrowserSnapshot.selectedDatasourceId!,
        dataBrowserSnapshot.selectedTable!,
        {
          row_ids: selectedRowIds,
          id_column: "id" // Assuming 'id' is the primary key column
        }
      );
      
      console.log(`Successfully deleted ${result.rows_affected} rows`);
      
      // Clear selection after successful deletion
      localStore.selectedRows = {};
      
      // Refresh the table data by triggering a reload
      // This will be handled by the effect that watches for changes
      // Force a reload by briefly setting loading state
      localStore.isServerLoading = true;
      setTimeout(() => {
        localStore.isServerLoading = false;
      }, 100);
      
    } catch (error) {
      console.error('Failed to delete rows:', error);
      // TODO: Show proper error message to user
    }
  }, [localSnapshot.selectedRows, localStore, dataBrowserSnapshot.selectedDatasourceId, dataBrowserSnapshot.selectedTable]);

  // Server-side handlers
  const {
    updateStoreForServerSide,
    handleServerSortingChange,
    handleServerFiltersChange,
    handleServerGlobalFilterChange,
    handlePageChange,
    handlePageSizeChange,
  } = useServerHandlers(localStore, datasourceId);

  // Load datasources if they haven't been loaded yet
  useEffect(() => {
    if (
      projectId &&
      datasourcesSnapshot.datasources.length === 0 &&
      !datasourcesSnapshot.isLoading
    ) {
      datasourcesActions.loadDatasources(projectId);
    }
  }, [
    projectId,
    datasourcesSnapshot.datasources.length,
    datasourcesSnapshot.isLoading,
  ]);

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
  }, [
    tableFromUrl,
    dataBrowserSnapshot.tables,
    dataBrowserSnapshot.selectedTable,
  ]);


  // Note: Client-side pagination removed since we're using server-side pagination
  // Data is already paginated from the server

  // Reset to first page when table changes
  useEffect(() => {
    localStore.currentPage = 1;
    localStore.sorting = [];
    localStore.columnFilters = [];
    localStore.globalFilter = "";
    localStore.selectedRows = {};
  }, [dataBrowserSnapshot.selectedTable, localStore]);

  // Debounced effect for filter changes
  const filterTimeoutRef = useRef<NodeJS.Timeout>();

  useEffect(() => {
    if (dataBrowserSnapshot.selectedTable && datasourceId) {
      // Clear existing timeout
      if (filterTimeoutRef.current) {
        clearTimeout(filterTimeoutRef.current);
      }

      // For filters, add a small debounce delay
      if (
        localSnapshot.columnFilters.length > 0 ||
        localSnapshot.globalFilter
      ) {
        filterTimeoutRef.current = setTimeout(() => {
          updateStoreForServerSide();
        }, 300); // 300ms debounce for filters
      } else {
        // Immediate update for non-filter changes (sorting, pagination)
        updateStoreForServerSide();
      }
    }

    // Cleanup timeout on unmount
    return () => {
      if (filterTimeoutRef.current) {
        clearTimeout(filterTimeoutRef.current);
      }
    };
  }, [
    dataBrowserSnapshot.selectedTable,
    datasourceId,
    localSnapshot.currentPage,
    localSnapshot.pageSize,
    localSnapshot.sorting,
    localSnapshot.columnFilters,
    localSnapshot.globalFilter,
    updateStoreForServerSide,
  ]);


  // Note: Column resizing is now handled by the DataTable component internally

  const handleClose = () => {
    dataBrowserActions.reset();
    onClose?.();
  };

  // Show loading state while datasources are being loaded
  if (datasourcesSnapshot.isLoading) {
    return <DatasourcesLoadingState onClose={handleClose} />;
  }

  // Show error state if failed to load datasources
  if (
    datasourcesSnapshot.error &&
    datasourcesSnapshot.datasources.length === 0
  ) {
    return (
      <DatasourcesErrorState error={datasourcesSnapshot.error} onClose={handleClose} />
    );
  }

  // Show not found only if datasources are loaded but this specific one doesn't exist
  if (!selectedDatasource) {
    return <DatasourceNotFoundState onClose={handleClose} />;
  }

  return (
    <div className={cn("flex flex-col h-full bg-background", className)}>
      {/* Error Display */}

      {/* Main Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Tab Navigation */}
        <TabNavigation
          localStore={localStore}
          localSnapshot={localSnapshot}
          selectedTable={dataBrowserSnapshot.selectedTable || undefined}
          onDeleteSelectedRows={handleDeleteSelectedRows}
        />

        {/* Content Area */}
        <div className="flex-1 overflow-hidden">
          {dataBrowserSnapshot.error && (
            <div className="mx-4 mt-4 p-3 bg-red-50 border border-red-200 rounded-md">
              <div className="flex items-center justify-between">
                <p className="text-sm text-red-800">
                  {dataBrowserSnapshot.error}
                </p>
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
          {dataBrowserSnapshot.selectedTable ? (
            localSnapshot.currentMode === "data" ? (
              <>
                {dataBrowserSnapshot.structureLoading ? (
                  <StructureLoadingState />
                ) : (
                  <div className="flex flex-col h-full">
                    <EditToolbar />
                    <div className="flex-1 overflow-hidden">
                      <DataView
                        localSnapshot={localSnapshot}
                        localStore={localStore}
                        tableStructure={dataBrowserSnapshot.tableStructure}
                        tableData={dataBrowserSnapshot.tableData}
                        totalRows={dataBrowserSnapshot.totalRows}
                        onPageChange={handlePageChange}
                        onPageSizeChange={handlePageSizeChange}
                        onServerSortingChange={handleServerSortingChange}
                        onServerFiltersChange={handleServerFiltersChange}
                        onServerGlobalFilterChange={handleServerGlobalFilterChange}
                        onDeleteSelectedRows={handleDeleteSelectedRows}
                      />
                    </div>
                  </div>
                )}
              </>
            ) : // Structure view - show table schema/columns
            dataBrowserSnapshot.tableStructure ? (
              <TableStructureView
                structure={dataBrowserSnapshot.tableStructure}
                loading={dataBrowserSnapshot.structureLoading}
              />
            ) : (
              <div className="flex items-center justify-center h-full">
                <div className="text-center">
                  <p className="text-muted-foreground">
                    {dataBrowserSnapshot.structureLoading
                      ? "Loading table structure..."
                      : "No structure information available"}
                  </p>
                </div>
              </div>
            )
          ) : (
            <NoTableSelectedState currentMode={localSnapshot.currentMode} />
          )}
        </div>
      </div>
    </div>
  );
}
