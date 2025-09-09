import { useMemo } from "react";
import { Message } from "../types";
import { cn } from "@/lib/utils";
import { FilePreview } from "@/components/ui/file-preview";
import { MarkdownRenderer } from "@/components/ui/markdown-renderer";
import { ChatToolUsage } from "@/components/chat/display/message/item/tools/chat-tool-usage";
import {
  chatBubbleVariants,
  type Animation,
} from "@/components/chat/display/message/item/chat-bubble-variants";
import { type ToolInvocation } from "@/components/chat/display/message/item/types";
import { dataUrlToUint8Array } from "@/components/chat/display/message/item/utils";
import {
  InteractionRenderer,
  hasInteraction,
} from "@/components/chat/display/message/item/interaction/interaction-renderer";
import { TodoList } from "@/components/chat/display/message/item/interaction/todo-list";

export interface MessageItemProps {
  message: Message;
  showTimeStamp?: boolean;
  animation?: Animation;
  className?: string;
  activeTools?: readonly {
    tool: string;
    toolUsageId: string;
    startTime: number;
  }[];
  isLastMessage?: boolean;
}

export function MessageItem({
  message,
  showTimeStamp = false,
  animation = "scale",
  className,
  activeTools = [],
  isLastMessage = false,
}: MessageItemProps) {
  // Convert file attachments to the expected format
  const experimental_attachments = message.file_attachments?.map(
    (attachment) => ({
      name: attachment.original_name,
      contentType: attachment.mime_type || "application/octet-stream",
      url: attachment.file_path,
    })
  );

  // Convert tool usages to tool invocations
  const completedToolUsageIds = new Set(message.tool_usages?.map(usage => usage.id) || []);
  
  const toolInvocations: ToolInvocation[] = [
    // Completed tool usages
    ...(message.tool_usages?.map((usage) => {
      // For MCP interaction tools, use parameters as the result for interaction rendering
      const isMcpInteraction = usage.tool_name.startsWith('mcp__interaction__');
      
      return {
        state: "result" as const,
        id: usage.id,
        toolName: usage.tool_name,
        result: isMcpInteraction ? (usage.parameters || {}) : (usage.output || {}),
        // Keep original data accessible
        originalOutput: usage.output,
        originalParameters: usage.parameters,
      };
    }) || []),
    // Active/in-progress tools (show on the last assistant message during streaming)
    // Only include active tools that aren't already in completed tool usages
    ...(message.role === "assistant" && isLastMessage && activeTools.length > 0
      ? activeTools
          .filter((tool: any) => !completedToolUsageIds.has(tool.toolUsageId))
          .map((tool: any) =>
            tool.status === "completed"
              ? {
                  state: "result" as const,
                  id: tool.toolUsageId,
                  toolName: tool.tool,
                  result: {
                    __completed: true,
                    executionTime: tool.executionTime,
                  },
                }
              : {
                  state: "call" as const,
                  id: tool.toolUsageId,
                  toolName: tool.tool,
                  args: tool.args || {},
                }
          )
      : []),
  ];

  const files = useMemo(() => {
    return experimental_attachments?.map((attachment) => {
      const dataArray = dataUrlToUint8Array(attachment.url);
      const file = new File([dataArray], attachment.name ?? "Unknown", {
        type: attachment.contentType,
      });
      return file;
    });
  }, [experimental_attachments]);

  const isUser = message.role === "user";
  const createdAt = message.createdAt ? new Date(message.createdAt) : undefined;
  const formattedTime = createdAt?.toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
  });

  const completedToolInvocations = toolInvocations?.filter(
    (invocation) =>
      invocation.state === "result" && !invocation.result.__cancelled && invocation.toolName !== 'TodoWrite'
  );

  const todoWriteInvocations = toolInvocations?.filter(
    (invocation) =>
      invocation.state === "result" && !invocation.result.__cancelled && invocation.toolName === 'TodoWrite'
  );

  const renderMessage = (className?: string) => (
    <div className={cn(" max-w-full overflow-auto flex flex-col", isUser ? "items-end" : "items-start")}>
      {files ? (
        <div className="mb-1 flex flex-wrap gap-2">
          {files.map((file, index) => {
            return <FilePreview file={file} key={index} />;
          })}
        </div>
      ) : null}

      <div className={className}>
        <MarkdownRenderer>{message.content}</MarkdownRenderer>
      </div>

      {showTimeStamp && createdAt ? (
        <time
          dateTime={createdAt.toISOString()}
          className={cn(
            "mt-1 block px-1 text-xs opacity-50",
            animation !== "none" && "duration-500 animate-in fade-in-0"
          )}
        >
          {formattedTime}
        </time>
      ) : null}
    </div>
  );

  return (
    <div
      className={cn(
        "px-4 py-3 sm:px-4 [&_.tool-use>div>svg]:w-3 [&_.tool-use]:rounded-md [&_.tool-use]:text-[10px] [&_.tool-use]:px-1 [&_.tool-use]:py-0",
        className
      )}
    >
      {isUser ? (
        renderMessage(cn(chatBubbleVariants({ isUser, animation })))
      ) : (
        <div
          className={cn(
            chatBubbleVariants({ isUser, animation }),
            "flex flex-col items-start gap-2 flex-wrap "
          )}
        >
          {renderMessage()}
          {/* Render interactions from tool outputs */}
          {completedToolInvocations?.map((invocation) => {
            if (invocation.state === "result") {
              // For MCP interaction tools, the raw parameters contain the interaction data
              // For other tools, check result (backward compatibility)  
              const isMcpInteraction = invocation.toolName?.startsWith('mcp__interaction__');
              const interactionData = isMcpInteraction 
                ? invocation.result // This now contains the original parameters
                : invocation.result;
              
              // For export_excel specifically, also check the original output
              let dataToCheck = interactionData;
              if (invocation.toolName === 'mcp__interaction__export_excel' && 
                  (invocation as any).originalOutput) {
                try {
                  // Parse the original output if it's a JSON string
                  const originalOutput = (invocation as any).originalOutput;
                  dataToCheck = typeof originalOutput === 'string' 
                    ? JSON.parse(originalOutput) 
                    : originalOutput;
                } catch (e) {
                  console.error('Failed to parse originalOutput for export_excel:', e);
                  dataToCheck = (invocation as any).originalOutput;
                }
              }
              
              if (hasInteraction(dataToCheck)) {
                console.log('Rendering interaction for tool:', invocation.toolName, 'with data:', dataToCheck);
                return (
                  <InteractionRenderer
                    key={`interaction-${invocation.id}`}
                    toolOutput={dataToCheck}
                  />
                );
              } else if (isMcpInteraction) {
                console.log('MCP interaction tool but no interaction detected:', invocation.toolName, 'data:', dataToCheck);
              }
            }
            return null;
          })}
          {/* Render TodoWrite invocations as proper todo lists - show only the most recent */}
          {todoWriteInvocations?.slice(-1).map((invocation) => {
            if (invocation.state === "result") {
              // Check for todos data in result
              const todos = invocation.result?.todos;
              
              if (todos && Array.isArray(todos) && todos.length > 0) {
                console.log(`Rendering TodoList with todos:`, todos);
                return (
                  <TodoList
                    key={`todo-${invocation.id}`}
                    todos={todos}
                  />
                );
              } else {
                console.log(`No todos found for invocation ${invocation.id}:`, {
                  resultTodos: invocation.result?.todos
                });
              }
            }
            return null;
          })}
          <ChatToolUsage
            toolInvocations={completedToolInvocations}
            className={
              "bg-white border border-transparent hover:border-slate-200 transition-all"
            }
          />
        </div>
      )}
    </div>
  );
}
