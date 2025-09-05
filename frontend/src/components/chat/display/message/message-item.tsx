import * as React from "react";
import { Message } from "../types";
import {
  ChatCompleted,
  ChatCompletedProps,
} from "@/components/chat/display/message/item/chat-completed";
import { ChatProgress } from "@/components/chat/display/message/item/chat-progress";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

export interface MessageItemProps {
  message: Message;
  showTimeStamp?: boolean;
  actions?: React.ReactNode;
  animation?: "none" | "scale" | "slide" | "fade";
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
  actions,
  animation = "scale",
  className,
  activeTools = [],
  isLastMessage = false,
}: MessageItemProps) {
  // DEBUG: Log props (only for last message with tools)
  if (isLastMessage && activeTools.length > 0) {
    console.log("🔧 MessageItem props:", {
      messageId: message.id,
      isLastMessage,
      activeTools: activeTools?.length || 0,
    });
  }

  // Convert our Message type to ChatCompleted's expected format
  const chatMessage: ChatCompletedProps = {
    id: message.id,
    role: message.role,
    content: message.content,
    createdAt: message.createdAt ? new Date(message.createdAt) : undefined,
    showTimeStamp,
    animation,
    actions: (
      <div className="flex items-center gap-2">
        {message.processing_time_ms && (
          <Badge variant="secondary" className="text-xs">
            {(message.processing_time_ms / 1000).toFixed(1)}s
          </Badge>
        )}
        {actions}
      </div>
    ),
    // Convert our file attachments to the expected format if they exist
    experimental_attachments: message.file_attachments?.map((attachment) => ({
      name: attachment.original_name,
      contentType: attachment.mime_type || "application/octet-stream",
      url: attachment.file_path, // This might need adjustment based on how files are served
    })),
    // Convert tool usages to tool invocations if they exist
    toolInvocations: [
      // Completed tool usages
      ...(message.tool_usages?.map((usage) => ({
        state: "result" as const,
        id: usage.id,
        toolName: usage.tool_name,
        result: usage.output || {},
      })) || []),
      // Active/in-progress tools (show on the last assistant message during streaming)
      ...(message.role === "assistant" && 
          isLastMessage && 
          activeTools.length > 0
        ? activeTools.map((tool: any) => 
            tool.status === 'completed' 
              ? {
                  state: "result" as const,
                  id: tool.toolUsageId,
                  toolName: tool.tool,
                  result: { 
                    __completed: true,
                    executionTime: tool.executionTime 
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
    ],
  };

  // Check if there are any in-progress tool invocations
  const progressInvocations = chatMessage.toolInvocations?.filter(
    (invocation) =>
      invocation.state === "partial-call" ||
      invocation.state === "call" ||
      (invocation.state === "result" && invocation.result.__cancelled === true)
  );

  return (
    <div
      className={cn(
        "px-4 py-3 sm:px-4 [&_.tool-use>div>svg]:w-3 [&_.tool-use]:rounded-md [&_.tool-use]:text-[10px] [&_.tool-use]:px-1 [&_.tool-use]:py-0",
        className
      )}
    >
      {progressInvocations && progressInvocations.length > 0 && (
        <ChatProgress toolInvocations={progressInvocations} />
      )}
      <ChatCompleted {...chatMessage} />
    </div>
  );
}
