import { useMemo, Suspense } from "react";
import { AskUser } from "./ask-user";
// import { WebSocketService } from "@/lib/services/websocket-service";
import { InteractiveTable } from "@/components/data-table/interactive-table";
import type { ChartType } from "@/components/data-chart/chart-types";
import { ChartDisplay } from "@/components/data-chart";
import { parseMcpToolResult } from "../../../tool/tool-call-utils";

// Stub implementation
const WebSocketService = {
  getInstance: () => ({
    sendMessage: (_msg: any) => {},
    sendAskUserResponse: (_interactionId: string, _response: any) => {},
    isConnected: () => false,
  }),
};

interface InteractionSpec {
  interaction_id: string;
  interaction_type:
    | "buttons"
    | "checkbox"
    | "input"
    | "chart"
    | "table"
    | "show_chart"
    | "show_table"
    | "markdown"
    | "excel_export";
  title: string;
  data: any;
  headers?: string[];
  chart_type?: string;
  options?: any;
  requires_response: boolean;
  created_at: string;
}

interface InteractionRendererProps {
  toolOutput: string | any;
  onAskUserSubmit?: (response: string | string[]) => void;
  isDisabled?: boolean;
  hasResponse?: boolean;
  selectedResponse?: string | string[];
  onScroll?: () => void;
}

