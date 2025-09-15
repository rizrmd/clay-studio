import {
  ColumnFiltersState,
  SortingState,
} from "@tanstack/react-table";
import { TableColumn, TableConfig } from "../demo-data";

export interface DataTableProps {
  columns: TableColumn[];
  data: any[];
  config?: TableConfig;
  className?: string;
  customSelectedRows?: Record<string, boolean>;
  persistenceKey?: string;
  serverSide?: {
    enabled: boolean;
    onSortingChange?: (sorting: SortingState) => void;
    onFiltersChange?: (filters: ColumnFiltersState) => void;
    onGlobalFilterChange?: (globalFilter: string) => void;
    onGetDistinctValues?: (column: string, search?: string) => Promise<string[]>;
    totalRows?: number;
  };
}

export interface PivotState {
  mode: boolean;
  columns: string[];
  aggregations: Record<string, string>;
  columnWidths: Record<string, number>;
}

export interface AggregationResult {
  __hasError?: boolean;
  __errorDetails?: {
    problematicValues: Array<{ value: any; index: number }>;
    validExamples: Array<{ value: any; index: number }>;
    validCount: number;
    totalCount: number;
  };
  value?: any;
}

export interface DateRange {
  __isDateRange: true;
  min: string;
  max: string;
}

export interface PivotRow {
  id: string;
  __isPivotRow: true;
  __pivotLevel: number;
  __rowCount: number;
  __groupKey: string;
  [key: string]: any;
}

export interface PivotTotal {
  id: string;
  __isPivotTotal: true;
  __rowCount: number;
  [key: string]: any;
}

export type ProcessedData = any[] | PivotRow[];