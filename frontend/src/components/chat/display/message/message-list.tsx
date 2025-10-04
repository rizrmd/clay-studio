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
          // processing_time_ms is null/undefined for incomplete messages, and a number (even 0) for complete ones
          // Note: Sometimes null values come through as objects from JSON parsing
          const hasValidProcessingTime =
            message.processing_time_ms !== null &&
            message.processing_time_ms !== undefined &&
            typeof message.processing_time_ms === 'number' &&
            !isNaN(message.processing_time_ms);

          // A message is incomplete if it has no processing_time_ms
          // The presence of processing_time_ms is the definitive indicator that streaming completed
          // Note: Some tools may have null output (filtered tools like TodoWrite) but that's OK
          const isStreamingComplete = streamingState && streamingState.isComplete;
          const isIncompleteMessage =
            message.role === "assistant" &&
            isLastMessage &&
            !hasValidProcessingTime &&  // Only check processing_time_ms, not tool outputs
            !isActivelyStreaming &&     // Not actively streaming (which would handle it differently)
            !isStreamingComplete;       // Not completed streaming either

          // Debug logging for last assistant message
          if (message.role === "assistant" && isLastMessage) {
            console.log("[MessageList] Last assistant message debug:", {
              messageId: message.id,
              hasValidProcessingTime,
              processing_time_ms: message.processing_time_ms,
              isActivelyStreaming,
              isStreamingComplete,
              isIncompleteMessage,
              hasProgressContent: !!message.progress_content,
              progressContentLength: message.progress_content?.length || 0,
              contentLength: message.content?.length || 0,
              toolUsagesCount: message.tool_usages?.length || 0,
              streamingStateExists: !!streamingState,
              streamingStateMessageId: streamingState?.messageId,
              streamingStateIsComplete: streamingState?.isComplete,
            });
          }
          
          
          
          
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

            // Use progress_content if available (contains internal thinking), otherwise use regular content
            const contentToShow = message.progress_content || message.content;

            // Only add content event if there's actual content to show
            // Note: We don't add content event for progress_content to avoid showing it twice
            // The InProgressMessage component will show "Thinking..." indicator instead
            if (contentToShow && contentToShow.trim() && message.content) {
              reconstructedEvents.push({
                type: "content" as const,
                timestamp: new Date(message.createdAt || Date.now()).getTime(),
                content: contentToShow,
              });
            }
            
            // Add tool events from tool_usages
            if (message.tool_usages) {
              message.tool_usages.forEach((usage) => {
                // Determine if the tool is complete or in-progress
                // A tool is complete if:
                // 1. It has output (not null/undefined) OR
                // 2. It's TodoWrite with parameters OR
                // 3. The message is completed AND the tool has execution_time_ms (filtered tools)
                const hasOutput = usage.output !== null && usage.output !== undefined;
                const isTodoWriteWithParams = usage.tool_name === "TodoWrite" && usage.parameters;
                const isFilteredButComplete = hasValidProcessingTime && 
                  usage.execution_time_ms !== null && usage.execution_time_ms !== undefined;
                
                const isComplete = hasOutput || isTodoWriteWithParams || isFilteredButComplete;
                
                // For TodoWrite, we need to use parameters (which contains the todos) not output
                let output = usage.output;
                if (usage.tool_name === "TodoWrite" && usage.parameters) {
                  output = usage.parameters;
                }
                
                // Always add the tool_start event first
                reconstructedEvents.push({
                  type: "tool_start" as const,
                  timestamp: new Date(usage.createdAt || Date.now()).getTime(),
                  tool: {
                    toolUsageId: usage.id,
                    toolName: usage.tool_name,
                    status: "active" as const,
                  },
                });

                // If complete, immediately add the tool_complete event
                if (isComplete) {
                  reconstructedEvents.push({
                    type: "tool_complete" as const,
                    timestamp: new Date(usage.createdAt || Date.now()).getTime() + 1, // Slightly later timestamp
                    tool: {
                      toolUsageId: usage.id,
                      toolName: usage.tool_name,
                      status: "completed" as const,
                      executionTime: usage.execution_time_ms,
                      output: output,
                    },
                  });
                }
              });
            }


            return (
              <InProgressMessage
                key={message.id || index}
                message={message as Message}
                events={reconstructedEvents}
                isIncomplete={true}
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
              showTimeStamp={true}
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