export function InteractionRenderer({
  toolOutput,
  onAskUserSubmit,
  isDisabled = false,
  hasResponse = false,
  selectedResponse,
  onScroll,
}: InteractionRendererProps) {
  // Parse the interaction spec from the tool output
  const interactionSpec = useMemo(() => {
    if (!toolOutput) return null;

    // Handle array outputs (take first element)
    let actualOutput = toolOutput;
    if (Array.isArray(toolOutput) && toolOutput.length > 0) {
      actualOutput = toolOutput[0];
    }

    try {
      // If it's already an object with the right structure
      if (typeof actualOutput === "object" && actualOutput.interaction_type) {
        return actualOutput as InteractionSpec;
      }

      // Handle raw MCP interaction parameter formats
      if (typeof actualOutput === "object") {
        // show_table format: {data: {columns: [...], rows: [...]}, title: "..."}
        if (actualOutput.data && 
            actualOutput.data.columns && 
            actualOutput.data.rows &&
            Array.isArray(actualOutput.data.columns) &&
            Array.isArray(actualOutput.data.rows)) {
          return {
            interaction_type: "show_table",
            title: actualOutput.title || "Data Table",
            data: actualOutput.data,
            interaction_id: `table-${Date.now()}`,
            requires_response: false,
            created_at: new Date().toISOString(),
          } as InteractionSpec;
        }
        
        // show_chart format: {data: {categories: [...], series: [...]}, chart_type: "...", title: "..."}
        if (actualOutput.data && 
            actualOutput.chart_type &&
            (actualOutput.data.categories || actualOutput.data.series)) {
          return {
            interaction_type: "show_chart",
            title: actualOutput.title || "Chart",
            chart_type: actualOutput.chart_type,
            data: actualOutput.data,
            interaction_id: `chart-${Date.now()}`,
            requires_response: false,
            created_at: new Date().toISOString(),
          } as InteractionSpec;
        }

        // excel_export format: {status: "success", download_url: "...", filename: "...", ...}
        if (actualOutput.status === "success" && actualOutput.download_url) {
          console.log('Found Excel export data:', actualOutput);
          return {
            interaction_type: "excel_export",
            title: "Excel Export",
            data: actualOutput,
            interaction_id: `excel-${Date.now()}`,
            requires_response: false,
            created_at: new Date().toISOString(),
          } as InteractionSpec;
        }

        // If it's an object with a 'text' property, use that
        if (actualOutput.text) {
          actualOutput = actualOutput.text;
        }
      }

      // If it's a string, try to extract JSON from it
      if (typeof actualOutput === "string") {
        // First try parsing as JSON directly
        try {
          const jsonOutput = JSON.parse(actualOutput);
          if (jsonOutput.status === "success" && jsonOutput.download_url) {
            return {
              interaction_type: "excel_export",
              title: "Excel Export",
              data: jsonOutput,
              interaction_id: `excel-${Date.now()}`,
              requires_response: false,
              created_at: new Date().toISOString(),
            } as InteractionSpec;
          }
        } catch (e) {
          // Fall back to MCP tool result parsing
        }
        
        return parseMcpToolResult(actualOutput) as InteractionSpec;
      }
    } catch (error) {
      console.error("Failed to parse interaction spec:", error);
    }

    return null;
  }, [toolOutput]);

  if (!interactionSpec) {
    // Not an interaction tool output, return null
    return null;
  }

  // Render based on interaction type
  switch (interactionSpec.interaction_type) {
    case "buttons":
    case "checkbox":
    case "input":
      // Use the existing AskUser component for user input interactions
      return (
        <AskUser
          promptType={interactionSpec.interaction_type}
          title={interactionSpec.title}
          options={interactionSpec.data.options}
          inputType={interactionSpec.data.input_type}
          placeholder={interactionSpec.data.placeholder}
          toolUseId={interactionSpec.interaction_id}
          onSubmit={(response) => {
            // Send response via WebSocket
            const wsService = WebSocketService.getInstance();
            wsService.sendAskUserResponse(
              interactionSpec.interaction_id,
              response
            );

            // Also call the provided callback if any
            if (onAskUserSubmit) {
              onAskUserSubmit(response);
            }
          }}
          isDisabled={isDisabled}
          hasResponse={hasResponse}
          selectedResponse={selectedResponse}
          onScroll={onScroll}
        />
      );

    case "chart":
      // Render interactive chart using ChartDisplay component with lazy loading
      return (
        <Suspense
          fallback={
            <div className="flex items-center justify-center h-64 text-muted-foreground">
              Loading chart...
            </div>
          }
        >
          <ChartDisplay
            interactionId={interactionSpec.interaction_id}
            title={interactionSpec.title}
            chartType={interactionSpec.data.chart_type || "line"}
            data={interactionSpec.data}
            options={interactionSpec.options}
            requiresResponse={interactionSpec.requires_response}
          />
        </Suspense>
      );

    case "table":
      // Render interactive table using DataTable component
      return (
        <InteractiveTable
          interactionId={interactionSpec.interaction_id}
          title={interactionSpec.title}
          data={interactionSpec.data}
          requiresResponse={interactionSpec.requires_response}
        />
      );

    case "show_table":
      // Render interactive table using DataTable component for show_table interaction
      const tableData = useMemo(() => {
        const data = interactionSpec.data;
        const headers = interactionSpec.headers;

        // Handle the case where data is already an object with columns and rows
        if (data && typeof data === "object" && data.columns && data.rows) {
          // Data is already in the expected format: {columns: [...], rows: [...]}
          const columns = data.columns.map((col: any, index: number) => ({
            key: col.key || col.id || `col_${index}`,
            label: col.label || col.name || col.header || `Column ${index + 1}`,
            data_type: col.data_type || col.type || "string" as const,
            width: col.width || 150,
            sortable: col.sortable !== false,
            filterable: col.filterable !== false,
          }));

          return { columns, rows: data.rows || [] };
        }

        // Legacy handling for array data
        if (!Array.isArray(data) || data.length === 0) {
          return { columns: [], rows: [] };
        }

        // If headers are provided, use them. Otherwise infer from first row
        let columns;
        if (headers && headers.length > 0) {
          columns = headers.map((header, index) => ({
            key: `col_${index}`,
            label: header,
            data_type: "string" as const,
            width: 150,
            sortable: true,
            filterable: true,
          }));
        } else {
          // Infer columns from first data row
          const firstRow = data[0];
          if (typeof firstRow === "object" && firstRow !== null) {
            columns = Object.keys(firstRow).map((key) => ({
              key,
              label: key,
              data_type: "string" as const,
              width: 150,
              sortable: true,
              filterable: true,
            }));
          } else {
            // Data is array of primitives, create generic columns
            columns = Array.isArray(firstRow)
              ? firstRow.map((_, index) => ({
                  key: `col_${index}`,
                  label: `Column ${index + 1}`,
                  data_type: "string" as const,
                  width: 150,
                  sortable: true,
                  filterable: true,
                }))
              : [
                  {
                    key: "value",
                    label: "Value",
                    data_type: "string" as const,
                    width: 150,
                    sortable: true,
                    filterable: true,
                  },
                ];
          }
        }

        // Transform data rows
        const rows = data.map((row) => {
          if (typeof row === "object" && row !== null && !Array.isArray(row)) {
            // Row is already an object
            return row;
          } else if (Array.isArray(row)) {
            // Convert array to object with column keys
            const rowObj: any = {};
            columns.forEach((col, index) => {
              rowObj[col.key] = row[index];
            });
            return rowObj;
          } else {
            // Primitive value
            return { [columns[0].key]: row };
          }
        });

        return { columns, rows };
      }, [interactionSpec.data, interactionSpec.headers]);

      return (
        <InteractiveTable
          interactionId={interactionSpec.interaction_id}
          title={interactionSpec.title}
          data={tableData}
          requiresResponse={interactionSpec.requires_response}
        />
      );

    case "show_chart":
      // Render interactive chart using ChartDisplay component for show_chart interaction
      return (
        <Suspense
          fallback={
            <div className="flex items-center justify-center h-64 text-muted-foreground">
              Loading chart...
            </div>
          }
        >
          <ChartDisplay
            interactionId={interactionSpec.interaction_id}
            title={interactionSpec.title}
            chartType={(interactionSpec.chart_type || "line") as ChartType}
            data={interactionSpec.data}
            options={interactionSpec.options}
            requiresResponse={interactionSpec.requires_response}
          />
        </Suspense>
      );

    case "markdown":
      // For now, just show the markdown content
      return (
        <div className="border rounded-lg p-4 bg-gray-50/50">
          <h3 className="font-medium text-sm mb-2">
            📝 {interactionSpec.title}
          </h3>
          <div className="prose prose-sm max-w-none">
            {interactionSpec.data.content}
          </div>
        </div>
      );

    case "excel_export":
      // Render Excel export result with download button
      return (
        <div className="border rounded-lg p-4 bg-green-50/50 border-green-200">
          <h3 className="font-medium text-sm mb-3 text-green-800 flex items-center gap-2">
            📊 Excel Export Complete
          </h3>
          <div className="space-y-2">
            <div className="text-sm text-gray-700">
              <strong>File:</strong> {interactionSpec.data.filename}
            </div>
            {interactionSpec.data.file_size && (
              <div className="text-sm text-gray-700">
                <strong>Size:</strong> {(interactionSpec.data.file_size / 1024).toFixed(1)} KB
              </div>
            )}
            {interactionSpec.data.sheets_count && (
              <div className="text-sm text-gray-700">
                <strong>Sheets:</strong> {interactionSpec.data.sheets_count}
              </div>
            )}
            <div className="mt-3">
              <a
                href={interactionSpec.data.download_url}
                download={interactionSpec.data.filename}
                className="inline-flex items-center gap-2 px-3 py-2 bg-green-600 hover:bg-green-700 text-white text-sm font-medium rounded-md transition-colors"
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                </svg>
                Download Excel File
              </a>
            </div>
          </div>
        </div>
      );

    default:
      return null;
  }
}

