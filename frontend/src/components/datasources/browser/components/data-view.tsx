import { useMemo, useCallback, useState } from "react";
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
import { CheckSquare, Square, Minus } from "lucide-react";

interface DataViewProps {
  localSnapshot: {
    currentPage: number;
    pageSize: number;
    sorting: readonly { readonly desc: boolean; readonly id: string; }[];
    columnFilters: readonly { readonly id: string; readonly value: unknown; }[];
    globalFilter: string;
    isServerLoading: boolean;
    selectedRows: Record<string, boolean>;
  };
  localStore: any;
  tableStructure: any;
  tableData: any;
  totalRows: number;
  onPageChange: (page: number) => void;
  onPageSizeChange: (newPageSize: number) => void;
  onServerSortingChange: (newSorting: any) => void;
  onServerFiltersChange: (newFilters: any) => void;
  onServerGlobalFilterChange: (newGlobalFilter: string) => void;
  onDeleteSelectedRows: () => void;
}

export const DataView = ({
  localSnapshot,
  localStore,
  tableStructure,
  tableData,
  totalRows,
  onPageChange,
  onPageSizeChange,
  onServerSortingChange,
  onServerFiltersChange,
  onServerGlobalFilterChange,
  onDeleteSelectedRows,
}: DataViewProps) => {
  const dataBrowserSnapshot = useSnapshot(dataBrowserStore);

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

    const { columns: dataColumns, rows } = tableData;
    const structureColumns = tableStructure.columns;

    // Transform existing data
    const transformedData = rows.map((row: any, rowIndex: number) => {
      const rowObject: Record<string, any> = { id: rowIndex };

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

      return rowObject;
    });

    // Add pending new rows at the beginning
    const newRows = dataBrowserSnapshot.pendingNewRows.map((newRow, index) => {
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
  }, [tableData, tableStructure, dataBrowserSnapshot.pendingNewRows]);

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
  const baseColumns = useMemo((): TableColumn[] => {
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
        const isAllSelected = allTableData.length > 0 && allTableData.every(row => localSnapshot.selectedRows[String(row.id)]);
        const isSomeSelected = allTableData.some(row => localSnapshot.selectedRows[String(row.id)]);
        
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
        const rowId = String(row.id);
        const isSelected = localSnapshot.selectedRows[rowId];
        
        // Get the current row index within the table data
        const rowIndexInCurrentPage = allTableData.findIndex(r => String(r.id) === rowId);
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
  }, [tableStructure, tableData, localSnapshot.selectedRows, handleRowSelect]);

  // Enhance columns with cell renderers for edit functionality
  const columns = useMemo(() => {
    return enhanceColumnsWithCellRenderers(baseColumns, {
      isEditable: true, // Always editable with click-to-edit
      onCellEdit: (rowId: string, columnKey: string, newValue: any) => {
        dataBrowserActions.setCellValue(rowId, columnKey, newValue);
      },
      editingDisabled: dataBrowserSnapshot.editingInProgress,
    });
  }, [baseColumns, dataBrowserSnapshot.editingInProgress]);

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
        <DataTable
          columns={columns}
          data={allTableData}
          customSelectedRows={localSnapshot.selectedRows}
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
      </div>
      <PaginationControls
        currentPage={localSnapshot.currentPage}
        pageSize={localSnapshot.pageSize}
        totalItems={totalRows || allTableData.length}
        onPageChange={onPageChange}
        onPageSizeChange={onPageSizeChange}
      />
    </div>
  );
};