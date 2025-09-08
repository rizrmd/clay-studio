import { useChat } from "@/lib/hooks/use-chat";
import type { Message } from "@/lib/types/chat";
import { useEffect, useRef } from "react";
import { MessageItem } from "./message-item";
import { TypingIndicator } from "@/components/ui/typing-indicator";

export const MessageList = () => {
  const { currentMessages, isStreaming, conversationId, currentActiveTools } = useChat();
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const endOfMessagesRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new messages arrive or when streaming state changes
  useEffect(() => {
    endOfMessagesRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [currentMessages]);

  useEffect(() => {
    endOfMessagesRef.current?.scrollIntoView({ behavior: "instant" });
  }, [conversationId]);

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
          if (message.content === "") return null;
          const isLastMessage = index === currentMessages.length - 1;
      
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
