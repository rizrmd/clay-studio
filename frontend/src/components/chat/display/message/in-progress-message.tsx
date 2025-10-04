import { cn } from "@/lib/utils";
import { MarkdownRenderer } from "@/components/ui/markdown-renderer";
import { Message } from "@/lib/types/chat";
import { Loader2, CheckCircle2, XCircle } from "lucide-react";
import { useEffect, useState } from "react";
import { getFriendlyToolName } from "@/lib/utils/tool-names";
import { TodoList } from "@/components/chat/display/message/item/interaction/todo-list";
import { Timestamp } from "@/components/ui/timestamp";

// Define the StreamingEvent interface here since it's not exported from chat-store
export interface StreamingEvent {
  type: "content" | "tool_start" | "tool_complete";
  timestamp: number;
  content?: string;
  tool?: {
    toolUsageId: string;
    toolName: string;
    status: "active" | "completed" | "error";
    executionTime?: number;
    output?: any;
  };
}

export interface InProgressMessageProps {
  message: Message;
  events: StreamingEvent[];
  className?: string;
  isIncomplete?: boolean; // Whether this message is incomplete (for showing thinking indicator)
}

function ToolStatusIcon({ status }: { status: string }) {
  switch (status) {
    case "active":
      return <Loader2 className="h-3 w-3 animate-spin text-blue-500" />;
    case "completed":
      return <CheckCircle2 className="h-3 w-3 text-green-500" />;
    case "error":
      return <XCircle className="h-3 w-3 text-red-500" />;
    default:
      return null;
  }
}

