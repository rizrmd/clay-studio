import { Message } from "../types";
import { cn } from "@/lib/utils";
import { MarkdownRenderer } from "@/components/ui/markdown-renderer";
import { ChatToolUsage } from "@/components/chat/display/message/item/tools/chat-tool-usage";
import { ChatProgress } from "@/components/chat/display/message/item/chat-progress";
import {
  chatBubbleVariants,
  type Animation,
} from "@/components/chat/display/message/item/chat-bubble-variants";
import { type ToolInvocation } from "@/components/chat/display/message/item/types";
import {
  InteractionRenderer,
  hasInteraction,
} from "@/components/chat/display/message/item/interaction/interaction-renderer";
import { TodoList } from "@/components/chat/display/message/item/interaction/todo-list";
import { parseMcpToolResult } from "@/components/chat/display/tool/tool-call-utils";
import { FileAttachmentDisplay } from "./file-attachment-display";

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
  // Convert file attachments to the expected format with proper download URLs
  const experimental_attachments = message.file_attachments?.map(
    (attachment) => {
      // Extract client_id and project_id from file_path
      // file_path format: .clients/{client_id}/{project_id}/uploads/{file_name}
      const pathParts = attachment.file_path.split('/');
      const clientId = pathParts[1]; // clients/{client_id}
      const projectId = pathParts[2]; // {project_id}
      const fileName = pathParts[pathParts.length - 1]; // {file_name}
      
      const downloadUrl = `/api/uploads/${clientId}/${projectId}/${fileName}`;
      
      return {
        id: attachment.id,
        name: attachment.original_name,
        contentType: attachment.mime_type || "application/octet-stream",
        url: downloadUrl,
        size: attachment.file_size,
        description: attachment.description,
        autoDescription: attachment.auto_description,
        isTextFile: attachment.is_text_file,
        preview: attachment.preview,
      };
    }
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

  // Don't convert to File objects - we'll display attachment info directly
  const fileAttachments = experimental_attachments;

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
      {fileAttachments && fileAttachments.length > 0 ? (
        <div className="mb-2 flex flex-wrap gap-2">
          {fileAttachments.map((attachment, index) => (
            <FileAttachmentDisplay 
              key={attachment.id || index} 
              attachment={attachment}
            />
          ))}
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
              if (invocation.toolName === 'mcp__interaction__export_excel') {
                // First try originalOutput, then fall back to result
                const sourceData = (invocation as any).originalOutput || invocation.result;
                
                try {
                  // Handle array format with MCP tool result (same as tool-usage-dialog.tsx)
                  if (Array.isArray(sourceData) && sourceData.length > 0) {
                    const firstItem = sourceData[0];
                    if (
                      firstItem &&
                      typeof firstItem === "object" &&
                      firstItem.text &&
                      firstItem.type === "text"
                    ) {
                      const parsedMcp = parseMcpToolResult(firstItem.text);
                      if (parsedMcp) {
                        dataToCheck = parsedMcp;
                      }
                    }
                  }
                  // Handle direct JSON string
                  else if (typeof sourceData === 'string') {
                    dataToCheck = JSON.parse(sourceData);
                  }
                  // Handle direct object
                  else if (typeof sourceData === 'object') {
                    dataToCheck = sourceData;
                  }
                } catch (e) {
                  console.error('Failed to parse export_excel data:', e, 'sourceData:', sourceData);
                  dataToCheck = sourceData;
                }
              }
              
              if (hasInteraction(dataToCheck)) {
                return (
                  <InteractionRenderer
                    key={`interaction-${invocation.id}`}
                    toolOutput={dataToCheck}
                  />
                );
              } else if (isMcpInteraction) {
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
                return (
                  <TodoList
                    key={`todo-${invocation.id}`}
                    todos={todos}
                  />
                );
              } else {
              }
            }
            return null;
          })}
          <ChatProgress toolInvocations={toolInvocations} />
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
