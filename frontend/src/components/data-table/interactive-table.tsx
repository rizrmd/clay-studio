"use client";

import { useMemo } from "react";
import { DataTable } from "./data-table-virtual";
import { TableColumn, TableConfig } from "./demo-data";

interface InteractiveTableProps {
  interactionId: string;
  title: string;
  data: {
    columns: TableColumn[];
    rows: any[];
    config?: TableConfig;
  };
  requiresResponse?: boolean;
}

export function InteractiveTable({
  interactionId: _interactionId,
  title,
  data,
  requiresResponse = false,
}: InteractiveTableProps) {
  // Prepare the data for DataTable component
  const tableData = useMemo(() => {
    if (!data?.rows) return [];
    return data.rows;
  }, [data?.rows]);

  const tableColumns = useMemo(() => {
    if (!data?.columns) return [];
    return data.columns;
  }, [data?.columns]);

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
    <div className="w-full space-y-2">
      {/* Title Header */}
      <div className="flex items-center gap-2 px-1">
        <span className="text-sm font-medium">{title}</span>
        <span className="text-xs text-muted-foreground">
          ({tableData.length} rows)
        </span>
      </div>

      {/* Data Table */}
      <div className="border rounded-lg overflow-hidden">
        <DataTable
          columns={tableColumns}
          data={tableData}
          config={tableConfig}
          className="max-h-[400px]"
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