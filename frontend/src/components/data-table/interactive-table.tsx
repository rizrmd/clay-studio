"use client";

import { useMemo, useState, useRef } from "react";
import { DataTable } from "./data-table";
import { TableColumn, TableConfig } from "./demo-data";
import { Maximize2, Minimize2, Download } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface InteractiveTableProps {
  interactionId: string;
  title: string;
  data: {
    columns: TableColumn[];
    rows: any[];
    config?: TableConfig;
  };
  requiresResponse?: boolean;
  persistenceKey?: string;
}

export function InteractiveTable({
  interactionId: _interactionId,
  title,
  data,
  requiresResponse = false,
  persistenceKey,
}: InteractiveTableProps) {
  const [isMaximized, setIsMaximized] = useState(false);
  const tableRef = useRef<any>(null);

  // Prepare the data for DataTable component
  const tableData = useMemo(() => {
    if (!data?.rows) return [];
    return data.rows;
  }, [data?.rows]);

  const tableColumns = useMemo(() => {
    if (!data?.columns) return [];
    return data.columns;
  }, [data?.columns]);

  // Function to download table data as CSV
  const downloadCSV = () => {
    if (!tableRef.current) {
      // Fallback to original data if table ref not available
      if (!tableData.length || !tableColumns.length) return;
      
      const headers = tableColumns.map(col => col.label || col.key).join(',');
      const csvRows = tableData.map(row => {
        return tableColumns.map(col => {
          const value = row[col.key];
          if (typeof value === 'string' && (value.includes(',') || value.includes('"') || value.includes('\n'))) {
            return `"${value.replace(/"/g, '""')}"`;
          }
          return value ?? '';
        }).join(',');
      });
      
      const csvContent = [headers, ...csvRows].join('\n');
      downloadFile(csvContent);
      return;
    }

    const table = tableRef.current;
    const filteredRows = table.getFilteredRowModel().rows;
    const visibleColumns = table.getVisibleLeafColumns();

    // Create CSV header from visible columns
    const headers = visibleColumns
      .filter((col: any) => col.id !== 'select') // Exclude selection column
      .map((col: any) => {
        const colDef = tableColumns.find(c => c.key === col.id);
        return colDef?.label || col.id;
      })
      .join(',');
    
    // Create CSV rows from filtered data
    const csvRows = filteredRows.map((row: any) => {
      return visibleColumns
        .filter((col: any) => col.id !== 'select')
        .map((col: any) => {
          const value = row.getValue(col.id);
          // Handle formatted values by converting them back to raw data
          let rawValue = value;
          
          // Handle special formatted values
          if (typeof value === 'object' && value?.__isDateRange) {
            rawValue = `${value.min} to ${value.max}`;
          } else if (typeof value === 'object' && value?.__hasError) {
            rawValue = value.value;
          }
          
          // Convert to string and escape CSV
          const stringValue = String(rawValue ?? '');
          if (stringValue.includes(',') || stringValue.includes('"') || stringValue.includes('\n')) {
            return `"${stringValue.replace(/"/g, '""')}"`;
          }
          return stringValue;
        })
        .join(',');
    });

    // Combine header and rows
    const csvContent = [headers, ...csvRows].join('\n');
    downloadFile(csvContent);
  };

  const downloadFile = (csvContent: string) => {
    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
    const link = document.createElement('a');
    const url = URL.createObjectURL(blob);
    link.setAttribute('href', url);
    link.setAttribute('download', `${title.replace(/[^a-zA-Z0-9]/g, '_')}.csv`);
    link.style.visibility = 'hidden';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  const tableConfig = useMemo(() => {
    // Merge default config with provided config
    const defaultConfig: TableConfig = {
      title: title,
      features: {
        sort: true,
        filter: true,
        columnVisibility: true,
        pivot: true,
        rowSelection: requiresResponse,
        globalSearch: false,
        stickyHeader: true,
        export: false,
        columnBorders: true,
        columnResize: true,
      },
    };

    if (data?.config) {
      return {
        ...defaultConfig,
        ...data.config,
        features: {
          ...defaultConfig.features,
          ...data.config.features,
        },
      };
    }

    return defaultConfig;
  }, [data?.config, title, requiresResponse]);

  // Handle empty data case
  if (!tableData.length || !tableColumns.length) {
    return (
      <div className="border rounded-lg p-6 bg-muted/30">
        <h3 className="font-medium text-sm mb-2">📋 {title}</h3>
        <div className="text-sm text-muted-foreground">
          No data available to display
        </div>
      </div>
    );
  }

  return (
    <div
      className={`w-full space-y-2 ${
        isMaximized
          ? "fixed inset-0 flex flex-col z-[100] pb-[20px] bg-background px-4"
          : ""
      }`}
    >
      {/* Title Header */}
      <div className={cn("flex items-center justify-between px-1", isMaximized && "pt-2")}>
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">{title}</span>
          <span className="text-xs text-muted-foreground whitespace-nowrap">
            ({tableData.length} rows)
          </span>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={downloadCSV}
            title="Download CSV"
          >
            <Download className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={() => setIsMaximized(!isMaximized)}
            title={isMaximized ? "Minimize" : "Maximize"}
          >
            {isMaximized ? (
              <Minimize2 className="h-4 w-4" />
            ) : (
              <Maximize2 className="h-4 w-4" />
            )}
          </Button>
        </div>
      </div>

      {/* Data Table */}
      <div className={cn(!isMaximized ? "border-2 rounded-md" : "relative flex-1")}>
        <DataTable
          columns={tableColumns}
          data={tableData}
          config={tableConfig}
          className={
            isMaximized ? "max-h-[calc(100vh-140px)]" : "max-h-[400px]"
          }
          persistenceKey={persistenceKey}
          ref={tableRef}
        />
      </div>

      {/* Response UI if needed */}
      {requiresResponse && (
        <div className="mt-3 p-3 border rounded-lg bg-muted/30">
          <div className="text-xs text-muted-foreground">
            Row selection is enabled. Select rows to respond with data.
          </div>
        </div>
      )}
    </div>
  );
}
