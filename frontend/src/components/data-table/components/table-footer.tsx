import { cn } from "@/lib/utils";
import { formatCellValue } from "../utils/formatters";
import { calculateStickyOffset } from "../utils";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";

interface TableFooterProps {
  table: any;
  totalRow: any;
  pivotColumns: string[];
  pivotColumnWidths: Record<string, number>;
  columnDefs: any[];
  aggregations: Record<string, string>;
  config?: any;
}

export function TableFooter({
  table,
  totalRow,
  pivotColumns,
  pivotColumnWidths,
  columnDefs,
  aggregations,
  config,
}: TableFooterProps) {
  if (!totalRow || config?.features?.pivot === false) {
    return null;
  }

  return (
    <tfoot className="sticky bottom-0 z-10">
      <tr className="border-t-2 border-b font-bold bg-muted">
        {table.getVisibleLeafColumns().map((column: any) => {
          const columnDef = columnDefs.find((col) => col.key === column.id);
          const value = totalRow[column.id];
          const currentAggregation = aggregations[columnDef?.key || ""] || "display";
          const isPivotCol =
            config?.features?.pivot !== false && pivotColumns.includes(column.id);
          const pivotIdx = pivotColumns.indexOf(column.id);

          const cellStickyOffset = calculateStickyOffset(
            pivotColumns,
            pivotColumnWidths,
            columnDefs,
            column.id
          );

          return (
            <td
              key={column.id}
              className={cn(
                "px-2 py-2 align-middle",
                isPivotCol &&
                  "sticky bg-muted z-20 shadow-[2px_0_4px_-2px_rgba(0,0,0,0.1)]",
                config?.features?.columnBorders !== false && "border-r"
              )}
              style={{
                width: column.getSize(),
                minWidth: column.getSize(),
                ...(isPivotCol && {
                  left: pivotIdx === 0 ? 0 : `${cellStickyOffset}px`,
                }),
              }}
            >
              {column.id === "select" ? null : value?.__isDateRange ? (
                <div className="flex items-center gap-1 text-xs font-semibold">
                  <span className="whitespace-nowrap">
                    {formatCellValue(value.min, "date")}
                  </span>
                  <span className="text-muted-foreground">→</span>
                  <span className="whitespace-nowrap">
                    {formatCellValue(value.max, "date")}
                  </span>
                </div>
              ) : value?.__hasError ? (
                <div className="flex items-center gap-1">
                  <Popover>
                    <PopoverTrigger asChild>
                      <button className="text-orange-600 dark:text-orange-400 hover:opacity-70">
                        <span className="text-xs">⚠️</span>
                      </button>
                    </PopoverTrigger>
                    <PopoverContent className="w-80">
                      <div className="space-y-2">
                        <div className="font-semibold text-sm">
                          Data Quality Issue in Total
                        </div>
                        {/* Additional error details would go here */}
                      </div>
                    </PopoverContent>
                  </Popover>
                  {value.value !== null && (
                    <span>
                      {currentAggregation === "count"
                        ? value.value.toLocaleString()
                        : formatCellValue(
                            value.value,
                            columnDef?.data_type,
                            columnDef?.format,
                            columnDef?.currency,
                            columnDef?.currencyDisplay
                          )}
                    </span>
                  )}
                </div>
              ) : currentAggregation === "display" ? (
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
  );
}