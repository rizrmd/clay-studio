"use client";

import * as React from "react";
import { useState, useMemo, useCallback, useRef } from "react";
import {
  ColumnDef,
  ColumnFiltersState,
  SortingState,
  VisibilityState,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  useReactTable,
  FilterFn,
} from "@tanstack/react-table";
import { useVirtualizer } from "@tanstack/react-virtual";
import { rankItem } from "@tanstack/match-sorter-utils";
import { Checkbox } from "@/components/ui/checkbox";
import { DataTableColumnHeader } from "./data-table-column-header";
import { DataTableColumnsMenu } from "./data-table-columns-menu";
import { cn } from "@/lib/utils";
import { TableColumn, TableConfig } from "./demo-data";
import { css } from "goober";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";

interface DataTableProps {
  columns: TableColumn[];
  data: any[];
  config?: TableConfig;
  className?: string;
}

// Fuzzy filter function
const fuzzyFilter: FilterFn<any> = (row, columnId, value, addMeta) => {
  const itemRank = rankItem(row.getValue(columnId), value);
  addMeta({ itemRank });
  return itemRank.passed;
};

export function DataTable({
  columns: columnDefs,
  data,
  config,
  className,
}: DataTableProps) {
  // Initialize sorting from initialState
  const [sorting, setSorting] = useState<SortingState>(() => {
    if (config?.initialState?.sorting) {
      return config.initialState.sorting.map(sort => ({
        id: sort.column,
        desc: sort.direction === 'desc'
      }));
    }
    return [];
  });
  
  // Initialize filters from initialState
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>(() => {
    if (config?.initialState?.filters) {
      return config.initialState.filters.map(filter => ({
        id: filter.column,
        value: filter.value
      }));
    }
    return [];
  });
  
  // Initialize column visibility from initialState
  const [columnVisibility, setColumnVisibility] = useState<VisibilityState>(
    config?.initialState?.columnVisibility || {}
  );
  
  const [rowSelection, setRowSelection] = useState({});
  const [globalFilter, setGlobalFilter] = useState("");
  const [columnOrder, setColumnOrder] = useState<string[]>([]);
  const [draggedColumn, setDraggedColumn] = useState<string | null>(null);
  const [dragOverColumn, setDragOverColumn] = useState<string | null>(null);

  // Helper to get column width consistently
  const getColumnWidth = (columnId: string) => {
    const colDef = columnDefs.find(c => c.key === columnId);
    return colDef?.width || 150;
  };

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
  const [pivotColumnWidths, setPivotColumnWidths] = useState<Record<string, number>>({});
  
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
          const width = headerEl ? headerEl.offsetWidth : getColumnWidth(columnId);
          setPivotColumnWidths((prevWidths) => ({
            ...prevWidths,
            [columnId]: width
          }));
          return [...prev, columnId];
        }
      });
    },
    [getColumnWidth]
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
      },
      header: ({ column, table }) => {
        // All columns are aggregatable by default
        const showAggregation = pivotMode;
        // Default aggregation is "display" for all columns
        const defaultAggregation = 'display';
        const currentAggregation =
          aggregations[col.key] || defaultAggregation;
        const isPivotColumn = pivotColumns.includes(col.key);
        const pivotLevel = isPivotColumn ? pivotColumns.indexOf(col.key) + 1 : 0;

        return (
          <div className="flex flex-col">
            <DataTableColumnHeader
              column={column}
              title={col.label}
              sortable={col.sortable !== false}
              filterable={col.filterable}
              pivotable={true}  // All columns are groupable by default
              aggregatable={true}  // All columns are aggregatable by default
              table={table}
              isPivotMode={pivotMode}
              isPivotColumn={isPivotColumn}
              pivotLevel={pivotLevel}
              onPivotToggle={handlePivotToggle}
              onAggregationChange={handleAggregationChange}
              currentAggregation={showAggregation && !isPivotColumn ? currentAggregation : undefined}
            />
          </div>
        );
      },
      cell: ({ row }) => {
        const value = row.getValue(col.key);
        return formatCellValue(value, col.data_type, col.format, col.currency, col.currencyDisplay);
      },
      filterFn: col.data_type === "number" ? "inNumberRange" : fuzzyFilter,
      enableSorting: col.sortable !== false && config?.features?.sort !== false,
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

  // Helper function for aggregation calculations
  const calculateAggregation = (values: any[], aggregationType: string) => {
    // For "display" aggregation, just return the first non-null value
    if (aggregationType === "display") {
      const firstValue = values.find((v) => v !== null && v !== undefined);
      return firstValue !== undefined ? firstValue : null;
    }
    
    // Filter out null and undefined values
    const nonNullValues = values.filter((v) => v !== null && v !== undefined);
    
    // For count, return the count of available rows (non-null values)
    if (aggregationType === "count") {
      return nonNullValues.length;
    }
    
    if (nonNullValues.length === 0) {
      return null;
    }
    
    // Try to extract numeric values and track both valid and problematic ones
    const problematicValues: any[] = [];
    const validExamples: any[] = [];
    const numericValues = nonNullValues
      .map((v, index) => {
        const num = Number(v);
        if (isNaN(num)) {
          problematicValues.push({ value: v, index });
          return null;
        }
        // Track some valid examples (up to 5)
        if (validExamples.length < 5) {
          validExamples.push({ value: num, index });
        }
        return num;
      })
      .filter((v): v is number => v !== null);
    
    // For numeric aggregations (sum, avg, min, max), use numeric values if any exist
    if (['sum', 'avg', 'min', 'max'].includes(aggregationType)) {
      if (numericValues.length === 0) {
        // No numeric values at all
        return null;
      }
      
      let result: number;
      switch (aggregationType) {
        case "sum":
          result = numericValues.reduce((a, b) => a + b, 0);
          break;
        case "avg":
          result = numericValues.reduce((a, b) => a + b, 0) / numericValues.length;
          break;
        case "min":
          result = Math.min(...numericValues);
          break;
        case "max":
          result = Math.max(...numericValues);
          break;
        default:
          result = numericValues[0] || 0;
      }
      
      // If there were problematic values, return with error details
      if (problematicValues.length > 0) {
        return {
          __hasError: true,
          __errorDetails: {
            problematicValues,
            validExamples,
            validCount: numericValues.length,
            totalCount: nonNullValues.length
          },
          value: result
        };
      }
      
      return result;
    }
    
    // For string/mixed values, use frequency-based aggregations
    const frequencyMap = new Map<any, number>();
    nonNullValues.forEach(value => {
      const key = String(value);
      frequencyMap.set(key, (frequencyMap.get(key) || 0) + 1);
    });
    
    // Sort by frequency
    const sortedByFrequency = Array.from(frequencyMap.entries())
      .sort((a, b) => a[1] - b[1]); // Sort by count ascending
    
    switch (aggregationType) {
      case "min":
        // Return the value with least occurrence
        return sortedByFrequency[0]?.[0] || null;
      case "max":
        // Return the value with most occurrence
        return sortedByFrequency[sortedByFrequency.length - 1]?.[0] || null;
      case "sum":
        // For strings, return count of unique values instead of concatenating
        return `${frequencyMap.size} unique`;
      case "avg":
        // For strings, return most common value (mode)
        return sortedByFrequency[sortedByFrequency.length - 1]?.[0] || null;
      default:
        return nonNullValues[0];
    }
  };

  // Store total row separately for sticky positioning
  const [totalRow, setTotalRow] = useState<any>(null);

  // Process data for pivot mode with multi-level support
  const processedData = useMemo(() => {
    // Pivot mode
    if (pivotMode && pivotColumns.length > 0) {
      // Helper function to parse group key back to individual values
      const parseGroupKey = (key: string) => {
        return key.split("|||");
      };

      // Recursive function to create nested groups
      const createNestedGroups = (
        rows: any[], 
        pivotCols: string[], 
        level: number = 0,
        parentKey: string = ""
      ): any[] => {
        if (pivotCols.length === 0 || level >= pivotCols.length) {
          return rows;
        }

        const currentPivotCol = pivotCols[level];
        const groups: Record<string, any[]> = {};

        // Group data by current pivot column
        rows.forEach((row) => {
          // Convert pivot value to string to avoid formatting issues
          const rawValue = row[currentPivotCol];
          const pivotValue = rawValue !== null && rawValue !== undefined ? String(rawValue) : "No Group";
          if (!groups[pivotValue]) {
            groups[pivotValue] = [];
          }
          groups[pivotValue].push(row);
        });

        const result: any[] = [];
        
        Object.entries(groups).forEach(([groupKey, groupRows]) => {
          const fullKey = parentKey ? `${parentKey}|||${groupKey}` : groupKey;
          
          // Create pivot row for this group
          const pivotRow: any = {
            id: `pivot-${fullKey}`,
            __isPivotRow: true,
            __pivotLevel: level,
            __rowCount: groupRows.length,
            __groupKey: fullKey,
          };

          // Set values for all pivot columns up to current level
          pivotCols.forEach((col, idx) => {
            if (idx < level) {
              // For parent pivot columns, use the value from parent key
              const parentValues = parseGroupKey(fullKey);
              pivotRow[col] = parentValues[idx];
            } else if (idx === level) {
              // Current pivot column
              pivotRow[col] = groupKey;
            } else {
              // Child pivot columns - leave empty
              pivotRow[col] = "";
            }
          });

          // Calculate aggregations for this group (all columns are aggregatable)
          columnDefs.forEach((col) => {
            if (!pivotCols.includes(col.key)) {
              const values = groupRows.map((r) => r[col.key]);
              // Default aggregation is "display" for all columns
              const defaultAggregation = 'display';
              const aggregationType =
                aggregations[col.key] || defaultAggregation;
              
              // Special handling for dates - show date range
              if (col.data_type === "date") {
                const dateValues = groupRows
                  .map((r) => r[col.key])
                  .filter(Boolean)
                  .map(v => new Date(v))
                  .filter(d => !isNaN(d.getTime()));
                
                if (dateValues.length > 0) {
                  const minDate = new Date(Math.min(...dateValues.map(d => d.getTime())));
                  const maxDate = new Date(Math.max(...dateValues.map(d => d.getTime())));
                  
                  if (minDate.getTime() === maxDate.getTime()) {
                    pivotRow[col.key] = minDate.toISOString();
                  } else {
                    // Store as object for special rendering
                    pivotRow[col.key] = {
                      __isDateRange: true,
                      min: minDate.toISOString(),
                      max: maxDate.toISOString()
                    };
                  }
                } else {
                  pivotRow[col.key] = null;
                }
              } else {
                // All other types use standard aggregation
                pivotRow[col.key] = calculateAggregation(values, aggregationType);
              }
            }
          });

          result.push(pivotRow);

          // If there are more pivot levels, create nested groups
          if (level < pivotCols.length - 1) {
            const nestedRows = createNestedGroups(
              groupRows, 
              pivotCols, 
              level + 1,
              fullKey
            );
            result.push(...nestedRows);
          }
        });

        return result;
      };

      // Create the nested pivot structure
      const result = createNestedGroups(data, pivotColumns);

      // Create a total row separately (not added to result)
      if (result.length > 0) {
        const totalRowData: any = {
          id: "pivot-total",
          __isPivotTotal: true,
          __rowCount: data.length,
        };

        // Set "TOTAL" for the first pivot column
        pivotColumns.forEach((col, idx) => {
          totalRowData[col] = idx === 0 ? "TOTAL" : "";
        });

        columnDefs.forEach((col) => {
          if (!pivotColumns.includes(col.key)) {
            const values = data.map((r) => r[col.key]);
            
            // Special handling for date columns - show date range
            if (col.data_type === "date") {
              const dateValues = values
                .filter(Boolean)
                .map(v => new Date(v))
                .filter(d => !isNaN(d.getTime()));
              
              if (dateValues.length > 0) {
                const minDate = new Date(Math.min(...dateValues.map(d => d.getTime())));
                const maxDate = new Date(Math.max(...dateValues.map(d => d.getTime())));
                
                if (minDate.getTime() === maxDate.getTime()) {
                  totalRowData[col.key] = minDate.toISOString();
                } else {
                  // Store as object for special rendering
                  totalRowData[col.key] = {
                    __isDateRange: true,
                    min: minDate.toISOString(),
                    max: maxDate.toISOString()
                  };
                }
              } else {
                totalRowData[col.key] = null;
              }
            } else {
              // All other columns use standard aggregation
              const defaultAggregation = 'display';
              const aggregationType =
                aggregations[col.key] || defaultAggregation;
              const calculationResult = calculateAggregation(values, aggregationType);
              // For total row, always use the calculated value even if there are errors
              totalRowData[col.key] = calculationResult?.__hasError ? calculationResult : calculationResult;
            }
          }
        });

        setTotalRow(totalRowData);
      } else {
        setTotalRow(null);
      }

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
              onCheckedChange={(value: boolean) => row.toggleSelected(!!value)}
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

  // Initialize column order state with default order
  React.useEffect(() => {
    const baseColumnIds = columns.map(col => col.id).filter((id): id is string => id !== undefined);
    if (config?.features?.rowSelection) {
      setColumnOrder(['select', ...baseColumnIds]);
    } else {
      setColumnOrder(baseColumnIds);
    }
  }, []);

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
      columnOrder,
    },
    onSortingChange: setSorting,
    onColumnFiltersChange: setColumnFilters,
    onColumnVisibilityChange: setColumnVisibility,
    onRowSelectionChange: setRowSelection,
    onGlobalFilterChange: setGlobalFilter,
    onColumnOrderChange: setColumnOrder,
    getCoreRowModel: getCoreRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getSortedRowModel: getSortedRowModel(),
    globalFilterFn: fuzzyFilter,
  });

  // Update column order when pivot columns change
  React.useEffect(() => {
    if (pivotColumns.length > 0 && pivotMode) {
      const currentColumns = table.getAllColumns().map(col => col.id);
      const pivotIds = pivotColumns.filter(id => currentColumns.includes(id));
      const nonPivotIds = currentColumns.filter(id => !pivotColumns.includes(id));
      
      // Put select column first if it exists, then pivot columns, then the rest
      const selectCol = nonPivotIds.filter(id => id === 'select');
      const otherCols = nonPivotIds.filter(id => id !== 'select');
      const newOrder = [...selectCol, ...pivotIds, ...otherCols];
      
      setColumnOrder(newOrder);
    }
  }, [pivotColumns, pivotMode, table]);

  // Update pivot column widths after DOM updates
  React.useEffect(() => {
    if (pivotColumns.length > 0 && pivotMode) {
      // Use requestAnimationFrame to ensure DOM has updated
      requestAnimationFrame(() => {
        const newWidths: Record<string, number> = {};
        pivotColumns.forEach(colId => {
          const headerEl = headerRefs.current[colId];
          if (headerEl) {
            newWidths[colId] = headerEl.offsetWidth;
          } else {
            newWidths[colId] = getColumnWidth(colId);
          }
        });
        
        // Only update if widths have actually changed
        setPivotColumnWidths(prev => {
          const hasChanged = Object.keys(newWidths).some(
            key => prev[key] !== newWidths[key]
          );
          return hasChanged ? newWidths : prev;
        });
      });
    }
  }, [pivotColumns, pivotMode, getColumnWidth]);

  // Virtual scrolling setup
  const tableContainerRef = React.useRef<HTMLDivElement>(null);
  // Use sorted rows instead of just filtered rows
  const rowsToRender = table.getRowModel().rows;

  const rowVirtualizer = useVirtualizer({
    count: rowsToRender.length,
    getScrollElement: () => tableContainerRef.current,
    estimateSize: () => 32,
    overscan: 10,
  });

  const virtualRows = rowVirtualizer.getVirtualItems();
  const totalSize = rowVirtualizer.getTotalSize();

  const paddingTop = virtualRows.length > 0 ? virtualRows?.[0]?.start || 0 : 0;
  const paddingBottom =
    virtualRows.length > 0
      ? totalSize - (virtualRows?.[virtualRows.length - 1]?.end || 0)
      : 0;

  return (
    <div
      ref={tableContainerRef}
      className={cn(
        "relative border bg-background overflow-auto text-sm",
        className
      )}
    >
      <table className="w-full">
        <thead className="sticky top-0 bg-background z-30">
          {table.getHeaderGroups().map((headerGroup) => (
            <tr
              key={headerGroup.id}
              className="relative after:content-[''] after:absolute after:bottom-0 after:left-0 after:right-0 after:h-[1px] after:bg-border"
            >
              {headerGroup.headers.map((header, index) => {
                const isLastColumn = index === headerGroup.headers.length - 1;
                const isDragOver = dragOverColumn === header.column.id;
                const isPivotColumn = pivotColumns.includes(header.column.id);
                const pivotIndex = pivotColumns.indexOf(header.column.id);
                
                // Calculate sticky offset for multiple pivot columns using stored widths
                let stickyOffset = 0;
                if (isPivotColumn && pivotIndex > 0) {
                  // Calculate cumulative width of previous pivot columns
                  for (let i = 0; i < pivotIndex; i++) {
                    stickyOffset += pivotColumnWidths[pivotColumns[i]] || getColumnWidth(pivotColumns[i]);
                  }
                }

                return (
                  <th
                    key={header.id}
                    ref={(el) => {
                      // Store ref without interfering with other handlers
                      if (el && header.column.id) {
                        headerRefs.current[header.column.id] = el;
                      }
                    }}
                    onDragOver={(e) => {
                      e.preventDefault();
                      if (draggedColumn && draggedColumn !== header.column.id) {
                        setDragOverColumn(header.column.id);
                      }
                    }}
                    onDragLeave={() => {
                      setDragOverColumn(null);
                    }}
                    onDrop={(e) => {
                      e.preventDefault();
                      if (draggedColumn && draggedColumn !== header.column.id) {
                        const allColumns = table
                          .getAllColumns()
                          .map((col) => col.id);
                        const draggedIndex = allColumns.indexOf(draggedColumn);
                        const targetIndex = allColumns.indexOf(
                          header.column.id
                        );

                        if (draggedIndex !== -1 && targetIndex !== -1) {
                          const newOrder = [...allColumns];
                          newOrder.splice(draggedIndex, 1);
                          newOrder.splice(targetIndex, 0, draggedColumn);
                          setColumnOrder(newOrder);
                        }
                      }
                      setDraggedColumn(null);
                      setDragOverColumn(null);
                    }}
                    className={cn(
                      "text-left align-middle font-medium text-muted-foreground relative transition-all",
                      draggedColumn === header.column.id && "opacity-40",
                      isDragOver && "bg-primary/10 border-l-4 border-primary",
                      draggedColumn &&
                        draggedColumn !== header.column.id &&
                        "cursor-move",
                      isPivotColumn && "sticky bg-background z-40 shadow-[2px_0_4px_-2px_rgba(0,0,0,0.1)]",
                      css`
                        > * {
                          min-height: 35px;
                          white-space: nowrap;
                        }
                      `
                    )}
                    style={{ 
                      width: header.getSize(),
                      ...(isPivotColumn && {
                        left: pivotIndex === 0 ? 0 : `${stickyOffset}px`
                      })
                    }}
                  >
                    <div className="flex items-center group">
                      <div
                        className={cn(
                          "flex items-center flex-1",
                          css`
                            > div {
                              flex: 1;
                              display: flex;
                              flex-direction: column;
                              button {
                                padding-left: 4px;
                                padding-right: 4px;
                              }
                            }
                          `
                        )}
                      >
                        {header.isPlaceholder
                          ? null
                          : flexRender(
                              header.column.columnDef.header,
                              header.getContext()
                            )}
                      </div>
                      {!header.isPlaceholder &&
                        header.column.id !== "select" &&
                        !isPivotColumn && (
                          <div
                            draggable
                            onDragStart={(e) => {
                              e.stopPropagation();
                              setDraggedColumn(header.column.id);
                              e.dataTransfer.effectAllowed = "move";

                              // Create custom drag image
                              const dragImage = document.createElement("div");
                              dragImage.className =
                                "absolute bg-primary text-primary-foreground px-3 py-1 rounded shadow-lg text-sm font-medium";
                              dragImage.textContent =
                                (header.column.columnDef.meta as any)?.title ||
                                header.column.id;
                              dragImage.style.position = "absolute";
                              dragImage.style.top = "-1000px";
                              document.body.appendChild(dragImage);
                              e.dataTransfer.setDragImage(dragImage, 0, 0);
                              setTimeout(
                                () => document.body.removeChild(dragImage),
                                0
                              );
                            }}
                            onDragEnd={() => {
                              setDraggedColumn(null);
                              setDragOverColumn(null);
                            }}
                            className={cn(
                              "cursor-move ml-2 opacity-0 group-hover:opacity-50 hover:!opacity-100 transition-opacity",
                              draggedColumn === header.column.id &&
                                "opacity-100"
                            )}
                          >
                            <svg
                              width="8"
                              height="14"
                              viewBox="0 0 8 14"
                              fill="currentColor"
                              className="text-muted-foreground"
                            >
                              <circle cx="2" cy="2" r="1" />
                              <circle cx="6" cy="2" r="1" />
                              <circle cx="2" cy="7" r="1" />
                              <circle cx="6" cy="7" r="1" />
                              <circle cx="2" cy="12" r="1" />
                              <circle cx="6" cy="12" r="1" />
                            </svg>
                          </div>
                        )}
                      {isLastColumn && <DataTableColumnsMenu table={table} />}
                    </div>
                  </th>
                );
              })}
            </tr>
          ))}
        </thead>
        <tbody>
          {paddingTop > 0 && (
            <tr>
              <td style={{ height: `${paddingTop}px` }} />
            </tr>
          )}
          {virtualRows.map((virtualRow) => {
            const row = rowsToRender[virtualRow.index];
            const rowData = row.original;


            // Check if this is a pivot row
            if (rowData?.__isPivotRow) {
              const pivotLevel = rowData.__pivotLevel || 0;
              return (
                <tr
                  key={row.id}
                  className={cn(
                    "border-b transition-colors",
                    pivotLevel === 0 
                      ? "bg-blue-50 dark:bg-blue-950/20 font-semibold"
                      : "bg-blue-50/30 dark:bg-blue-950/10 hover:bg-blue-100",
                    pivotLevel > 0 && "pl-4"
                  )}
                >
                  {row.getVisibleCells().map((cell) => {
                    const column = columnDefs.find(
                      (col) => col.key === cell.column.id
                    );
                    const value = rowData[cell.column.id];
                    // Use cell.column.id to match aggregations state
                    const currentAggregation = aggregations[cell.column.id] || aggregations[column?.key || ''] || "display";
                    const isPivotCol = pivotColumns.includes(cell.column.id);
                    const pivotIdx = pivotColumns.indexOf(cell.column.id);
                    
                    // Calculate sticky offset for pivot columns using stored widths
                    let cellStickyOffset = 0;
                    if (isPivotCol && pivotIdx > 0) {
                      for (let i = 0; i < pivotIdx; i++) {
                        cellStickyOffset += pivotColumnWidths[pivotColumns[i]] || getColumnWidth(pivotColumns[i]);
                      }
                    }

                    return (
                      <td
                        key={cell.id}
                        className={cn(
                          "px-2 py-1 align-middle",
                          column?.aggregatable &&
                            "font-semibold text-blue-700 dark:text-blue-400",
                          pivotColumns.includes(cell.column.id) && pivotLevel > 0 && "pl-6",
                          isPivotCol && "sticky bg-blue-50 dark:bg-blue-950/40 z-10 shadow-[2px_0_4px_-2px_rgba(0,0,0,0.1)]"
                        )}
                        style={{ 
                          width: cell.column.getSize(),
                          ...(isPivotCol && {
                            left: pivotIdx === 0 ? 0 : `${cellStickyOffset}px`
                          })
                        }}
                      >
                        {cell.column.id === "select" ? null : 
                          pivotColumns.includes(cell.column.id) ? (
                            // Render pivot column value (not aggregated)
                            value !== null && value !== undefined && value !== "" ? (
                              <div className="flex items-center gap-1">
                                {pivotLevel > 0 && pivotColumns.indexOf(cell.column.id) === pivotLevel && (
                                  <div className="absolute ml-[-25px]">→</div>
                                )}
                                {/* Format pivot column values based on their data type */}
                                {column?.data_type === "boolean" ? (
                                  formatCellValue(value === "true" || value === true, "boolean")
                                ) : (
                                  <span>{String(value)}</span>
                                )}
                              </div>
                            ) : (
                              <span className="text-muted-foreground"></span>
                            )
                          ) : !pivotColumns.includes(cell.column.id) ? (
                            // Render aggregated values for non-pivot columns
                            value?.__isDateRange ? (
                              // Render date range for date columns
                              <div className="flex items-center gap-1 text-xs">
                                <span className="whitespace-nowrap">{formatCellValue(value.min, "date")}</span>
                                <span className="text-muted-foreground">→</span>
                                <span className="whitespace-nowrap">{formatCellValue(value.max, "date")}</span>
                              </div>
                            ) : value?.__hasError ? (
                              <Popover>
                                <PopoverTrigger asChild>
                                  <button className="flex items-center gap-1 text-orange-600 dark:text-orange-400 hover:underline">
                                    <span className="text-xs">⚠️</span>
                                    {value.value !== null && (
                                      <span className="text-xs">({formatCellValue(value.value, currentAggregation === "count" ? "number" : column?.data_type, column?.format, column?.currency, column?.currencyDisplay)})</span>
                                    )}
                                  </button>
                                </PopoverTrigger>
                                <PopoverContent className="w-96 p-3">
                                  <div className="space-y-3">
                                    <div className="flex items-center justify-between">
                                      <div className="font-semibold text-sm">Data Quality Issue</div>
                                      <div className="text-xs text-muted-foreground">
                                        {value.__errorDetails.validCount} valid / {value.__errorDetails.totalCount} total values
                                      </div>
                                    </div>
                                    
                                    <div className="border rounded-md overflow-hidden">
                                      <table className="w-full text-xs">
                                        <thead className="bg-muted/50">
                                          <tr>
                                            <th className="px-2 py-1 text-left font-medium">Row</th>
                                            <th className="px-2 py-1 text-left font-medium">Value</th>
                                            <th className="px-2 py-1 text-left font-medium">Status</th>
                                          </tr>
                                        </thead>
                                        <tbody>
                                          {/* Show first 2 good examples */}
                                          {value.__errorDetails.validExamples?.slice(0, 2).map((example: any, idx: number) => (
                                            <tr key={`valid-${idx}`} className="border-t">
                                              <td className="px-2 py-1">{example.index + 1}</td>
                                              <td className="px-2 py-1 font-mono">{formatCellValue(example.value, column?.data_type, column?.format, column?.currency, column?.currencyDisplay)}</td>
                                              <td className="px-2 py-1">
                                                <span className="text-green-600 dark:text-green-400">✓ Valid</span>
                                              </td>
                                            </tr>
                                          ))}
                                          
                                          {/* Show problematic value in the middle */}
                                          {value.__errorDetails.problematicValues.slice(0, 1).map((pv: any, idx: number) => (
                                            <tr key={`invalid-${idx}`} className="border-t bg-red-50 dark:bg-red-950/20">
                                              <td className="px-2 py-1">{pv.index + 1}</td>
                                              <td className="px-2 py-1 font-mono text-red-600 dark:text-red-400">"{pv.value}"</td>
                                              <td className="px-2 py-1">
                                                <span className="text-red-600 dark:text-red-400">✗ Invalid</span>
                                              </td>
                                            </tr>
                                          ))}
                                          
                                          {/* Show remaining good examples */}
                                          {value.__errorDetails.validExamples?.slice(2, 4).map((example: any, idx: number) => (
                                            <tr key={`valid-rest-${idx}`} className="border-t">
                                              <td className="px-2 py-1">{example.index + 1}</td>
                                              <td className="px-2 py-1 font-mono">{formatCellValue(example.value, column?.data_type, column?.format, column?.currency, column?.currencyDisplay)}</td>
                                              <td className="px-2 py-1">
                                                <span className="text-green-600 dark:text-green-400">✓ Valid</span>
                                              </td>
                                            </tr>
                                          ))}
                                          
                                          {value.__errorDetails.problematicValues.length > 1 && (
                                            <tr className="border-t">
                                              <td colSpan={3} className="px-2 py-1 text-center text-muted-foreground">
                                                ... and {value.__errorDetails.problematicValues.length - 1} more invalid value(s)
                                              </td>
                                            </tr>
                                          )}
                                        </tbody>
                                      </table>
                                    </div>
                                  </div>
                                </PopoverContent>
                              </Popover>
                            ) : value === null ? (
                              <span className="text-muted-foreground">—</span>
                            ) : currentAggregation === "display" ? (
                              // For display aggregation, show formatted value based on column type
                              formatCellValue(value, column?.data_type, column?.format, column?.currency, column?.currencyDisplay)
                            ) : currentAggregation === "count" ? (
                              // For count aggregation, always show as plain number regardless of column type
                              <div className="flex items-center gap-1">
                                <span className="text-xs text-muted-foreground">count:</span>
                                <span>{value.toLocaleString()}</span>
                              </div>
                            ) : typeof value === "number" && !isNaN(value) ? (
                              <div className="flex items-center gap-1">
                                <span className="text-xs text-muted-foreground">
                                  {currentAggregation}:
                                </span>
                                {formatCellValue(value, column?.data_type, column?.format, column?.currency, column?.currencyDisplay)}
                              </div>
                            ) : (
                              // For non-numeric values (e.g., strings from other aggregations)
                              <div className="flex items-center gap-1">
                                <span className="text-xs text-muted-foreground">
                                  {currentAggregation}:
                                </span>
                                <span>{String(value || '')}</span>
                              </div>
                            )
                          ) : (
                            // For any other non-aggregated values in pivot rows
                            // Don't format pivot column values as currency
                            pivotColumns.includes(cell.column.id) ? String(value || '') : formatCellValue(value, column?.data_type, column?.format, column?.currency, column?.currencyDisplay)
                          )
                        }
                      </td>
                    );
                  })}
                </tr>
              );
            }

            // Skip total row in virtual scroll (we'll render it separately)
            if (rowData?.__isPivotTotal) {
              return null;
            }

            // Regular data row
            return (
              <tr
                key={row.id}
                className={cn(
                  "border-b transition-colors hover:bg-muted/50",
                  row.getIsSelected() && "bg-muted"
                )}
              >
                {row.getVisibleCells().map((cell) => {
                  const isPivotCol = pivotColumns.includes(cell.column.id);
                  const pivotIdx = pivotColumns.indexOf(cell.column.id);
                  
                  // Calculate sticky offset for pivot columns using stored widths
                  let cellStickyOffset = 0;
                  if (isPivotCol && pivotIdx > 0) {
                    for (let i = 0; i < pivotIdx; i++) {
                      cellStickyOffset += pivotColumnWidths[pivotColumns[i]] || getColumnWidth(pivotColumns[i]);
                    }
                  }
                  
                  return (
                    <td
                      key={cell.id}
                      className={cn(
                        "px-2 py-1 align-middle",
                        isPivotCol && pivotMode && "sticky bg-background z-10 shadow-[2px_0_4px_-2px_rgba(0,0,0,0.1)]"
                      )}
                      style={{ 
                        width: cell.column.getSize(),
                        ...(isPivotCol && pivotMode && {
                          left: pivotIdx === 0 ? 0 : `${cellStickyOffset}px`
                        })
                      }}
                    >
                      {flexRender(cell.column.columnDef.cell, cell.getContext())}
                    </td>
                  );
                })}
              </tr>
            );
          })}
          {paddingBottom > 0 && (
            <tr>
              <td style={{ height: `${paddingBottom}px` }} />
            </tr>
          )}
        </tbody>
        {/* Sticky Total Row */}
        {totalRow && pivotMode && (
          <tfoot className="sticky bottom-0 z-10">
            <tr className="border-t-2 border-b font-bold bg-muted">
              {table.getVisibleLeafColumns().map((column) => {
                const columnDef = columnDefs.find(
                  (col) => col.key === column.id
                );
                const value = totalRow[column.id];
                const currentAggregation = aggregations[columnDef?.key || ''] || "display";
                const isPivotCol = pivotColumns.includes(column.id);
                const pivotIdx = pivotColumns.indexOf(column.id);
                
                // Calculate sticky offset for pivot columns using stored widths
                let cellStickyOffset = 0;
                if (isPivotCol && pivotIdx > 0) {
                  for (let i = 0; i < pivotIdx; i++) {
                    cellStickyOffset += pivotColumnWidths[pivotColumns[i]] || getColumnWidth(pivotColumns[i]);
                  }
                }

                return (
                  <td
                    key={column.id}
                    className={cn(
                      "px-2 py-2 align-middle",
                      isPivotCol && "sticky bg-muted z-20 shadow-[2px_0_4px_-2px_rgba(0,0,0,0.1)]"
                    )}
                    style={{ 
                      width: column.getSize(),
                      ...(isPivotCol && {
                        left: pivotIdx === 0 ? 0 : `${cellStickyOffset}px`
                      })
                    }}
                  >
                    {column.id === "select" ? null : 
                      value?.__isDateRange ? (
                        // Render date range in total row
                        <div className="flex items-center gap-1 text-xs font-semibold">
                          <span className="whitespace-nowrap">{formatCellValue(value.min, "date")}</span>
                          <span className="text-muted-foreground">→</span>
                          <span className="whitespace-nowrap">{formatCellValue(value.max, "date")}</span>
                        </div>
                      ) : value?.__hasError ? (
                        // Show value with warning icon in total row
                        <div className="flex items-center gap-1">
                          <Popover>
                            <PopoverTrigger asChild>
                              <button className="text-orange-600 dark:text-orange-400 hover:opacity-70">
                                <span className="text-xs">⚠️</span>
                              </button>
                            </PopoverTrigger>
                            <PopoverContent className="w-80">
                              <div className="space-y-2">
                                <div className="font-semibold text-sm">Data Quality Issue in Total</div>
                                <div className="text-xs text-muted-foreground">
                                  Column: <span className="font-mono">{value.__errorDetails.columnKey}</span>
                                </div>
                                <div className="text-xs">
                                  Found {value.__errorDetails.problematicValues.length} non-numeric value(s) out of {value.__errorDetails.totalCount} total
                                </div>
                                <div className="text-xs">
                                  <div className="font-semibold mb-1">Sample problematic values:</div>
                                  <div className="font-mono bg-muted p-2 rounded text-orange-600 dark:text-orange-400">
                                    {value.__errorDetails.problematicValues.slice(0, 3).map((pv: any, idx: number) => (
                                      <div key={idx}>"{pv.value}" at row {pv.index + 1}</div>
                                    ))}
                                    {value.__errorDetails.problematicValues.length > 3 && (
                                      <div>... and {value.__errorDetails.problematicValues.length - 3} more</div>
                                    )}
                                  </div>
                                </div>
                                {value.__errorDetails.validCount > 0 && (
                                  <div className="text-xs text-muted-foreground">
                                    Total calculated from {value.__errorDetails.validCount} valid numeric value(s)
                                  </div>
                                )}
                              </div>
                            </PopoverContent>
                          </Popover>
                          {value.value !== null && (
                            <span>
                              {currentAggregation === "count" 
                                ? value.value.toLocaleString()
                                : formatCellValue(value.value, columnDef?.data_type, columnDef?.format, columnDef?.currency, columnDef?.currencyDisplay)}
                            </span>
                          )}
                        </div>
                      ) : currentAggregation === "display" ? (
                        // For display aggregation in total row, just show the value
                        formatCellValue(
                          value,
                          columnDef?.data_type,
                          columnDef?.format,
                          columnDef?.currency,
                          columnDef?.currencyDisplay
                        )
                      ) : currentAggregation === "count" ? (
                        value?.toLocaleString()
                      ) : (
                        formatCellValue(
                          value,
                          columnDef?.data_type,
                          columnDef?.format,
                          columnDef?.currency,
                          columnDef?.currencyDisplay
                        )
                      )}
                  </td>
                );
              })}
            </tr>
          </tfoot>
        )}
      </table>
    </div>
  );
}

