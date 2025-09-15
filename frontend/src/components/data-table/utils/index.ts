import { rankItem } from "@tanstack/match-sorter-utils";
import { FilterFn } from "@tanstack/react-table";

// Fuzzy filter function
export const fuzzyFilter: FilterFn<any> = (row, columnId, value, addMeta) => {
  const itemRank = rankItem(row.getValue(columnId), value);
  addMeta({ itemRank });
  return itemRank.passed;
};

// Helper to get column width consistently
export const getColumnWidth = (columnId: string, columnDefs: any[]) => {
  const colDef = columnDefs.find((c) => c.key === columnId);
  return colDef?.width || 150;
};

// Calculate sticky offset for pivot columns
export const calculateStickyOffset = (
  pivotColumns: string[],
  pivotColumnWidths: Record<string, number>,
  columnDefs: any[],
  targetColumnId: string
): number => {
  const pivotIndex = pivotColumns.indexOf(targetColumnId);
  if (pivotIndex <= 0) return 0;

  let stickyOffset = 0;
  for (let i = 0; i < pivotIndex; i++) {
    stickyOffset +=
      pivotColumnWidths[pivotColumns[i]] ||
      getColumnWidth(pivotColumns[i], columnDefs);
  }
  return stickyOffset;
};