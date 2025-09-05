import { useMemo, Suspense } from "react";
import { AskUser } from "./ask-user";
// import { WebSocketService } from "@/lib/services/websocket-service";
import { InteractiveTable } from "@/components/data-table/interactive-table";
import { ChartDisplay } from "@/components/data-chart";
import type { ChartType } from "@/components/data-chart/chart-types";

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
  interaction_type: "buttons" | "checkbox" | "input" | "chart" | "table" | "show_chart" | "show_table" | "markdown";
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

      // If it's an object with a 'text' property, use that
      if (typeof actualOutput === "object" && actualOutput.text) {
        actualOutput = actualOutput.text;
      }

      // If it's a string, try to extract JSON from it
      if (typeof actualOutput === "string") {
        // Look for JSON block in the output
        const jsonMatch = actualOutput.match(/```json\n([\s\S]*?)\n```/);
        if (jsonMatch) {
          const parsed = JSON.parse(jsonMatch[1]);
          if (parsed.interaction_type) {
            return parsed as InteractionSpec;
          }
        }

        // Try parsing the whole string as JSON
        const parsed = JSON.parse(actualOutput);
        if (parsed.interaction_type) {
          return parsed as InteractionSpec;
        }
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
            wsService.sendAskUserResponse(interactionSpec.interaction_id, response);

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
        <Suspense fallback={<div className="flex items-center justify-center h-64 text-muted-foreground">Loading chart...</div>}>
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
          }));
        } else {
          // Infer columns from first data row
          const firstRow = data[0];
          if (typeof firstRow === 'object' && firstRow !== null) {
            columns = Object.keys(firstRow).map(key => ({
              key,
              label: key,
              data_type: "string" as const,
            }));
          } else {
            // Data is array of primitives, create generic columns
            columns = Array.isArray(firstRow) 
              ? firstRow.map((_, index) => ({
                  key: `col_${index}`,
                  label: `Column ${index + 1}`,
                  data_type: "string" as const,
                }))
              : [{ key: 'value', label: 'Value', data_type: "string" as const }];
          }
        }

        // Transform data rows
        const rows = data.map((row) => {
          if (typeof row === 'object' && row !== null && !Array.isArray(row)) {
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
        <Suspense fallback={<div className="flex items-center justify-center h-64 text-muted-foreground">Loading chart...</div>}>
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
          <h3 className="font-medium text-sm mb-2">📝 {interactionSpec.title}</h3>
          <div className="prose prose-sm max-w-none">
            {interactionSpec.data.content}
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

    if (typeof actualOutput === "string") {
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