function formatCellValue(
  value: any,
  dataType?: string,
  format?: string,
  currency?: string,
  currencyDisplay?: 'symbol' | 'code' | 'name'
): React.ReactNode {
  if (value === null || value === undefined)
    return <span className="text-muted-foreground">—</span>;

  // Handle NaN specifically
  if (typeof value === "number" && isNaN(value))
    return <span className="text-muted-foreground">—</span>;

  switch (dataType) {
    case "date":
      try {
        const date = new Date(value);
        if (isNaN(date.getTime())) {
          return <span className="text-muted-foreground">—</span>;
        }
        return date.toLocaleDateString("en-US", {
          year: "numeric",
          month: "short",
          day: "numeric",
        });
      } catch {
        return <span className="text-muted-foreground">—</span>;
      }
    case "currency":
      const numValue = Number(value);
      if (isNaN(numValue)) {
        return <span className="text-muted-foreground">—</span>;
      }
      
      // Determine locale based on currency
      const getLocale = (curr: string) => {
        switch (curr) {
          case 'USD': return 'en-US';
          case 'EUR': return 'de-DE';
          case 'GBP': return 'en-GB';
          case 'JPY': return 'ja-JP';
          case 'CNY': return 'zh-CN';
          case 'IDR': return 'id-ID';
          case 'SGD': return 'en-SG';
          case 'MYR': return 'ms-MY';
          case 'THB': return 'th-TH';
          case 'VND': return 'vi-VN';
          case 'PHP': return 'en-PH';
          default: return 'en-US';
        }
      };
      
      const locale = getLocale(currency || 'USD');
      const currencyCode = currency || 'USD';
      
      try {
        return new Intl.NumberFormat(locale, {
          style: "currency",
          currency: currencyCode,
          currencyDisplay: currencyDisplay || 'symbol',
          minimumFractionDigits: ['IDR', 'JPY', 'VND'].includes(currencyCode) ? 0 : 2,
          maximumFractionDigits: ['IDR', 'JPY', 'VND'].includes(currencyCode) ? 0 : 2,
        }).format(numValue);
      } catch {
        // Fallback to simple formatting if Intl fails
        return `${currencyCode} ${numValue.toLocaleString()}`;
      }
    case "number":
      const num = Number(value);
      if (isNaN(num)) {
        return <span className="text-muted-foreground">—</span>;
      }
      if (format === "percentage") {
        return `${(num * 100).toFixed(2)}%`;
      }
      return num.toLocaleString();
    case "boolean":
      return (
        <div
          className={cn(
            "inline-flex items-center justify-center rounded-full px-2 py-1 font-medium",
            value
              ? "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400"
              : "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400"
          )}
        >
          {value ? "Yes" : "No"}
        </div>
      );
    default:
      return String(value);
  }
}
