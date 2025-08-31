import { useMemo } from "react";
import { AskUser } from "./ask-user";
import { WebSocketService } from "@/services/chat/websocket-service";
import { InteractiveTable } from "@/components/data-table/interactive-table";

interface InteractionSpec {
  interaction_id: string;
  interaction_type: "buttons" | "checkbox" | "input" | "chart" | "table" | "markdown";
  title: string;
  data: any;
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
    console.log("InteractionRenderer: parsing toolOutput", toolOutput);
    if (!toolOutput) return null;
    
    // Handle array outputs (take first element)
    let actualOutput = toolOutput;
    if (Array.isArray(toolOutput) && toolOutput.length > 0) {
      actualOutput = toolOutput[0];
      console.log("InteractionRenderer: extracted from array", actualOutput);
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
          console.log("InteractionRenderer: found JSON block", jsonMatch[1]);
          const parsed = JSON.parse(jsonMatch[1]);
          if (parsed.interaction_type) {
            console.log("InteractionRenderer: successfully parsed interaction spec", parsed);
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
      // For now, just show a placeholder for charts
      return (
        <div className="border rounded-lg p-4 bg-blue-50/50">
          <h3 className="font-medium text-sm mb-2">📊 {interactionSpec.title}</h3>
          <div className="text-xs text-muted-foreground">
            Chart visualization will be implemented here
          </div>
          <pre className="mt-2 text-xs bg-white p-2 rounded">
            {JSON.stringify(interactionSpec.data, null, 2)}
          </pre>
        </div>
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
    console.log("hasInteraction: toolOutput is null/undefined");
    return false;
  }
  
  console.log("hasInteraction: checking toolOutput", typeof toolOutput, toolOutput);
  
  // If it's an array, check the first element
  if (Array.isArray(toolOutput) && toolOutput.length > 0) {
    console.log("hasInteraction: toolOutput is array, checking first element", toolOutput[0]);
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
      console.log("hasInteraction: found object with interaction_type");
      return true;
    }
    
    if (typeof actualOutput === "string") {
      // Check for interaction JSON in the output
      const hasInteractionType = actualOutput.includes('"interaction_type"');
      const hasInteractionId = actualOutput.includes('"interaction_id"');
      console.log("hasInteraction: string check", { hasInteractionType, hasInteractionId });
      return hasInteractionType && hasInteractionId;
    }
  } catch (error) {
    console.error("hasInteraction: error", error);
    return false;
  }
  
  console.log("hasInteraction: no interaction found");
  return false;
}