// Helper function to check if a tool output contains an interaction
export function hasInteraction(toolOutput: any): boolean {
  if (!toolOutput) {
    return false;
  }

  // If it's an array, check the first element
  if (Array.isArray(toolOutput) && toolOutput.length > 0) {
    return hasInteraction(toolOutput[0]);
  }

  try {
    // Handle array outputs (take first element)
    let actualOutput = toolOutput;
    if (Array.isArray(toolOutput) && toolOutput.length > 0) {
      actualOutput = toolOutput[0];
    }

    // If it's an object with a 'text' property, use that
    if (typeof actualOutput === "object" && actualOutput.text) {
      actualOutput = actualOutput.text;
    }

    if (typeof actualOutput === "object" && actualOutput.interaction_type) {
      return true;
    }

    // Check for MCP interaction parameter formats (raw data from show_table, show_chart, etc.)
    if (typeof actualOutput === "object") {
      // show_table format: {data: {columns: [...], rows: [...]}, title: "..."}
      if (actualOutput.data && 
          actualOutput.data.columns && 
          actualOutput.data.rows &&
          Array.isArray(actualOutput.data.columns) &&
          Array.isArray(actualOutput.data.rows)) {
        return true;
      }
      
      // show_chart format: {data: {categories: [...], series: [...]}, chart_type: "...", title: "..."}
      if (actualOutput.data && 
          actualOutput.chart_type &&
          (actualOutput.data.categories || actualOutput.data.series)) {
        return true;
      }

      // excel_export format: {status: "success", download_url: "...", filename: "...", ...}
      if (actualOutput.status === "success" && actualOutput.download_url) {
        console.log('hasInteraction: Found Excel export interaction:', actualOutput);
        return true;
      }
    }

    if (typeof actualOutput === "string") {
      // Check for Excel export JSON
      try {
        const jsonOutput = JSON.parse(actualOutput);
        if (jsonOutput.status === "success" && jsonOutput.download_url) {
          return true;
        }
      } catch (e) {
        // Not valid JSON, continue with other checks
      }
      
      // Check for interaction JSON in the output
      const hasInteractionType = actualOutput.includes('"interaction_type"');
      const hasInteractionId = actualOutput.includes('"interaction_id"');
      return hasInteractionType && hasInteractionId;
    }
  } catch (error) {
    console.error("hasInteraction: error", error);
    return false;
  }

  return false;
}
