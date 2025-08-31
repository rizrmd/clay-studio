"use client";

import { DataTable } from "./data-table-virtual";
import { demoColumns, generateDemoData, TableConfig } from "./demo-data";
import { useState } from "react";
import { 
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Label } from "@/components/ui/label";

export function TableDemo() {
  const [data] = useState(() => generateDemoData(100));
  const [pivotMode, setPivotMode] = useState(true); // Start in pivot mode

  const config: TableConfig = {
    features: {
      sort: true,
      filter: true,
      export: true,
      columnVisibility: true,
      rowSelection: true,
      globalSearch: true,
      stickyHeader: true,
      pivot: true
    },
    initialState: {
      sorting: [{ column: "date", direction: "desc" }, { column: "revenue", direction: "desc" }],
      filters: [{ column: "in_stock", value: true }],
      columnVisibility: { "id": false },
      pivot: pivotMode ? {
        enabled: true,
        columns: ["category", "status"],
        aggregations: { "price": "avg", "quantity": "sum", "revenue": "sum" }
      } : { enabled: false, columns: [] }
    }
  };

  return (
    <div className="p-8 space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Table with Pivot Mode Demo</h1>
          <p className="text-muted-foreground">
            Transform your data with pivot tables for powerful aggregation and analysis
          </p>
        </div>
        
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <Label htmlFor="pivot-mode">Pivot Mode:</Label>
            <Select value={pivotMode ? "pivot" : "none"} onValueChange={(value) => setPivotMode(value === "pivot")}>
              <SelectTrigger id="pivot-mode" className="w-[180px]">
                <SelectValue placeholder="Select mode" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="none">Normal View</SelectItem>
                <SelectItem value="pivot">Pivot Table</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
      </div>

      <div className="border rounded-lg">
        <DataTable
          columns={demoColumns}
          data={data}
          config={config}
          className="h-[600px]"
        />
      </div>

      <div className="mt-8 p-4 bg-muted rounded-lg">
        <h2 className="font-semibold mb-2">How to use Pivot Mode:</h2>
        <ul className="space-y-2 text-sm">
          <li>
            <strong>Enable Pivot:</strong> Click on any column header that can be pivoted (Category, Status, etc.) 
            and select "Pivot by this column" from the dropdown menu.
          </li>
          <li>
            <strong>Data Aggregation:</strong> Once in pivot mode, numeric columns (Price, Quantity, Revenue) 
            are automatically aggregated using their default functions (sum, avg, etc.).
          </li>
          <li>
            <strong>Change Aggregations:</strong> Click on any numeric column header → Aggregation → 
            Choose from sum, avg, count, min, or max to change how values are calculated.
          </li>
          <li>
            <strong>Total Row:</strong> A TOTAL row is automatically added at the bottom showing 
            grand totals for all aggregated columns.
          </li>
        </ul>
        
        <div className="mt-4 p-3 bg-blue-50 dark:bg-blue-950/20 rounded border border-blue-200 dark:border-blue-800">
          <p className="text-xs text-blue-700 dark:text-blue-400">
            <strong>💡 New Features:</strong>
          </p>
          <ul className="text-xs text-blue-700 dark:text-blue-400 mt-1 space-y-1">
            <li>• Multi-level pivot support - pivot by multiple columns (1st, 2nd, 3rd indicators)</li>
            <li>• Sticky pivot columns - remain visible when scrolling horizontally</li>
            <li>• Multi-currency support - Price in USD ($), Revenue in IDR (Rp)</li>
            <li>• Initial configurations - table starts with predefined sort, filter, and pivot settings</li>
            <li>• Date range display - dates show min-max range in pivot mode</li>
            <li>• Sticky total row at bottom with aggregation summaries</li>
          </ul>
        </div>
        
        <div className="mt-2 p-3 bg-orange-50 dark:bg-orange-950/20 rounded border border-orange-200 dark:border-orange-800">
          <p className="text-xs text-orange-700 dark:text-orange-400">
            <strong>⚠️ Demo Data:</strong> This demo intentionally includes some problematic data (rows 20, 40, 60, 80, 100) 
            with non-numeric values in numeric columns to demonstrate error handling. 
            Check the browser console to see detailed warnings about which rows have issues. 
            In pivot mode, cells with aggregation errors will show a warning indicator.
          </p>
        </div>
      </div>
    </div>
  );
}