import { Row } from "@tanstack/react-table";
import { cn } from "@/lib/utils";
import { formatCellValue } from "../utils/formatters";
import { calculateStickyOffset } from "../utils";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";

interface PivotRowProps {
  row: Row<any>;
  columnDefs: any[];
  pivotColumns: string[];
  pivotColumnWidths: Record<string, number>;
  aggregations: Record<string, string>;
  config?: any;
}

export function PivotRow({
  row,
  columnDefs,
  pivotColumns,
  pivotColumnWidths,
  aggregations,
  config,
}: PivotRowProps) {
  const rowData = row?.original;
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
        const column = columnDefs.find((col) => col.key === cell.column.id);
        const value = rowData[cell.column.id];
        const currentAggregation =
          aggregations[cell.column.id] ||
          aggregations[column?.key || ""] ||
          "display";
        const isPivotCol =
          config?.features?.pivot !== false &&
          pivotColumns.includes(cell.column.id);
        const pivotIdx = pivotColumns.indexOf(cell.column.id);

        const cellStickyOffset = calculateStickyOffset(
          pivotColumns,
          pivotColumnWidths,
          columnDefs,
          cell.column.id
        );

        return (
          <td
            key={cell.id}
            className={cn(
              "px-2 py-1 align-middle",
              column?.aggregatable &&
                "font-semibold text-blue-700 dark:text-blue-400",
              pivotColumns.includes(cell.column.id) &&
                pivotLevel > 0 &&
                "pl-6",
              isPivotCol &&
                "sticky bg-blue-50 dark:bg-blue-950/40 z-10 shadow-[2px_0_4px_-2px_rgba(0,0,0,0.1)]",
              config?.features?.columnBorders !== false && "border-r"
            )}
            style={{
              width: cell.column.getSize(),
              minWidth: cell.column.getSize(),
              ...(isPivotCol && {
                left: pivotIdx === 0 ? 0 : `${cellStickyOffset}px`,
              }),
            }}
          >
            {cell.column.id === "select" ? null : pivotColumns.includes(
                cell.column.id
              ) ? (
              // Render pivot column value (not aggregated)
              value !== null && value !== undefined && value !== "" ? (
                <div className="flex items-center gap-1">
                  {pivotLevel > 0 &&
                    pivotColumns.indexOf(cell.column.id) === pivotLevel && (
                      <div className="absolute ml-[-25px]">→</div>
                    )}
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
                <div className="flex items-center gap-1 text-xs">
                  <span className="whitespace-nowrap">
                    {formatCellValue(value.min, "date")}
                  </span>
                  <span className="text-muted-foreground">→</span>
                  <span className="whitespace-nowrap">
                    {formatCellValue(value.max, "date")}
                  </span>
                </div>
              ) : value?.__hasError ? (
                <Popover>
                  <PopoverTrigger asChild>
                    <button className="flex items-center gap-1 text-orange-600 dark:text-orange-400 hover:underline">
                      <span className="text-xs">⚠️</span>
                      {value.value !== null && (
                        <span className="text-xs">
                          (
                          {formatCellValue(
                            value.value,
                            currentAggregation === "count" ? "number" : column?.data_type,
                            column?.format,
                            column?.currency,
                            column?.currencyDisplay
                          )}
                          )
                        </span>
                      )}
                    </button>
                  </PopoverTrigger>
                  <PopoverContent className="w-96 p-3">
                    <div className="space-y-3">
                      <div className="flex items-center justify-between">
                        <div className="font-semibold text-sm">
                          Data Quality Issue
                        </div>
                        <div className="text-xs text-muted-foreground">
                          {value.__errorDetails.validCount} valid /{" "}
                          {value.__errorDetails.totalCount} total values
                        </div>
                      </div>
                      {/* Additional error details UI would go here */}
                    </div>
                  </PopoverContent>
                </Popover>
              ) : value === null ? (
                <span className="text-muted-foreground">—</span>
              ) : currentAggregation === "display" ? (
                formatCellValue(
                  value,
                  column?.data_type,
                  column?.format,
                  column?.currency,
                  column?.currencyDisplay
                )
              ) : currentAggregation === "count" ? (
                <div className="flex items-center gap-1">
                  <span className="text-xs text-muted-foreground">count:</span>
                  <span>{value.toLocaleString()}</span>
                </div>
              ) : typeof value === "number" && !isNaN(value) ? (
                <div className="flex items-center gap-1">
                  <span className="text-xs text-muted-foreground">
                    {currentAggregation}:
                  </span>
                  {formatCellValue(
                    value,
                    column?.data_type,
                    column?.format,
                    column?.currency,
                    column?.currencyDisplay
                  )}
                </div>
              ) : (
                <div className="flex items-center gap-1">
                  <span className="text-xs text-muted-foreground">
                    {currentAggregation}:
                  </span>
                  <span>{String(value || "")}</span>
                </div>
              )
            ) : (
              String(value || "")
            )}
          </td>
        );
      })}
    </tr>
  );
}