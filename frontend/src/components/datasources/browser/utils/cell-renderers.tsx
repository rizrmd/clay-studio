import React from "react";
import { ClickableCell } from "../components/clickable-cell";
import type { TableColumn } from "@/components/data-table/demo-data";

export interface CellRendererContext {
  isEditable: boolean;
  onCellEdit?: (rowId: string, columnKey: string, newValue: any) => void;
  editingDisabled?: boolean;
}

export function createClickableCellRenderer(
  context: CellRendererContext
) {
  return (value: any, row: any, column: TableColumn, defaultRenderer: (value: any) => React.ReactNode) => {
    if (!context.isEditable || context.editingDisabled) {
      // Use default rendering for non-editable cells
      return defaultRenderer(value);
    }

    const rowId = row.id || row._id || String(row[Object.keys(row)[0]]);

    return (
      <ClickableCell
        value={value}
        rowId={rowId}
        columnKey={column.key}
        dataType={column.data_type}
        nullable={column.nullable}
        onCellEdit={(rowId, columnKey, newValue) => {
          context.onCellEdit?.(rowId, columnKey, newValue);
        }}
        defaultRenderer={defaultRenderer}
        disabled={context.editingDisabled}
      />
    );
  };
}

export function createReadOnlyCellRenderer() {
  return (value: any, _row: any, _column: TableColumn, defaultRenderer: (value: any) => React.ReactNode) => {
    return defaultRenderer(value);
  };
}

export function enhanceColumnsWithCellRenderers(
  columns: TableColumn[],
  context: CellRendererContext
): TableColumn[] {
  const cellRenderer = context.isEditable 
    ? createClickableCellRenderer(context)
    : createReadOnlyCellRenderer();

  return columns.map(column => ({
    ...column,
    cellRenderer: column.cellRenderer || cellRenderer,
  }));
}