import * as React from "react";
import { Message } from "../types";
import { ChatMessage, ChatMessageProps } from "@/components/chat/display/message/item/chat-message";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

export interface MessageItemProps {
  message: Message;
  showTimeStamp?: boolean;
  actions?: React.ReactNode;
  animation?: "none" | "scale" | "slide" | "fade";
  className?: string;
}

export function MessageItem({
  message,
  showTimeStamp = false,
  actions,
  animation = "scale",
  className,
}: MessageItemProps) {
  // Convert our Message type to ChatMessage's expected format
  const chatMessage: ChatMessageProps = {
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
    toolInvocations: message.tool_usages?.map((usage) => ({
      state: "result" as const,
      id: usage.id,
      toolName: usage.tool_name,
      result: usage.output || {},
    })),
  };

  return (
    <div className={cn("px-4 py-3 sm:px-4 [&_.tool-use>div>svg]:w-3 [&_.tool-use]:rounded-md [&_.tool-use]:text-[10px] [&_.tool-use]:px-1 [&_.tool-use]:py-0", className)}>
      <ChatMessage {...chatMessage} />
    </div>
  );
}
