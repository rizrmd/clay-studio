import * as React from "react";
import { flexRender, HeaderGroup } from "@tanstack/react-table";
import { cn } from "@/lib/utils";
import { css } from "goober";
import { DataTableColumnsMenu } from "./columns-menu";
import { calculateStickyOffset } from "../utils";

interface TableHeaderProps {
  headerGroups: HeaderGroup<any>[];
  pivotColumns: string[];
  pivotColumnWidths: Record<string, number>;
  columnDefs: any[];
  config?: any;
  headerRefs: React.MutableRefObject<Record<string, HTMLTableCellElement | null>>;
  table: any;
}

export function TableHeader({
  headerGroups,
  pivotColumns,
  pivotColumnWidths,
  columnDefs,
  config,
  headerRefs,
  table,
}: TableHeaderProps) {
  return (
    <thead className="sticky top-0 bg-background z-30">
      {headerGroups.map((headerGroup) => (
        <tr
          key={headerGroup.id}
          className="relative bg-white after:content-[''] after:absolute after:bottom-0 after:left-0 after:right-0 after:h-[1px] after:bg-border"
        >
          {headerGroup.headers.map((header, index) => {
            const isLastColumn = index === headerGroup.headers.length - 1;
            const isPivotColumn =
              config?.features?.pivot !== false &&
              pivotColumns.includes(header.column.id);
            const pivotIndex = pivotColumns.indexOf(header.column.id);

            // Calculate sticky offset for multiple pivot columns using stored widths
            const stickyOffset = calculateStickyOffset(
              pivotColumns,
              pivotColumnWidths,
              columnDefs,
              header.column.id
            );

            return (
              <th
                key={header.id}
                ref={(el) => {
                  // Store ref without interfering with other handlers
                  if (el && header.column.id) {
                    headerRefs.current[header.column.id] = el;
                  }
                }}
                className={cn(
                  "text-left align-middle font-medium text-muted-foreground relative transition-all",
                  isPivotColumn &&
                    "sticky bg-background z-40 shadow-[2px_0_4px_-2px_rgba(0,0,0,0.1)]",
                  config?.features?.columnBorders !== false && "border-r",
                  css`
                    > * {
                      min-height: 35px;
                    }
                  `
                )}
                style={{
                  width: header.getSize(),
                  minWidth: header.getSize(),
                  position: isPivotColumn ? 'sticky' : 'relative',
                  ...(isPivotColumn && {
                    left: pivotIndex === 0 ? 0 : `${stickyOffset}px`,
                  }),
                }}
              >
                <div className="th-l1 flex flex-1 items-stretch group">
                  <div
                    className={cn(
                      "th-l2 flex items-center flex-1",
                      css`
                        > div {
                          flex: 1;
                          display: flex;
                          flex-direction: column;

                          .button {
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
                  {/* Column resize handle */}
                  {header.column.getCanResize() && (
                    <div
                      onMouseDown={header.getResizeHandler()}
                      onTouchStart={header.getResizeHandler()}
                      className={cn(
                        "absolute top-0 right-0 h-full w-1 cursor-col-resize select-none touch-none bg-transparent hover:bg-blue-500 transition-colors",
                        header.column.getIsResizing() && "bg-blue-500 cursor-col-resize"
                      )}
                      style={{
                        transform: 'translateX(50%)',
                        cursor: 'col-resize',
                      }}
                    />
                  )}
                  {isLastColumn && <DataTableColumnsMenu table={table} />}
                </div>
              </th>
            );
          })}
        </tr>
      ))}
    </thead>
  );
}