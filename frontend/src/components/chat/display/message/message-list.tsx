import { useChat } from "@/lib/hooks/use-chat";
import type { Message } from "@/lib/types/chat";
import { useEffect, useRef } from "react";
import { MessageItem } from "./message-item";
import { InProgressMessage } from "./in-progress-message";
import { TypingIndicator } from "@/components/ui/typing-indicator";

export const MessageList = () => {
  const { currentMessages, isStreaming, conversationId, currentActiveTools, isLoadingMessages, streamingState } = useChat();
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const endOfMessagesRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new messages arrive or when streaming state changes
  useEffect(() => {
    endOfMessagesRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [currentMessages]);

  useEffect(() => {
    endOfMessagesRef.current?.scrollIntoView({ behavior: "instant" });
  }, [conversationId]);

  if (isLoadingMessages) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center text-muted-foreground">
          <TypingIndicator />
        </div>
      </div>
    );
  }

  if (!currentMessages || currentMessages.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center text-muted-foreground">
          <p>No messages yet. Start a conversation!</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-auto flex relative" ref={scrollAreaRef}>
      <div className="flex flex-col absolute inset-0 mx-auto max-w-2xl">
        {currentMessages.map((message, index) => {
          const isLastMessage = index === currentMessages.length - 1;
          
          // Check if this is an in-progress message (during active streaming)
          // Must have events to be considered actively streaming (not just a stale state after refresh)
          const isActivelyStreaming = 
            message.role === "assistant" && 
            isLastMessage && 
            streamingState && 
            streamingState.messageId === message.id &&
            !streamingState.isComplete &&
            streamingState.events && 
            streamingState.events.length > 0;
          
          // Check if message appears incomplete (after refresh, no streaming state)
          // An incomplete message has no processing_time_ms and is the last assistant message
          // processing_time_ms is null for incomplete messages, and a number (even 0) for complete ones
          const isIncompleteMessage = 
            message.role === "assistant" && 
            isLastMessage &&
            (message.processing_time_ms === null || message.processing_time_ms === undefined) &&
            !isActivelyStreaming; // Not actively streaming (which would handle it differently)
          
          
          // Use InProgressMessage for actively streaming messages
          if (isActivelyStreaming) {
            return (
              <InProgressMessage
                key={message.id || index}
                message={message as Message}
                events={[...(streamingState.events || [])]}
              />
            );
          }
          
          // For incomplete messages after refresh, reconstruct events from tool_usages
          if (isIncompleteMessage) {
            // Reconstruct events from existing tool usages
            const reconstructedEvents = [];
            
            // Add content event if message has content
            if (message.content) {
              reconstructedEvents.push({
                type: "content" as const,
                timestamp: new Date(message.createdAt || Date.now()).getTime(),
                content: message.content,
              });
            }
            
            // Add tool events from tool_usages
            if (message.tool_usages) {
              message.tool_usages.forEach((usage) => {
                // For TodoWrite, we need to use parameters (which contains the todos) not output
                let output = usage.output;
                if (usage.tool_name === "TodoWrite" && usage.parameters) {
                  output = usage.parameters;
                }
                
                reconstructedEvents.push({
                  type: "tool_complete" as const,
                  timestamp: new Date(usage.createdAt || Date.now()).getTime(),
                  tool: {
                    toolUsageId: usage.id,
                    toolName: usage.tool_name,
                    status: "completed" as const,
                    executionTime: usage.execution_time_ms,
                    output: output,
                  },
                });
              });
            }
            
            return (
              <InProgressMessage
                key={message.id || index}
                message={message as Message}
                events={reconstructedEvents}
              />
            );
          }
          
          // Skip truly empty messages (no content, no tools, and complete)
          if (message.content === "" && 
              (!message.tool_usages || message.tool_usages.length === 0) &&
              message.processing_time_ms !== null && 
              message.processing_time_ms !== undefined) { // Only skip if it's marked as complete
            return null;
          }
      
          return (
            <MessageItem
              key={message.id || index}
              message={message as Message}
              activeTools={currentActiveTools}
              isLastMessage={isLastMessage}
            />
          );
        })}
        {isStreaming && (
          <div className="px-4 sm:px-4">
            <TypingIndicator />
          </div>
        )}
        <div ref={endOfMessagesRef} className="end-of-chat min-h-[90px] w-full" />
      </div>
    </div>
  );
};