export function InProgressMessage({
  message,
  events,
  className,
  isIncomplete = false,
}: InProgressMessageProps) {
  // Update timeago strings every second for live updates
  const [, setUpdateTrigger] = useState(0);
  
  useEffect(() => {
    const interval = setInterval(() => {
      setUpdateTrigger(prev => prev + 1);
    }, 1000); // Update every second
    
    return () => clearInterval(interval);
  }, []);
  // Track tool states and build timeline
  const toolStates = new Map<string, StreamingEvent["tool"]>();
  const processedEvents: Array<{ 
    type: "content" | "tool" | "todos"; 
    content?: string; 
    toolUsageId?: string;
    tool?: StreamingEvent["tool"];
    timestamp?: number;
    todos?: Array<{ content: string; status: "pending" | "in_progress" | "completed" }>;
  }> = [];
  
  let contentBuffer = "";
  let latestTodos: Array<{ content: string; status: "pending" | "in_progress" | "completed" }> = [];

  let firstEventTimestamp: number | null = null;
  
  events.forEach((event, index) => {
    if (!firstEventTimestamp && event.timestamp) {
      firstEventTimestamp = event.timestamp;
    }
    
    if (event.type === "content") {
      contentBuffer += event.content || "";
      // If next event is not content or this is the last event, flush buffer
      const nextEvent = events[index + 1];
      if (!nextEvent || nextEvent.type !== "content") {
        if (contentBuffer) {
          processedEvents.push({ 
            type: "content", 
            content: contentBuffer,
            timestamp: event.timestamp 
          });
          contentBuffer = "";
        }
      }
    } else if (event.type === "tool_start" && event.tool) {
      // Flush any buffered content first
      if (contentBuffer) {
        processedEvents.push({ 
          type: "content", 
          content: contentBuffer,
          timestamp: event.timestamp 
        });
        contentBuffer = "";
      }
      
      // Track this tool and add to timeline (create a copy to avoid read-only issues)
      const toolCopy = { ...event.tool };
      toolStates.set(event.tool.toolUsageId, toolCopy);
      
      // Only add to timeline if it's not TodoWrite (TodoWrite will be shown as todos when complete)
      if (event.tool.toolName !== "TodoWrite") {
        processedEvents.push({ 
          type: "tool", 
          toolUsageId: event.tool.toolUsageId,
          tool: toolCopy,
          timestamp: event.timestamp
        });
      }
    } else if (event.type === "tool_complete" && event.tool) {
      // Update the existing tool state
      const existingTool = toolStates.get(event.tool.toolUsageId);
      if (existingTool) {
        // Create updated tool object instead of mutating
        const updatedTool = {
          ...existingTool,
          status: event.tool.status,
          executionTime: event.tool.executionTime
        };

        // Update in tracking map
        toolStates.set(event.tool.toolUsageId, updatedTool);

        // Find and update the tool in processedEvents (only if it exists)
        const toolEventIndex = processedEvents.findIndex(
          e => e.type === "tool" && e.toolUsageId === event.tool?.toolUsageId
        );
        if (toolEventIndex !== -1) {
          // Replace the entire event object to ensure proper re-render
          processedEvents[toolEventIndex] = {
            ...processedEvents[toolEventIndex],
            tool: updatedTool,
            timestamp: processedEvents[toolEventIndex].timestamp || event.timestamp
          };
        }
        
        // If this is TodoWrite tool, extract and add todos
        if (event.tool.toolName === "TodoWrite" && event.tool.status === "completed") {
          // Flush any buffered content first
          if (contentBuffer) {
            processedEvents.push({ 
              type: "content", 
              content: contentBuffer,
              timestamp: event.timestamp 
            });
            contentBuffer = "";
          }
          
          // Try to extract todos from the tool output
          // For TodoWrite, the todos might be in output (which could be parameters for reconstructed events)
          let todos: Array<{ content: string; status: "pending" | "in_progress" | "completed" }> | null = null;
          
          
          try {
            // Check both output and parameters fields since TodoWrite stores todos in parameters
            const dataSource = event.tool.output || (event.tool as any).parameters;
            
            if (dataSource) {
              // The data might be the todos directly or wrapped in an object
              if (typeof dataSource === "object") {
                if (Array.isArray(dataSource)) {
                  // Data is directly the todos array
                  todos = dataSource;
                } else if (dataSource.todos) {
                  // Data has a todos property (this is the expected format)
                  todos = dataSource.todos;
                } else if (dataSource.parameters?.todos) {
                  // Data contains parameters with todos (from backend)
                  todos = dataSource.parameters.todos;
                }
              } else if (typeof dataSource === "string") {
                // Try to parse JSON string
                try {
                  const parsed = JSON.parse(dataSource);
                  if (Array.isArray(parsed)) {
                    todos = parsed;
                  } else if (parsed.todos) {
                    todos = parsed.todos;
                  } else if (parsed.parameters?.todos) {
                    todos = parsed.parameters.todos;
                  }
                } catch {
                  // Not valid JSON, ignore
                }
              }
            }
          } catch (e) {
            console.error("Failed to parse TodoWrite output:", e, event.tool.output);
          }
          
          if (todos && Array.isArray(todos) && todos.length > 0) {
            // Save the latest todos - will be rendered at the end
            latestTodos = todos as Array<{ content: string; status: "pending" | "in_progress" | "completed" }>;
            
            // Remove the TodoWrite tool indicator from processedEvents
            const toolEventIndex = processedEvents.findIndex(
              e => e.type === "tool" && e.toolUsageId === event.tool?.toolUsageId
            );
            if (toolEventIndex !== -1) {
              // Remove the tool indicator completely
              processedEvents.splice(toolEventIndex, 1);
            }
          }
        }
      }
    }
  });

  // Flush any remaining content
  if (contentBuffer) {
    processedEvents.push({ type: "content", content: contentBuffer });
  }
  
  // Add latest todos at the end if we have them
  if (latestTodos.length > 0) {
    processedEvents.push({
      type: "todos",
      todos: latestTodos
    });
  }


  return (
    <div
      className={cn(
        "px-4 py-3 sm:px-4",
        className
      )}
    >
      <div className="flex flex-col gap-2">
        {/* Show thinking indicator if message is incomplete (no processing_time_ms) */}
        {isIncomplete && (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            <span>Thinking...</span>
          </div>
        )}

        {processedEvents.map((event, index) => {
          const timestamp = event.timestamp ? new Date(event.timestamp) : null;
            
          if (event.type === "content") {
            return (
              <div key={index} className="max-w-full overflow-auto">
                <MarkdownRenderer>{event.content || ""}</MarkdownRenderer>
              </div>
            );
          } else if (event.type === "tool" && event.tool) {
            const friendlyName = getFriendlyToolName(event.tool.toolName);
            
            return (
              <div key={index} className="group">
                <div
                  className={cn(
                    "inline-flex items-center gap-2 px-3 py-1.5 rounded-md text-xs self-start",
                    "bg-slate-50 border border-slate-200",
                    "max-w-fit relative"
                  )}
                >
                  <ToolStatusIcon status={event.tool.status} />
                  <span className="font-medium">{friendlyName}</span>
                  {event.tool.status === "completed" && event.tool.executionTime && (
                    <span className="text-muted-foreground">
                      {(event.tool.executionTime / 1000).toFixed(2)}s
                    </span>
                  )}
                  {timestamp && (
                    <Timestamp
                      date={timestamp}
                      format="custom"
                      showTime={true}
                      showSeconds={true}
                      className="absolute -right-1 translate-x-full top-1/2 -translate-y-1/2 text-[10px] text-muted-foreground opacity-0 group-hover:opacity-60 transition-opacity whitespace-nowrap pl-2"
                      showTooltip={false}
                    />
                  )}
                </div>
              </div>
            );
          } else if (event.type === "todos" && event.todos) {
            return (
              <TodoList 
                key={index} 
                todos={event.todos} 
                className="mt-0"
              />
            );
          }
          return null;
        })}
        
        {/* Show message timestamp */}
        <Timestamp
          date={message.createdAt || new Date()}
          format="relative"
          className="mt-1 block opacity-50"
        />
      </div>
    </div>
  );
}