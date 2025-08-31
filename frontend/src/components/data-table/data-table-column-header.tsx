"use client";

import { Column, Table } from "@tanstack/react-table";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  EyeOff,
  Filter,
  Check,
  Search,
  X,
  Table2,
  ChevronRight,
} from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  DropdownMenuItem,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Input } from "@/components/ui/input";
import { useState, useMemo } from "react";

interface DataTableColumnHeaderProps<TData, TValue>
  extends React.HTMLAttributes<HTMLDivElement> {
  column: Column<TData, TValue>;
  title: string;
  sortable?: boolean;
  filterable?: boolean;
  pivotable?: boolean;
  aggregatable?: boolean;
  table: Table<TData>;
  isPivotMode?: boolean;
  isPivotColumn?: boolean;
  pivotLevel?: number;
  onPivotToggle?: (columnId: string) => void;
  onAggregationChange?: (columnId: string, aggregation: string) => void;
  currentAggregation?: string;
}

export function DataTableColumnHeader<TData, TValue>({
  column,
  title,
  sortable = true,
  filterable = false,
  pivotable = false,
  aggregatable = false,
  table,
  isPivotMode = false,
  isPivotColumn = false,
  pivotLevel = 0,
  onPivotToggle,
  onAggregationChange,
  currentAggregation,
  className,
}: DataTableColumnHeaderProps<TData, TValue>) {
  const isFiltered = column.getFilterValue() !== undefined;
  const [filterOpen, setFilterOpen] = useState(false);
  const [searchTerm, setSearchTerm] = useState("");

  // Get unique values for this column
  const uniqueValues = useMemo(() => {
    if (!filterable) return [];
    const values = new Set<string>();
    table.getCoreRowModel().flatRows.forEach((row) => {
      const value = row.getValue(column.id);
      if (value !== null && value !== undefined) {
        values.add(String(value));
      }
    });
    return Array.from(values).sort();
  }, [filterable, table, column.id]);

  // Filter values based on search term
  const filteredValues = useMemo(() => {
    if (!searchTerm) return uniqueValues;
    return uniqueValues.filter((value) =>
      value.toLowerCase().includes(searchTerm.toLowerCase())
    );
  }, [uniqueValues, searchTerm]);

  if (!sortable && !filterable && !column.getCanHide()) {
    return <div className={cn("font-medium px-2", className)}>{title}</div>;
  }

  return (
    <div className="flex flex-col gap-1">
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button
            className={cn(
              "w-full flex items-center justify-between px-0 py-1 text-left hover:bg-muted/50 data-[state=open]:bg-muted",
              className
            )}
          >
            <div className="flex items-center gap-1">
              <span className="font-medium">{title}</span>
              {isPivotColumn && pivotLevel > 0 && (
                <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400">
                  {pivotLevel === 1
                    ? "1st"
                    : pivotLevel === 2
                    ? "2nd"
                    : pivotLevel === 3
                    ? "3rd"
                    : `${pivotLevel}th`}
                </span>
              )}
            </div>
            <div className="flex items-center space-x-1">
              {isFiltered && <Filter className="h-3 w-3 text-primary" />}
              {column.getIsSorted() === "desc" ? (
                <ArrowDown className="h-3 w-3" />
              ) : column.getIsSorted() === "asc" ? (
                <ArrowUp className="h-3 w-3" />
              ) : null}
            </div>
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="w-48">
          {filterable && (
            <>
              <div className="p-1">
                <Popover open={filterOpen} onOpenChange={setFilterOpen}>
                  <PopoverTrigger asChild>
                    <div className="flex">
                      <Button
                        variant="outline"
                        size="sm"
                        className="w-full h-7 text-xs justify-between"
                      >
                        <span className="truncate">
                          {isFiltered
                            ? `${column.getFilterValue()}`
                            : "Filter values..."}
                        </span>
                        <Search className="ml-2 h-3 w-3 shrink-0" />
                      </Button>
                      {isFiltered && (
                        <div className="pl-1">
                          <Button
                            variant="outline"
                            size="sm"
                            className="h-7 max-w-0 px-0 min-w-[30px] rounded-sm text-xs"
                            onClick={() => column.setFilterValue(undefined)}
                          >
                            <X className="h-3 w-3" />
                          </Button>
                        </div>
                      )}
                    </div>
                  </PopoverTrigger>
                  <PopoverContent className="w-48 p-0" align="start">
                    <div className="p-2 border-b">
                      <Input
                        placeholder="Search values..."
                        value={searchTerm}
                        onChange={(e) => setSearchTerm(e.target.value)}
                        className="h-7 text-xs"
                      />
                    </div>
                    <div className="max-h-48 overflow-auto">
                      {filteredValues.length === 0 ? (
                        <div className="p-2 text-xs text-muted-foreground">
                          No values found
                        </div>
                      ) : (
                        filteredValues.map((value) => (
                          <button
                            key={value}
                            onClick={() => {
                              column.setFilterValue(
                                value === column.getFilterValue()
                                  ? undefined
                                  : value
                              );
                              setFilterOpen(false);
                            }}
                            className="w-full flex items-center px-3 py-2 text-xs hover:bg-muted text-left"
                          >
                            <Check
                              className={cn(
                                "mr-2 h-3 w-3",
                                value === column.getFilterValue()
                                  ? "opacity-100"
                                  : "opacity-0"
                              )}
                            />
                            {value}
                          </button>
                        ))
                      )}
                    </div>
                  </PopoverContent>
                </Popover>
              </div>

              <DropdownMenuSeparator />
            </>
          )}

          {/* Pivot Section */}
          {pivotable && (
            <>
              <div className="p-1">
                <Button
                  variant="ghost"
                  size="sm"
                  className="w-full h-7 justify-start text-xs"
                  onClick={() => onPivotToggle?.(column.id)}
                >
                  <Table2 className="mr-2 h-3 w-3" />
                  {isPivotColumn ? "Remove from pivot" : "Pivot by this column"}
                  {isPivotColumn && <Check className="ml-auto h-3 w-3" />}
                </Button>
              </div>
              {isPivotMode && aggregatable && (
                <>
                  <DropdownMenuSeparator />
                  <DropdownMenuSub>
                    <DropdownMenuSubTrigger className="text-xs">
                      <ChevronRight className="mr-2 h-3 w-3" />
                      Aggregation
                    </DropdownMenuSubTrigger>
                    <DropdownMenuSubContent>
                      {["display", "sum", "avg", "count", "min", "max"].map(
                        (agg) => (
                          <DropdownMenuItem
                            key={agg}
                            onClick={() =>
                              onAggregationChange?.(column.id, agg)
                            }
                            className="text-xs"
                          >
                            {agg.charAt(0).toUpperCase() + agg.slice(1)}
                          </DropdownMenuItem>
                        )
                      )}
                    </DropdownMenuSubContent>
                  </DropdownMenuSub>
                </>
              )}
              <DropdownMenuSeparator />
            </>
          )}

          <div className="flex p-1 gap-1">
            {sortable && column.getCanSort() && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 flex-1"
                onClick={() => {
                  if (column.getIsSorted() === "asc") {
                    column.toggleSorting(true);
                  } else if (column.getIsSorted() === "desc") {
                    column.clearSorting();
                  } else {
                    column.toggleSorting(false);
                  }
                }}
              >
                {column.getIsSorted() === "desc" ? (
                  <>
                    <ArrowDown className="mr-1 h-3 w-3" />
                    Z→A
                  </>
                ) : column.getIsSorted() === "asc" ? (
                  <>
                    <ArrowUp className="mr-1 h-3 w-3" />
                    A→Z
                  </>
                ) : (
                  <>
                    <ArrowUpDown className="mr-1 h-3 w-3" />
                    Sort
                  </>
                )}
              </Button>
            )}

            {column.getCanHide() && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 flex-1"
                onClick={() => column.toggleVisibility(false)}
              >
                <EyeOff className="mr-1 h-3 w-3" />
                Hide
              </Button>
            )}
          </div>
        </DropdownMenuContent>
      </DropdownMenu>

      {/* Clickable Aggregation Selector */}
      {currentAggregation && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <button
              className={cn(
                "mb-1 -mt-1 inline-flex items-center gap-1 text-[10px] uppercase px-2 py-0.5 rounded-md  hover:bg-blue-100 dark:bg-blue-950/50 dark:hover:bg-blue-950  font-semibold transition-all border  ml-1",
                currentAggregation === "display"
                  ? " border"
                  : "text-blue-600 dark:text-blue-400 bg-blue-50 border-blue-200 dark:border-blue-800"
              )}
              title="Click to change aggregation"
            >
              <span>{currentAggregation}</span>
              <ChevronRight className="h-2.5 w-2.5 rotate-90" />
            </button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start" className="w-32">
            <div className="px-2 py-1 text-xs font-semibold text-muted-foreground">
              Aggregation
            </div>
            {["display", "sum", "avg", "count", "min", "max"].map((agg) => (
              <DropdownMenuItem
                key={agg}
                onClick={() => onAggregationChange?.(column.id, agg)}
                className="text-xs"
              >
                <span className="flex-1">
                  {agg.charAt(0).toUpperCase() + agg.slice(1)}
                </span>
                {currentAggregation === agg && (
                  <Check className="ml-2 h-3 w-3 text-primary" />
                )}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </div>
  );
}
