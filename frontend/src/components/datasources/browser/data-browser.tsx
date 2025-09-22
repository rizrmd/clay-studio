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
import { TabNavigation, TabNavigationRef } from "./components/tab-navigation";
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
  
  // Track initial load to prevent duplicate calls
  const hasLoadedStructure = useRef(false);
  const tabNavRef = useRef<TabNavigationRef>(null);
  const hasLoadedData = useRef(false);
  const isLoadingTimeout = useRef<NodeJS.Timeout | null>(null);

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

  // Handle select all rows across all pages  
  const handleSelectAllRows = useCallback(async () => {
    if (!dataBrowserSnapshot.selectedDatasourceId || !dataBrowserSnapshot.selectedTable) return;
    
    // Set loading state
    localStore.isSelectingAll = true;
    
    // Clear existing selections first
    localStore.selectedRows = {};
    
    try {
      // For now, use the existing table data endpoint to get all data and extract IDs
      // This is less efficient but will work reliably
      console.log('Fetching all table data for row IDs...');
      
      const result = await datasourcesApi.getTableData(
        dataBrowserSnapshot.selectedDatasourceId,
        dataBrowserSnapshot.selectedTable,
        {
          page: 1,
          limit: Math.min(dataBrowserSnapshot.totalRows, 10000), // Get all rows up to limit
          sort_column: undefined,
          sort_direction: undefined,
          filters: undefined
        }
      );
      
      console.log('Table data result:', result);
      
      // Extract row IDs from the actual data using the same logic as the table
      console.log('Processing data:', {
        hasData: !!(result as any).data,
        dataLength: (result as any).data?.length,
        hasRows: !!result.rows,
        rowsLength: result.rows?.length,
        hasColumns: !!result.columns,
        columnsLength: result.columns?.length,
        firstRow: (result as any).data?.[0] || result.rows?.[0],
        columns: result.columns
      });
      
      // Try both data and rows properties (API inconsistency)
      const dataArray = (result as any).data || result.rows;
      
      if (dataArray && Array.isArray(dataArray) && dataArray.length > 0) {
        dataArray.forEach((row: any, rowIndex: number) => {
          // Use the same ID extraction logic as the table transformation
          const firstColumnIndex = 0; // First column usually contains the ID
          const rowId = Array.isArray(row) ? row[firstColumnIndex] : row?.id || row?.[Object.keys(row)[0]];
          
          if (rowIndex < 3) { // Only log first 3 rows to avoid spam
            console.log(`Row ${rowIndex}: rowId = ${rowId}, row =`, row);
          }
          
          if (rowId != null) {
            localStore.selectedRows[String(rowId)] = true;
          }
        });
        
        console.log('Selected rows after processing:', Object.keys(localStore.selectedRows).slice(0, 10));
      } else {
        console.warn('No data to process or data not in expected format:', {
          result,
          hasData: !!(result as any).data,
          hasRows: !!result.rows,
          dataLength: (result as any).data?.length,
          rowsLength: result.rows?.length
        });
      }
      
      // Increment version to force re-render
      localStore.selectionVersion++;
      
      console.log('handleSelectAllRows SUCCESS:', {
        totalRows: dataBrowserSnapshot.totalRows,
        fetchedRows: (result as any).data?.length || result.rows?.length || 0,
        selectedCount: Object.keys(localStore.selectedRows).length,
        selectionVersion: localStore.selectionVersion,
        firstFewKeys: Object.keys(localStore.selectedRows).slice(0, 10),
        columns: result.columns
      });
    } catch (error) {
      console.error('Failed to fetch table data for selection:', error);
      // Fallback to current page selection
      const currentTableData = dataBrowserSnapshot.tableData;
      if (currentTableData?.rows) {
        currentTableData.rows.forEach((row: any) => {
          // Use the first column value as the ID
          const rowId = row[0];
          
          if (rowId != null) {
            localStore.selectedRows[String(rowId)] = true;
          }
        });
        
        localStore.selectionVersion++;
      }
    } finally {
      // Always clear loading state
      localStore.isSelectingAll = false;
    }
  }, [localStore, dataBrowserSnapshot.selectedDatasourceId, dataBrowserSnapshot.selectedTable, dataBrowserSnapshot.totalRows, dataBrowserSnapshot.tableData]);

  // Handle deselect all rows
  const handleDeselectAllRows = useCallback(() => {
    // Clear all selections by resetting the object
    localStore.selectedRows = {};
    
    // Increment version to force re-render
    localStore.selectionVersion++;
    
    console.log('Deselected all rows');
  }, [localStore]);

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
      // Reset data load flags when datasource changes
      hasLoadedStructure.current = false;
      hasLoadedData.current = false;
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
    if (dataBrowserSnapshot.selectedTable) {
      localStore.currentPage = 1;
      localStore.sorting = [];
      localStore.columnFilters = [];
      localStore.globalFilter = "";
      localStore.selectedRows = {};
      // Reset data load flags for new table
      hasLoadedStructure.current = false;
      hasLoadedData.current = false;
    }
  }, [dataBrowserSnapshot.selectedTable]);

  // Load structure and data in parallel when table is selected
  useEffect(() => {
    if (dataBrowserSnapshot.selectedTable && datasourceId && !hasLoadedData.current) {
      // Clear any existing timeout
      if (isLoadingTimeout.current) {
        clearTimeout(isLoadingTimeout.current);
      }
      
      // Set a timeout to prevent rapid duplicate calls
      isLoadingTimeout.current = setTimeout(() => {
        // Load structure in parallel
        if (!hasLoadedStructure.current) {
          dataBrowserActions.loadTableStructure();
          hasLoadedStructure.current = true;
        }
        
        // Load data
        updateStoreForServerSide();
        hasLoadedData.current = true;
        isLoadingTimeout.current = null;
      }, 100);
    }
  }, [dataBrowserSnapshot.selectedTable, datasourceId, updateStoreForServerSide]);

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
        // Skip if we haven't loaded data yet
        if (hasLoadedData.current) {
          updateStoreForServerSide();
        }
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
          ref={tabNavRef}
          localStore={localStore}
          localSnapshot={{
            ...localSnapshot,
            isSelectingAll: localSnapshot.isSelectingAll
          }}
          selectedTable={dataBrowserSnapshot.selectedTable || undefined}
          totalRows={dataBrowserSnapshot.totalRows}
          onDeleteSelectedRows={handleDeleteSelectedRows}
          onSelectAllRows={handleSelectAllRows}
          onDeselectAllRows={handleDeselectAllRows}
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
                        onAddNewRow={() => tabNavRef.current?.openAddRowModal()}
                        onServerSortingChange={handleServerSortingChange}
                        onServerFiltersChange={handleServerFiltersChange}
                        onServerGlobalFilterChange={handleServerGlobalFilterChange}
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
