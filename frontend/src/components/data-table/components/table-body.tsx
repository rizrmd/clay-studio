import { Row } from "@tanstack/react-table";
import { PivotRow } from "./pivot-row";
import { DataRow } from "./data-row";

interface TableBodyProps {
  virtualRows: any[];
  rowsToRender: Row<any>[];
  useVirtualScrolling: boolean;
  paddingTop: number;
  paddingBottom: number;
  pivotColumns: string[];
  pivotColumnWidths: Record<string, number>;
  columnDefs: any[];
  aggregations: Record<string, string>;
  pivotMode: boolean;
  config?: any;
  customSelectedRows?: Record<string, boolean>;
}

export function TableBody({
  virtualRows,
  rowsToRender,
  useVirtualScrolling,
  paddingTop,
  paddingBottom,
  pivotColumns,
  pivotColumnWidths,
  columnDefs,
  aggregations,
  pivotMode,
  config,
  customSelectedRows,
}: TableBodyProps) {
  const rowsToIterate = useVirtualScrolling
    ? virtualRows
    : rowsToRender.map((_, index) => ({ index }));

  return (
    <tbody>
      {useVirtualScrolling && paddingTop > 0 && (
        <tr>
          <td style={{ height: `${paddingTop}px` }} />
        </tr>
      )}
      {rowsToIterate.map((virtualRow) => {
        const row = rowsToRender[virtualRow.index];
        const rowData = row?.original;

        // Check if this is a pivot row
        if (rowData?.__isPivotRow) {
          return (
            <PivotRow
              key={row.id}
              row={row}
              columnDefs={columnDefs}
              pivotColumns={pivotColumns}
              pivotColumnWidths={pivotColumnWidths}
              aggregations={aggregations}
              config={config}
            />
          );
        }

        // Skip total row in virtual scroll (rendered separately in footer)
        if (rowData?.__isPivotTotal) {
          return null;
        }

        // Regular data row
        return (
          <DataRow
            key={row.id}
            row={row}
            pivotColumns={pivotColumns}
            pivotColumnWidths={pivotColumnWidths}
            columnDefs={columnDefs}
            pivotMode={pivotMode}
            config={config}
            customSelectedRows={customSelectedRows}
          />
        );
      })}
      {useVirtualScrolling && paddingBottom > 0 && (
        <tr>
          <td style={{ height: `${paddingBottom}px` }} />
        </tr>
      )}
    </tbody>
  );
}