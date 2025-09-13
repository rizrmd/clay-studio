import { useState } from "react";
import { ChevronUp, ChevronDown, ChevronLeft, ChevronRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { cn } from "@/lib/utils";
import type { QueryResult } from "@/lib/api/datasources";

interface DataGridProps {
  data: QueryResult | null;
  loading: boolean;
  currentPage: number;
  pageSize: number;
  totalRows: number;
  sortColumn: string | null;
  sortDirection: "asc" | "desc";
  onPageChange: (page: number) => void;
  onPageSizeChange: (size: number) => void;
  onSort: (column: string, direction: "asc" | "desc") => void;
}

export function DataGrid({
  data,
  loading,
  currentPage,
  pageSize,
  totalRows,
  sortColumn,
  sortDirection,
  onPageChange,
  onPageSizeChange,
  onSort,
}: DataGridProps) {
  const [selectedCells, setSelectedCells] = useState<Set<string>>(new Set());

  const handleSort = (column: string) => {
    if (sortColumn === column) {
      // Toggle direction
      onSort(column, sortDirection === "asc" ? "desc" : "asc");
    } else {
      // New column, default to asc
      onSort(column, "asc");
    }
  };

  const getSortIcon = (column: string) => {
    if (sortColumn !== column) {
      return null;
    }
    return sortDirection === "asc" ? (
      <ChevronUp className="h-3 w-3" />
    ) : (
      <ChevronDown className="h-3 w-3" />
    );
  };

  const totalPages = Math.ceil(totalRows / pageSize);
  const startRow = (currentPage - 1) * pageSize + 1;
  const endRow = Math.min(currentPage * pageSize, totalRows);

  if (loading) {
    return (
      <div className="flex flex-col h-full">
        <div className="flex-1 p-4">
          <div className="animate-pulse space-y-4">
            {/* Header skeleton */}
            <div className="flex space-x-4">
              {[...Array(5)].map((_, i) => (
                <div key={i} className="h-4 bg-muted rounded w-20" />
              ))}
            </div>
            {/* Rows skeleton */}
            {[...Array(10)].map((_, i) => (
              <div key={i} className="flex space-x-4">
                {[...Array(5)].map((_, j) => (
                  <div key={j} className="h-4 bg-muted rounded w-20" />
                ))}
              </div>
            ))}
          </div>
        </div>
        <div className="border-t p-3">
          <div className="h-4 bg-muted rounded w-40" />
        </div>
      </div>
    );
  }

  if (!data) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-muted-foreground">No data available</p>
      </div>
    );
  }

  if (data.rows.length === 0) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <p className="text-muted-foreground">No rows found</p>
          <p className="text-sm text-muted-foreground mt-1">
            The table appears to be empty
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Table Container */}
      <div className="flex-1 overflow-auto">
        <Table>
          <TableHeader className="sticky top-0 bg-background">
            <TableRow>
              {data.columns.map((column, index) => (
                <TableHead
                  key={`${column}-${index}`}
                  className="cursor-pointer hover:bg-muted/50 select-none"
                  onClick={() => handleSort(column)}
                >
                  <div className="flex items-center gap-1">
                    <span className="truncate">{column}</span>
                    {getSortIcon(column)}
                  </div>
                </TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {data.rows.map((row, rowIndex) => (
              <TableRow key={rowIndex} className="hover:bg-muted/50">
                {row.map((cell, cellIndex) => {
                  const cellKey = `${rowIndex}-${cellIndex}`;
                  return (
                    <TableCell
                      key={cellKey}
                      className={cn(
                        "font-mono text-xs max-w-xs truncate cursor-pointer",
                        selectedCells.has(cellKey) && "bg-primary/10"
                      )}
                      onClick={() => {
                        const newSelected = new Set(selectedCells);
                        if (newSelected.has(cellKey)) {
                          newSelected.delete(cellKey);
                        } else {
                          newSelected.add(cellKey);
                        }
                        setSelectedCells(newSelected);
                      }}
                      title={cell || "NULL"}
                    >
                      {cell || (
                        <span className="text-muted-foreground italic">NULL</span>
                      )}
                    </TableCell>
                  );
                })}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      {/* Pagination Footer */}
      <div className="border-t p-3 bg-background">
        <div className="flex items-center justify-between">
          {/* Info */}
          <div className="flex items-center gap-4 text-sm text-muted-foreground">
            <span>
              Showing {startRow}-{endRow} of {totalRows} rows
            </span>
            {data.execution_time_ms && (
              <span>
                • Query executed in {data.execution_time_ms}ms
              </span>
            )}
          </div>

          {/* Controls */}
          <div className="flex items-center gap-4">
            {/* Page Size */}
            <div className="flex items-center gap-2">
              <span className="text-sm text-muted-foreground">Rows per page:</span>
              <Select
                value={pageSize.toString()}
                onValueChange={(value) => onPageSizeChange(parseInt(value))}
              >
                <SelectTrigger className="h-8 w-16">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="25">25</SelectItem>
                  <SelectItem value="50">50</SelectItem>
                  <SelectItem value="100">100</SelectItem>
                  <SelectItem value="200">200</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {/* Pagination */}
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => onPageChange(currentPage - 1)}
                disabled={currentPage <= 1}
                className="h-8 w-8 p-0"
              >
                <ChevronLeft className="h-4 w-4" />
              </Button>
              
              <span className="text-sm text-muted-foreground">
                Page {currentPage} of {totalPages}
              </span>
              
              <Button
                variant="outline"
                size="sm"
                onClick={() => onPageChange(currentPage + 1)}
                disabled={currentPage >= totalPages}
                className="h-8 w-8 p-0"
              >
                <ChevronRight className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}