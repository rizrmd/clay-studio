import React from "react";

export interface TableColumn {
  key: string;
  label: string;
  data_type: "string" | "number" | "date" | "boolean" | "currency";
  sortable?: boolean;
  filterable?: boolean;
  width?: number;
  format?: string;
  nullable?: boolean;
  // Currency settings
  currency?:
    | "USD"
    | "EUR"
    | "GBP"
    | "JPY"
    | "CNY"
    | "IDR"
    | "SGD"
    | "MYR"
    | "THB"
    | "VND"
    | "PHP";
  currencyDisplay?: "symbol" | "code" | "name";
  // Grouping properties
  group?: string; // Column header group name
  groupable?: boolean; // Can be used for row grouping
  aggregatable?: boolean; // Can be aggregated in pivot mode
  aggregation?: "display" | "sum" | "avg" | "count" | "min" | "max"; // Default aggregation
  // Custom cell renderer
  cellRenderer?: (
    value: any, 
    row: any, 
    column: TableColumn, 
    defaultRenderer: (value: any) => React.ReactNode
  ) => React.ReactNode;
  // Custom header renderer
  headerRenderer?: () => React.ReactNode;
}

export type GroupingMode = "none" | "pivot";

export interface InitialState {
  // Sorting configuration
  sorting?: Array<{
    column: string;
    direction: "asc" | "desc";
  }>;

  // Filter configuration
  filters?: Array<{
    column: string;
    value: any;
  }>;

  // Global filter
  globalFilter?: string;

  // Column visibility
  columnVisibility?: Record<string, boolean>;

  // Pivot configuration
  pivot?: {
    enabled: boolean;
    columns: string[];
    aggregations?: Record<
      string,
      "display" | "sum" | "avg" | "count" | "min" | "max"
    >;
  };

  // Pagination configuration
  pagination?: {
    pageIndex: number;
    pageSize: number;
  };
}

export interface TableConfig {
  title?: string;
  description?: string;

  // Feature flags
  features?: {
    sort?: boolean;
    filter?: boolean;
    export?: boolean;
    columnVisibility?: boolean;
    rowSelection?: boolean;
    globalSearch?: boolean;
    stickyHeader?: boolean;
    pivot?: boolean;
    pagination?: boolean;
    columnBorders?: boolean;
    columnResize?: boolean;
  };

  // Initial state - all configurations in one place
  initialState?: InitialState;
}

export const demoColumns: TableColumn[] = [
  {
    key: "id",
    label: "ID",
    data_type: "number",
    sortable: true,
    filterable: true,
  },
  {
    key: "product",
    label: "Product",
    data_type: "string",
    filterable: true,
    sortable: true,
  },
  {
    key: "category",
    label: "Category",
    data_type: "string",
    filterable: true,
    sortable: true,
  },
  {
    key: "price",
    label: "Price (USD)",
    data_type: "currency",
    format: "currency",
    currency: "USD",
    currencyDisplay: "symbol",
    sortable: true,
  },
  {
    key: "quantity",
    label: "Quantity",
    data_type: "number",
    sortable: true,
  },
  {
    key: "revenue",
    label: "Revenue (IDR)",
    data_type: "currency",
    format: "currency",
    currency: "IDR",
    currencyDisplay: "code",
    sortable: true,
  },
  {
    key: "date",
    label: "Date",
    data_type: "date",
    sortable: true,
  },
  {
    key: "status",
    label: "Status",
    data_type: "string",
    filterable: true,
    sortable: true,
  },
  {
    key: "in_stock",
    label: "In Stock",
    data_type: "boolean",
    sortable: true,
    filterable: true,
  },
];

const products = [
  "Widget A",
  "Widget B",
  "Gadget X",
  "Gadget Y",
  "Tool Z",
  "Device M",
  "Component N",
  "Module P",
];
const categories = [
  "Electronics",
  "Hardware",
  "Software",
  "Accessories",
  "Services",
  "Components",
  "Systems",
];
const statuses = [
  "Pending",
  "Shipped",
  "Delivered",
  "Cancelled",
  "Returned",
  "Processing",
  "On Hold",
];

export function generateDemoData(count: number) {
  return Array.from({ length: count }, (_, i) => {
    // Occasionally introduce problematic data for testing error handling
    const hasProblematicData = i % 20 === 0 && i > 0; // Every 20th row has issues

    return {
      id: i + 1,
      product: products[Math.floor(Math.random() * products.length)],
      category: categories[Math.floor(Math.random() * categories.length)],
      price: hasProblematicData ? "N/A" : Math.random() * 1000,
      quantity: hasProblematicData
        ? undefined
        : Math.floor(Math.random() * 100) + 1,
      revenue: hasProblematicData ? null : Math.random() * 100000000, // IDR values in millions
      date: new Date(
        2024,
        Math.floor(Math.random() * 12),
        Math.floor(Math.random() * 28) + 1
      ).toISOString(),
      status: statuses[Math.floor(Math.random() * statuses.length)],
      in_stock: Math.random() > 0.3,
    };
  });
}

// Generate clean demo data without any issues
export function generateCleanDemoData(count: number) {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    product: products[Math.floor(Math.random() * products.length)],
    category: categories[Math.floor(Math.random() * categories.length)],
    price: Math.random() * 1000,
    quantity: Math.floor(Math.random() * 100) + 1,
    revenue: Math.random() * 100000000, // IDR values in millions
    date: new Date(
      2024,
      Math.floor(Math.random() * 12),
      Math.floor(Math.random() * 28) + 1
    ).toISOString(),
    status: statuses[Math.floor(Math.random() * statuses.length)],
    in_stock: Math.random() > 0.3,
  }));
}
