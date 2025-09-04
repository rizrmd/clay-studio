import { Button } from "@/components/ui/button";
import { Bot, Square } from "lucide-react";
import { ToolCallIndicator } from "../tool/tool-call-indicator";
import { TodoList } from "../interaction/todo-list";
import type { ToolUsage } from "@/lib/types/chat";
import type { Message } from "../types";
import type { ActiveToolInfo } from "../types";
import { useMemo } from "react";

interface LoadingIndicatorProps {
  activeTools: ActiveToolInfo[];
  canStop: boolean;
  onStop?: () => void;
  thinkingWord: string;
  streamingMessageTools?: ToolUsage[];
  messages?: Message[];
}

export function LoadingIndicator({
  activeTools,
  canStop,
  onStop,
  thinkingWord,
  streamingMessageTools,
  messages,
}: LoadingIndicatorProps) {
  // Find the latest TodoWrite data from messages
  const latestTodoWrite = useMemo(() => {
    if (!messages) return null;

    // Look for the last assistant message with TodoWrite data
    for (let i = messages.length - 1; i >= 0; i--) {
      if (messages[i].role === "assistant" && messages[i].todoWrite) {
        return messages[i].todoWrite;
      }
    }
    return null;
  }, [messages]);
  return (
    <div className="flex max-w-[45rem] mx-auto cursor-default">
      <div className="flex flex-1 relative p-2 rounded-lg">
        <div className="flex gap-3 justify-start flex-1 pr-[45px]">
          <div className="flex h-8 w-8 shrink-0 select-none items-center justify-center rounded-md bg-muted">
            <Bot className="h-4 w-4" />
          </div>
          <div className="flex flex-col gap-2 max-w-[70%] items-start">
            <div className="rounded-lg p-3 text-sm bg-muted">
              <div className="flex items-center space-x-2">
                <div className="flex items-center space-x-2 flex-1">
                  <div className="flex items-center space-x-1">
                    <div className="h-1 w-1 animate-bounce rounded-full bg-muted-foreground [animation-delay:-0.3s]"></div>
                    <div className="h-1 w-1 animate-bounce rounded-full bg-muted-foreground [animation-delay:-0.15s]"></div>
                    <div className="h-1 w-1 animate-bounce rounded-full bg-muted-foreground"></div>
                  </div>
                  {activeTools.length === 0 ? (
                    <span className="text-muted-foreground text-sm animate-pulse font-medium">
                      {thinkingWord}...
                    </span>
                  ) : (
                    <span className="text-muted-foreground text-sm animate-pulse font-medium flex items-center flex-1">
                      {thinkingWord} {activeTools.length}{" "}
                      {activeTools.length > 1 ? "tools" : "tool"}...
                    </span>
                  )}
                </div>

                {canStop && onStop && (
                  <div className="pl-6">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={onStop}
                      className="h-7 px-2"
                    >
                      <Square className="h-3 w-3 mr-1" />
                      Stop
                    </Button>
                  </div>
                )}
              </div>
            </div>
            {/* Show TodoList if there's TodoWrite data */}
            {latestTodoWrite && (latestTodoWrite as any).todos && (
              <div className="ml-3 mt-2">
                <TodoList todos={(latestTodoWrite as any).todos} />
              </div>
            )}

            {activeTools.length > 0 && (
              <ToolCallIndicator
                key={`active-tools-${activeTools
                  .map((t) => t.toolName)
                  .join("-")}`}
                tools={activeTools.map((t) => t.toolName)}
                variant="full"
                isCompleted={false}
                className="ml-3"
                toolUsages={activeTools.map((tool) => {
                  // Use activeTools as the source of truth since it's most up-to-date
                  const messageToolUsage = streamingMessageTools?.find(
                    (tu) => tu.tool_name === tool.toolName
                  );

                  return {
                    id: tool.tool_usage_id || messageToolUsage?.id || "",
                    message_id: messageToolUsage?.message_id || "",
                    tool_name: tool.toolName,
                    parameters: messageToolUsage?.parameters || null,
                    output:
                      tool.status === "completed"
                        ? messageToolUsage?.output || {
                            status: "completed",
                            result: "Tool execution completed",
                          }
                        : { status: "executing" },
                    execution_time_ms:
                      tool.execution_time_ms ||
                      messageToolUsage?.execution_time_ms,
                    createdAt: tool.started_at || messageToolUsage?.createdAt,
                  };
                })}
              />
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
