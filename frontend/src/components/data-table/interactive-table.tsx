"use client";

import { useMemo, useState } from "react";
import { DataTable } from "./data-table-virtual";
import { TableColumn, TableConfig } from "./demo-data";
import { Maximize2, Minimize2 } from "lucide-react";
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
}

export function InteractiveTable({
  interactionId: _interactionId,
  title,
  data,
  requiresResponse = false,
}: InteractiveTableProps) {
  const [isMaximized, setIsMaximized] = useState(false);
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
    <div
      className={`w-full space-y-2 ${
        isMaximized
          ? "fixed inset-0 flex flex-col z-[100] bottom-[70px] bg-background px-4"
          : ""
      }`}
    >
      {/* Title Header */}
      <div className={cn("flex items-center justify-between px-1", isMaximized && "pt-2")}>
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">{title}</span>
          <span className="text-xs text-muted-foreground">
            ({tableData.length} rows)
          </span>
        </div>
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

      {/* Data Table */}
      <div className={cn(!isMaximized ? "border-2 rounded-md" : "overflow-hidden relative flex-1")}>
        <DataTable
          columns={tableColumns}
          data={tableData}
          config={tableConfig}
          className={
            isMaximized ? "max-h-[calc(100vh-120px)]" : "max-h-[400px]"
          }
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
