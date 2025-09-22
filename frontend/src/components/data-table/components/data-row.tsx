import { Row, flexRender } from "@tanstack/react-table";
import { cn } from "@/lib/utils";
import { calculateStickyOffset } from "../utils";

interface DataRowProps {
  row: Row<any>;
  pivotColumns: string[];
  pivotColumnWidths: Record<string, number>;
  columnDefs: any[];
  pivotMode: boolean;
  config?: any;
  customSelectedRows?: Record<string, boolean>;
}

export function DataRow({
  row,
  pivotColumns,
  pivotColumnWidths,
  columnDefs,
  pivotMode,
  config,
  customSelectedRows,
}: DataRowProps) {
  // Try multiple ways to get the row ID
  const rowId = row.original?.id ?? row.original?._id ?? row.id;
  const isCustomSelected = customSelectedRows?.[String(rowId)];
  
  return (
    <tr
      key={row.id}
      className={cn(
        "border-b transition-colors hover:bg-muted/50",
        row.getIsSelected() && "bg-muted"
      )}
      data-selected={isCustomSelected ? "true" : undefined}
    >
      {row.getVisibleCells().map((cell) => {
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
              config?.features?.pivot !== false &&
                isPivotCol &&
                pivotMode &&
                "sticky bg-background z-10 shadow-[2px_0_4px_-2px_rgba(0,0,0,0.1)]",
              config?.features?.columnBorders !== false && "border-r"
            )}
            style={{
              width: cell.column.getSize(),
              minWidth: cell.column.getSize(),
              ...(config?.features?.pivot !== false &&
                isPivotCol &&
                pivotMode && {
                  left: pivotIdx === 0 ? 0 : `${cellStickyOffset}px`,
                }),
            }}
          >
            {flexRender(cell.column.columnDef.cell, cell.getContext())}
          </td>
        );
      })}
    </tr>
  );
}