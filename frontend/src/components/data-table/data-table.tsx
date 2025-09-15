"use client";

import * as React from "react";
import { useState, useMemo, useCallback, useRef } from "react";
import {
  ColumnDef,
  ColumnFiltersState,
  ColumnSizingState,
  SortingState,
  VisibilityState,
  getCoreRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  useReactTable,
} from "@tanstack/react-table";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Checkbox } from "@/components/ui/checkbox";
import { DataTableColumnHeader } from "./components/column-header";
import { TableHeader } from "./components/table-header";
import { TableBody } from "./components/table-body";
import { TableFooter } from "./components/table-footer";
import { cn } from "@/lib/utils";
import type { DataTableProps } from "./types";
import { formatCellValue } from "./utils/formatters";
import { processPivotData } from "./lib/pivot";
import { fuzzyFilter, getColumnWidth } from "./utils";


export const DataTable = React.forwardRef<any, DataTableProps>(
  ({ columns: columnDefs, data, config, className, customSelectedRows, persistenceKey, serverSide }, ref) => {
    // Initialize sorting from initialState
    const [sorting, setSorting] = useState<SortingState>(() => {
      if (config?.initialState?.sorting) {
        return config.initialState.sorting.map((sort: any) => ({
          id: sort.column,
          desc: sort.direction === "desc",
        }));
      }
      return [];
    });

    // Server-side sorting handler
    const handleSortingChange = useCallback(
      (updater: any) => {
        if (serverSide?.enabled && serverSide.onSortingChange) {
          const newSorting =
            typeof updater === "function" ? updater(sorting) : updater;
          setSorting(newSorting);
          serverSide.onSortingChange(newSorting);
        } else {
          setSorting(updater);
        }
      },
      [serverSide, sorting]
    );

    // Initialize filters from initialState
    const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>(
      () => {
        if (config?.initialState?.filters) {
          return config.initialState.filters.map((filter: any) => ({
            id: filter.column,
            value: filter.value,
          }));
        }
        return [];
      }
    );

    // Server-side filtering handler
    const handleColumnFiltersChange = useCallback(
      (updater: any) => {
        if (serverSide?.enabled && serverSide.onFiltersChange) {
          const newFilters =
            typeof updater === "function" ? updater(columnFilters) : updater;
          setColumnFilters(newFilters);
          serverSide.onFiltersChange(newFilters);
        } else {
          setColumnFilters(updater);
        }
      },
      [serverSide, columnFilters]
    );

    // Initialize column visibility from initialState
    const [columnVisibility, setColumnVisibility] = useState<VisibilityState>(
      config?.initialState?.columnVisibility || {}
    );

    const [rowSelection, setRowSelection] = useState({});
    const [globalFilter, setGlobalFilter] = useState("");

    // Column sizing state with localStorage persistence
    const getStorageKey = (key: string) => persistenceKey ? `${persistenceKey}_${key}` : null;

    const [columnSizing, setColumnSizing] = useState<ColumnSizingState>(() => {
      const storageKey = getStorageKey('columnSizing');
      if (storageKey && typeof window !== 'undefined') {
        try {
          const stored = localStorage.getItem(storageKey);
          if (stored) {
            return JSON.parse(stored);
          }
        } catch (e) {
          console.warn('Failed to load column sizing from localStorage:', e);
        }
      }
      return {};
    });

    // Save column sizing to localStorage
    const handleColumnSizingChange = useCallback((updater: any) => {
      const newSizing = typeof updater === 'function' ? updater(columnSizing) : updater;
      setColumnSizing(newSizing);
      
      const storageKey = getStorageKey('columnSizing');
      if (storageKey && typeof window !== 'undefined') {
        try {
          localStorage.setItem(storageKey, JSON.stringify(newSizing));
        } catch (e) {
          console.warn('Failed to save column sizing to localStorage:', e);
        }
      }
    }, [columnSizing, persistenceKey]);

    // Server-side global filter handler
    const handleGlobalFilterChange = useCallback(
      (updater: any) => {
        if (serverSide?.enabled && serverSide.onGlobalFilterChange) {
          const newGlobalFilter =
            typeof updater === "function" ? updater(globalFilter) : updater;
          setGlobalFilter(newGlobalFilter);
          serverSide.onGlobalFilterChange(newGlobalFilter);
        } else {
          setGlobalFilter(updater);
        }
      },
      [serverSide, globalFilter]
    );


    // Initialize pivot state from initialState
    const [pivotMode, setPivotMode] = useState<boolean>(
      config?.initialState?.pivot?.enabled || false
    );
    const [pivotColumns, setPivotColumns] = useState<string[]>(
      config?.initialState?.pivot?.columns || []
    );
    const [aggregations, setAggregations] = useState<Record<string, string>>(
      config?.initialState?.pivot?.aggregations || {}
    );

    // Store pivot column widths for reliable sticky offset calculations
    const [pivotColumnWidths, setPivotColumnWidths] = useState<
      Record<string, number>
    >({});

    // Refs for header columns to get actual DOM widths
    const headerRefs = useRef<Record<string, HTMLTableCellElement | null>>({});

    // Pivot handlers
    const handlePivotToggle = useCallback(
      (columnId: string) => {
        setPivotColumns((prev) => {
          if (prev.includes(columnId)) {
            // Remove from pivot columns
            const newColumns = prev.filter((id) => id !== columnId);
            // Remove width from storage
            setPivotColumnWidths((prevWidths) => {
              const newWidths = { ...prevWidths };
              delete newWidths[columnId];
              return newWidths;
            });
            // If no columns left, disable pivot mode
            if (newColumns.length === 0) {
              setPivotMode(false);
            }
            return newColumns;
          } else {
            // Add to pivot columns and enable pivot mode
            setPivotMode(true);
            // Get actual width from DOM element or fall back to defined width
            const headerEl = headerRefs.current[columnId];
            const width = headerEl
              ? headerEl.offsetWidth
              : getColumnWidth(columnId, columnDefs);
            setPivotColumnWidths((prevWidths) => ({
              ...prevWidths,
              [columnId]: width,
            }));
            return [...prev, columnId];
          }
        });
      },
      [columnDefs]
    );

    const handleAggregationChange = useCallback(
      (columnId: string, aggregation: string) => {
        setAggregations((prev) => ({
          ...prev,
          [columnId]: aggregation,
        }));
      },
      []
    );

    // Convert column definitions to TanStack format
    const columns = useMemo<ColumnDef<any>[]>(() => {
      return columnDefs.map((col) => ({
        id: col.key,
        accessorKey: col.key,
        meta: {
          title: col.label,
          headerRenderer: col.headerRenderer,
        },
        header: ({ column, table }) => {
          // All columns are aggregatable by default
          const showAggregation = pivotMode;
          // Default aggregation is "display" for all columns
          const defaultAggregation = "display";
          const currentAggregation =
            aggregations[col.key] || defaultAggregation;
          const isPivotColumn = pivotColumns.includes(col.key);
          const pivotLevel = isPivotColumn
            ? pivotColumns.indexOf(col.key) + 1
            : 0;

          return (
            <DataTableColumnHeader
              column={column}
              title={col.label}
              sortable={col.sortable !== false}
              filterable={col.filterable}
              pivotable={config?.features?.pivot !== false} // Pivot feature enabled by default but can be disabled
              aggregatable={config?.features?.pivot !== false} // Aggregation only available when pivot is enabled
              table={table}
              isPivotMode={pivotMode}
              isPivotColumn={isPivotColumn}
              pivotLevel={pivotLevel}
              onPivotToggle={handlePivotToggle}
              onAggregationChange={handleAggregationChange}
              currentAggregation={
                showAggregation && !isPivotColumn
                  ? currentAggregation
                  : undefined
              }
              serverSide={
                serverSide?.enabled
                  ? {
                      enabled: true,
                      onGetDistinctValues: serverSide.onGetDistinctValues,
                    }
                  : undefined
              }
            />
          );
        },
        cell: ({ row }) => {
          const value = row.getValue(col.key);
          
          // Default cell renderer function
          const defaultRenderer = (val: any) => formatCellValue(
            val,
            col.data_type,
            col.format,
            col.currency,
            col.currencyDisplay
          );
          
          // Use custom cell renderer if provided, passing default renderer
          if (col.cellRenderer) {
            return col.cellRenderer(value, row.original, col, defaultRenderer);
          }
          
          // Default cell rendering
          return defaultRenderer(value);
        },
        filterFn: col.data_type === "number" ? "inNumberRange" : fuzzyFilter,
        enableSorting:
          col.sortable !== false && config?.features?.sort !== false,
        enableHiding: config?.features?.columnVisibility !== false,
        size: col.width || 150,
      }));
    }, [
      columnDefs,
      config,
      pivotMode,
      pivotColumns,
      handlePivotToggle,
      handleAggregationChange,
      aggregations,
    ]);


    // Store total row separately for sticky positioning
    const [totalRow, setTotalRow] = useState<any>(null);

    // Process data for pivot mode with multi-level support
    const processedData = useMemo(() => {
      // Pivot mode - only if feature is enabled
      if (
        config?.features?.pivot !== false &&
        pivotMode &&
        pivotColumns.length > 0
      ) {
        const { processedData: result, totalRow: computedTotalRow } = processPivotData(
          data,
          pivotColumns,
          columnDefs,
          aggregations
        );
        setTotalRow(computedTotalRow);
        return result;
      }

      // No pivot - clear total row and return original data
      setTotalRow(null);
      return data;
    }, [
      data,
      pivotMode,
      pivotColumns,
      columnDefs,
      aggregations,
      config?.features?.pivot,
    ]);

    // Add row selection column if enabled
    const tableColumns = useMemo(() => {
      if (config?.features?.rowSelection) {
        return [
          {
            id: "select",
            size: 40,
            header: ({ table }) => (
              <Checkbox
                checked={table.getIsAllPageRowsSelected()}
                onCheckedChange={(value: boolean) =>
                  table.toggleAllPageRowsSelected(!!value)
                }
                aria-label="Select all"
              />
            ),
            cell: ({ row }) => (
              <Checkbox
                checked={row.getIsSelected()}
                onCheckedChange={(value: boolean) =>
                  row.toggleSelected(!!value)
                }
                aria-label="Select row"
              />
            ),
            enableSorting: false,
            enableHiding: false,
          },
          ...columns,
        ];
      }
      return columns;
    }, [columns, config?.features?.rowSelection]);

    const table = useReactTable({
      data: processedData,
      columns: tableColumns,
      filterFns: {
        fuzzy: fuzzyFilter,
      },
      state: {
        sorting,
        columnFilters,
        columnVisibility,
        rowSelection,
        globalFilter,
        columnSizing,
      },
      onSortingChange: handleSortingChange,
      onColumnFiltersChange: handleColumnFiltersChange,
      onColumnVisibilityChange: setColumnVisibility,
      onRowSelectionChange: setRowSelection,
      onGlobalFilterChange: handleGlobalFilterChange,
      onColumnSizingChange: handleColumnSizingChange,
      columnResizeMode: 'onChange',
      enableColumnResizing: true,
      getCoreRowModel: getCoreRowModel(),
      getFilteredRowModel: serverSide?.enabled
        ? getCoreRowModel()
        : getFilteredRowModel(),
      getSortedRowModel: serverSide?.enabled
        ? getCoreRowModel()
        : getSortedRowModel(),
      globalFilterFn: fuzzyFilter,
      manualSorting: serverSide?.enabled,
      manualFiltering: serverSide?.enabled,
      rowCount: serverSide?.totalRows ?? processedData.length,
    });

    // Expose table instance through ref
    React.useImperativeHandle(ref, () => table, [table]);

    // Reset pivot state when pivot feature is disabled
    React.useEffect(() => {
      if (config?.features?.pivot === false) {
        setPivotMode(false);
        setPivotColumns([]);
        setAggregations({});
        setPivotColumnWidths({});
      }
    }, [config?.features?.pivot]);

    // Update pivot column widths after DOM updates
    React.useEffect(() => {
      if (
        config?.features?.pivot !== false &&
        pivotColumns.length > 0 &&
        pivotMode
      ) {
        // Use requestAnimationFrame to ensure DOM has updated
        requestAnimationFrame(() => {
          const newWidths: Record<string, number> = {};
          pivotColumns.forEach((colId) => {
            const headerEl = headerRefs.current[colId];
            if (headerEl) {
              newWidths[colId] = headerEl.offsetWidth;
            } else {
              newWidths[colId] = getColumnWidth(colId, columnDefs);
            }
          });

          // Only update if widths have actually changed
          setPivotColumnWidths((prev) => {
            const hasChanged = Object.keys(newWidths).some(
              (key) => prev[key] !== newWidths[key]
            );
            return hasChanged ? newWidths : prev;
          });
        });
      }
    }, [pivotColumns, pivotMode, columnDefs]);

    // Virtual scrolling setup
    const tableContainerRef = React.useRef<HTMLDivElement>(null);
    // Get rows to render
    const rowsToRender = table.getRowModel().rows;

    // Always use virtual scrolling since we removed pagination
    const useVirtualScrolling = true;

    const rowVirtualizer = useVirtualizer({
      count: useVirtualScrolling ? rowsToRender.length : 0,
      getScrollElement: () => tableContainerRef.current,
      estimateSize: () => 32,
      overscan: 10,
    });

    const virtualRows = useVirtualScrolling
      ? rowVirtualizer.getVirtualItems()
      : [];
    const totalSize = useVirtualScrolling ? rowVirtualizer.getTotalSize() : 0;

    const paddingTop =
      useVirtualScrolling && virtualRows.length > 0
        ? virtualRows?.[0]?.start || 0
        : 0;
    const paddingBottom =
      useVirtualScrolling && virtualRows.length > 0
        ? totalSize - (virtualRows?.[virtualRows.length - 1]?.end || 0)
        : 0;

    return (
      <div
        className={cn(
          "relative border bg-background flex flex-col text-sm",
          className,
          // Force cursor during column resize
          table.getState().columnSizingInfo.isResizingColumn && "[&_*]:cursor-col-resize"
        )}
      >
        <div ref={tableContainerRef} className="flex-1 overflow-auto">
          <table 
            className="w-full" 
            style={{ 
              tableLayout: "fixed",
              width: table.getCenterTotalSize(),
            }}
          >
            <TableHeader
              headerGroups={table.getHeaderGroups()}
              pivotColumns={pivotColumns}
              pivotColumnWidths={pivotColumnWidths}
              columnDefs={columnDefs}
              config={config}
              headerRefs={headerRefs}
              table={table}
            />
            <TableBody
              virtualRows={virtualRows}
              rowsToRender={rowsToRender}
              useVirtualScrolling={useVirtualScrolling}
              paddingTop={paddingTop}
              paddingBottom={paddingBottom}
              pivotColumns={pivotColumns}
              pivotColumnWidths={pivotColumnWidths}
              columnDefs={columnDefs}
              aggregations={aggregations}
              pivotMode={pivotMode}
              config={config}
              customSelectedRows={customSelectedRows}
            />
            <TableFooter
              table={table}
              totalRow={totalRow}
              pivotColumns={pivotColumns}
              pivotColumnWidths={pivotColumnWidths}
              columnDefs={columnDefs}
              aggregations={aggregations}
              config={config}
            />

          </table>
        </div>
      </div>
    );
  }
);

DataTable.displayName = "DataTable";

