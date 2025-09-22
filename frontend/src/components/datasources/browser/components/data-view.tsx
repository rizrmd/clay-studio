import React, { useMemo, useCallback } from "react";
import { useSnapshot } from "valtio";
import { cn } from "@/lib/utils";
import { DataTable } from "@/components/data-table/data-table";
import { TableColumn } from "@/components/data-table/demo-data";
import { css } from "goober";
import { PaginationControls } from "../pagination-controls";
import { mapDbTypeToDisplayType, getColumnWidth, analyzeColumnContent } from "../utils/column-utils";
import { enhanceColumnsWithCellRenderers } from "../utils/cell-renderers";
import { dataBrowserStore, dataBrowserActions } from "@/lib/store/data-browser-store";
import { datasourcesApi } from "@/lib/api/datasources";
import { CheckSquare, Square, Minus, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";

interface DataViewProps {
  localSnapshot: {
    currentPage: number;
    pageSize: number;
    sorting: readonly { readonly desc: boolean; readonly id: string; }[];
    columnFilters: readonly { readonly id: string; readonly value: unknown; }[];
    globalFilter: string;
    isServerLoading: boolean;
    selectedRows: Record<string, boolean>;
    selectionVersion: number;
  };
  localStore: any;
  tableStructure: any;
  tableData: any;
  totalRows: number;
  onPageChange: (page: number) => void;
  onPageSizeChange: (newPageSize: number) => void;
  onAddNewRow: () => void;
  onServerSortingChange: (newSorting: any) => void;
  onServerFiltersChange: (newFilters: any) => void;
  onServerGlobalFilterChange: (newGlobalFilter: string) => void;
}

export const DataView = ({
  localSnapshot,
  localStore,
  tableStructure,
  tableData,
  totalRows,
  onPageChange,
  onPageSizeChange,
  onAddNewRow,
  onServerSortingChange,
  onServerFiltersChange,
  onServerGlobalFilterChange,
}: DataViewProps) => {
  const dataBrowserSnapshot = useSnapshot(dataBrowserStore);
  
  // Create a local snapshot to ensure reactivity
  const localStoreSnapshot = useSnapshot(localStore);
  
  // Debug: Log when selectedRows changes
  React.useEffect(() => {
    console.log('DataView: selectedRows changed:', {
      count: Object.keys(localStoreSnapshot.selectedRows).length,
      keys: Object.keys(localStoreSnapshot.selectedRows).slice(0, 5),
      version: localStoreSnapshot.selectionVersion
    });
  }, [localStoreSnapshot.selectedRows, localStoreSnapshot.selectionVersion]);

  // Row selection handlers
  const handleRowSelect = useCallback((rowId: string, isSelected: boolean) => {
    if (isSelected) {
      localStore.selectedRows[rowId] = true;
    } else {
      delete localStore.selectedRows[rowId];
    }
  }, [localStore]);

  // Transform data for DataTable using structure column order
  const allTableData = useMemo(() => {
    if (!tableData || !tableStructure) {
      return [];
    }

    const { columns: dataColumns, data: rows = [] } = tableData || {};
    const structureColumns = tableStructure.columns;

    // Transform existing data
    const transformedData = rows.map((row: any, rowIndex: number) => {
      // Calculate the absolute row index accounting for pagination
      const absoluteRowIndex = ((localSnapshot.currentPage - 1) * localSnapshot.pageSize) + rowIndex;
      const rowObject: Record<string, any> = { 
        __rowIndex: absoluteRowIndex // Use a different property for our sequential index
      };
      
      // Map data based on structure column order, not data column order
      structureColumns.forEach((structureColumn: any) => {
        // Find the corresponding index in the data columns
        const dataColumnIndex = dataColumns.indexOf(structureColumn.name);
        if (dataColumnIndex !== -1) {
          rowObject[structureColumn.name] = row[dataColumnIndex] ?? null;
        } else {
          // Column exists in structure but not in data (possibly empty column)
          rowObject[structureColumn.name] = null;
        }
      });

      // Set the row ID - use the database ID if available, otherwise use our sequential index
      const dbIdColumn = structureColumns.find((col: any) => col.name === 'id');
      if (dbIdColumn && rowObject.id) {
        // Use the actual database ID
        rowObject.id = rowObject.id;
      } else {
        // Use our sequential index as fallback
        rowObject.id = absoluteRowIndex;
      }

      return rowObject;
    });

    // Add pending new rows at the beginning
    const newRows = (dataBrowserSnapshot.pendingNewRows || []).map((newRow, index) => {
      const rowObject: Record<string, any> = { 
        id: newRow.__tempId,
        __isNewRow: true,
        __newRowIndex: index
      };

      // Map new row data to match structure
      structureColumns.forEach((structureColumn: any) => {
        rowObject[structureColumn.name] = newRow[structureColumn.name] ?? null;
      });

      return rowObject;
    });

    return [...newRows, ...transformedData];
  }, [tableData, tableStructure, dataBrowserSnapshot.pendingNewRows, localSnapshot.currentPage, localSnapshot.pageSize]);

  const handleSelectAll = useCallback((isSelected: boolean) => {
    if (isSelected) {
      // Select all rows on current page
      allTableData.forEach(row => {
        localStore.selectedRows[String(row.id)] = true;
      });
    } else {
      // Deselect all rows on current page
      allTableData.forEach(row => {
        delete localStore.selectedRows[String(row.id)];
      });
    }
  }, [localStore, allTableData]);


  // Server-side distinct values handler
  const handleGetDistinctValues = useCallback(
    async (column: string, search?: string): Promise<string[]> => {
      if (!dataBrowserSnapshot.selectedDatasourceId || !dataBrowserSnapshot.selectedTable) {
        return [];
      }

      try {
        const result = await datasourcesApi.getDistinctValues(
          dataBrowserSnapshot.selectedDatasourceId,
          dataBrowserSnapshot.selectedTable,
          {
            column,
            search,
            limit: 100, // Limit to 100 distinct values
          }
        );
        return result.values as string[];
      } catch (error) {
        console.error("Failed to fetch distinct values:", error);
        return [];
      }
    },
    [dataBrowserSnapshot.selectedDatasourceId, dataBrowserSnapshot.selectedTable]
  );

  // Generate columns for DataTable using table structure as source of truth
  // Force recreation when selection changes - include selectionVersion to ensure updates
  const baseColumns = React.useMemo((): TableColumn[] => {
    if (!tableStructure?.columns.length) {
      return [];
    }

    const rows = tableData?.rows ?? [];

    // Create row index column with hover checkbox functionality
    const rowIndexColumn: TableColumn = {
      key: '__row_index',
      label: '#',
      data_type: 'string',
      width: 60,
      sortable: false,
      filterable: false,
      headerRenderer: () => {
        const isAllSelected = allTableData.length > 0 && allTableData.every(row => Boolean(localStoreSnapshot.selectedRows[String(row.id)]));
        const isSomeSelected = allTableData.some(row => Boolean(localStoreSnapshot.selectedRows[String(row.id)]));
        
        return (
          <div 
            className="flex items-center justify-center w-full h-full cursor-pointer"
            onClick={() => handleSelectAll(!isAllSelected)}
          >
            {isSomeSelected && !isAllSelected ? (
              <Minus className="h-4 w-4 text-primary" />
            ) : isAllSelected ? (
              <CheckSquare className="h-4 w-4 text-primary" />
            ) : (
              <Square className="h-4 w-4 text-muted-foreground hover:text-primary" />
            )}
          </div>
        );
      },
      cellRenderer: (_value: any, row: any, _column: any, _defaultRenderer: any) => {
        // row object structure in DataTable has the actual data in row, not row.original
        const rowId = String(row.id !== undefined ? row.id : row._id);
        // Use local snapshot created in this component
        const isSelected = Boolean(localStoreSnapshot.selectedRows[rowId]);
        
        // Get the current row index within the table data
        const rowIndexInCurrentPage = allTableData.findIndex(r => String(r.id) === rowId);
        
        // Check selection state for this row
        if (rowIndexInCurrentPage < 3) { // Only log first 3 rows to avoid spam
          console.log('cellRenderer row:', rowId, 'isSelected:', isSelected, 'has key:', String(rowId) in localStoreSnapshot.selectedRows);
        }
        // Calculate the actual row index accounting for pagination
        const actualRowIndex = ((localSnapshot.currentPage - 1) * localSnapshot.pageSize) + rowIndexInCurrentPage + 1;
        
        return (
          <div 
            className="flex items-center justify-center w-full h-full"
          >
            <span className={cn(
              "text-xs text-muted-foreground row-hover:hidden cursor-pointer",
              isSelected && "hidden"
            )}
            onClick={() => handleRowSelect(rowId, true)}
            >
              {actualRowIndex}
            </span>
            <div
              className={cn(
                "hidden row-hover:flex items-center justify-center cursor-pointer",
                isSelected && "flex row-hover:flex"
              )}
              onClick={() => handleRowSelect(rowId, !isSelected)}
            >
              {isSelected ? (
                <CheckSquare className="h-4 w-4 text-primary" />
              ) : (
                <Square className="h-4 w-4 text-muted-foreground hover:text-primary" />
              )}
            </div>
          </div>
        );
      }
    };

    const dataColumns = tableStructure.columns.map((structureColumn: any) => {
      const calculatedWidth = getColumnWidth(structureColumn.name);
      const contentAnalysis = analyzeColumnContent(
        structureColumn.name,
        rows as any
      );

      // Use structure data type as primary, fallback to content analysis
      const dataType =
        mapDbTypeToDisplayType(structureColumn.data_type) ||
        contentAnalysis.dataType;

      // Create label with required marker if not nullable
      const label = structureColumn.nullable === false 
        ? `${structureColumn.name} *`
        : structureColumn.name;

      return {
        key: structureColumn.name,
        label: label,
        data_type: dataType,
        width: calculatedWidth,
        sortable: true,
        filterable: true,
        nullable: structureColumn.is_nullable,
      } as TableColumn;
    });
    
    return [rowIndexColumn, ...dataColumns];
  }, [tableStructure, tableData, localStoreSnapshot.selectedRows, localStoreSnapshot.selectionVersion, handleRowSelect, handleSelectAll, allTableData, localSnapshot.currentPage, localSnapshot.pageSize]);

  // Enhance columns with cell renderers for edit functionality
  // Re-create columns when selection changes to ensure cellRenderers have latest state
  const columns = React.useMemo(() => enhanceColumnsWithCellRenderers(baseColumns, {
    isEditable: true, // Always editable with click-to-edit
    onCellEdit: (rowId: string, columnKey: string, newValue: any) => {
      dataBrowserActions.setCellValue(rowId, columnKey, newValue);
    },
    editingDisabled: dataBrowserSnapshot.editingInProgress,
  }), [baseColumns, dataBrowserSnapshot.editingInProgress]);

  if (!tableStructure) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <p className="text-muted-foreground">No table structure loaded</p>
          <p className="text-sm text-muted-foreground mt-1">
            Unable to load table schema information
          </p>
        </div>
      </div>
    );
  }

  if (columns.length === 0) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <p className="text-muted-foreground">Table has no columns</p>
          <p className="text-sm text-muted-foreground mt-1">
            This table appears to have no defined schema
          </p>
        </div>
      </div>
    );
  }

  if (!tableData && !localSnapshot.isServerLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-muted-foreground">No table data loaded</p>
      </div>
    );
  }

  // Check if data is empty
  const hasData = allTableData.length > 0;

  return (
    <div className="h-full flex flex-col">
      <div className="flex-1 overflow-hidden relative">
        {localSnapshot.isServerLoading && (
          <div className="absolute inset-0 bg-background/80 backdrop-blur-sm z-50 top-[40px] border-b bottom-[1px] flex items-center justify-center">
            <div className="flex items-center gap-2">
              <div className="animate-spin h-4 w-4 border-2 border-primary border-t-transparent rounded-full"></div>
              <span className="text-sm text-muted-foreground">Loading...</span>
            </div>
          </div>
        )}
        {hasData ? (
          <DataTable
            columns={[...columns]}
            data={allTableData}
            customSelectedRows={localStoreSnapshot.selectedRows}
            persistenceKey={`datasource-${dataBrowserSnapshot.selectedDatasourceId}-table-${dataBrowserSnapshot.selectedTable}`}
            config={{
              features: {
                sort: true,
                filter: true,
                globalSearch: true,
                columnVisibility: true,
                rowSelection: false,
                pivot: false,
              },
              initialState: {
                columnVisibility: {},
                sorting: localSnapshot.sorting.map((sort) => ({
                  column: sort.id,
                  direction: sort.desc ? ("desc" as const) : ("asc" as const),
                })),
                filters: localSnapshot.columnFilters.map((filter) => ({
                  column: filter.id,
                  value: filter.value,
                })),
                globalFilter: localSnapshot.globalFilter,
              },
            }}
            serverSide={{
              enabled: true,
              onSortingChange: onServerSortingChange,
              onFiltersChange: onServerFiltersChange,
              onGlobalFilterChange: onServerGlobalFilterChange,
              onGetDistinctValues: handleGetDistinctValues,
              totalRows: totalRows,
            }}
            className={cn(
              "h-full -ml-[1px] -mt-[1px]",
              css`
                td {
                  white-space: nowrap !important;
                  overflow: hidden !important;
                  text-overflow: ellipsis !important;
                  max-width: 200px !important;
                }
                th {
                  white-space: nowrap !important;
                  overflow: hidden !important;
                  text-overflow: ellipsis !important;
                  padding: 0 !important;

                  .th-l2 > div {
                    height: 40px;
                    > .button {
                      height: 40px;
                    }
                  }
                }
                table {
                  table-layout: fixed !important;
                }
                tr {
                  height: 40px !important;
                }
                tbody tr:hover .row-hover\\:hidden {
                  display: none !important;
                }
                tbody tr:hover .row-hover\\:flex {
                  display: flex !important;
                }
                tbody tr[data-selected="true"] {
                  background-color: rgb(239 246 255) !important;
                }
                tbody tr[data-selected="true"]:hover {
                  background-color: rgb(219 234 254) !important;
                }
                @media (prefers-color-scheme: dark) {
                  tbody tr[data-selected="true"] {
                    background-color: rgb(30 58 138 / 0.3) !important;
                  }
                  tbody tr[data-selected="true"]:hover {
                    background-color: rgb(30 58 138 / 0.5) !important;
                  }
                }
              `
            )}
          />
        ) : (
          <div className="flex items-center justify-center h-full">
            <div className="text-center space-y-4">
              <div>
                <p className="text-muted-foreground">No data available</p>
                <p className="text-sm text-muted-foreground mt-1">
                  This table is empty
                </p>
              </div>
              <Button
                onClick={onAddNewRow}
                className="gap-2"
                variant="default"
              >
                <Plus className="h-4 w-4" />
                Add New Row
              </Button>
            </div>
          </div>
        )}
      </div>
      {hasData && (
        <PaginationControls
          currentPage={localSnapshot.currentPage}
          pageSize={localSnapshot.pageSize}
          totalItems={totalRows || allTableData.length}
          onPageChange={onPageChange}
          onPageSizeChange={onPageSizeChange}
        />
      )}
    </div>
  );
};