import { useCallback, useEffect, useState } from "react";
import { useSnapshot } from "valtio";
import { chatStore } from "../store/chat/chat-store";
import { wsService } from "../services/ws-service";
import type { Message } from "../types/chat";

export interface RetryState {
  isIdle: boolean;
  lastUserMessage: Message | null;
  canRetry: boolean;
  isRetrying: boolean;
}

export const useRetryConversation = () => {
  const snap = useSnapshot(chatStore);
  const [retryState, setRetryState] = useState<RetryState>({
    isIdle: false,
    lastUserMessage: null,
    canRetry: false,
    isRetrying: false,
  });

  // Check if conversation is idle (last message is from user and not streaming)
  useEffect(() => {
    if (!snap.conversation_id || snap.conversation_id === "new") {
      setRetryState({
        isIdle: false,
        lastUserMessage: null,
        canRetry: false,
        isRetrying: false,
      });
      return;
    }

    const conversation = snap.map[snap.conversation_id];
    if (!conversation || !conversation.messages || conversation.messages.length === 0) {
      setRetryState({
        isIdle: false,
        lastUserMessage: null,
        canRetry: false,
        isRetrying: false,
      });
      return;
    }

    const lastMessage = conversation.messages[conversation.messages.length - 1];
    const isStreaming = wsService.isStreaming(snap.conversation_id);
    const isConnected = wsService.isConnected();

    // Check if conversation is idle:
    // 1. Last message is from user
    // 2. Not currently streaming
    // 3. WebSocket is connected
    const isIdle = lastMessage.role === "user" && !isStreaming && isConnected;
    
    setRetryState((prev) => ({
      ...prev,
      isIdle,
      lastUserMessage: isIdle ? {
        ...lastMessage,
        file_attachments: lastMessage.file_attachments ? [...lastMessage.file_attachments] : undefined,
        tool_usages: lastMessage.tool_usages ? [...lastMessage.tool_usages] : undefined
      } as Message : null,
      canRetry: isIdle && !prev.isRetrying,
    }));
  }, [snap.conversation_id, snap.map, snap.streaming]);

  // Retry sending the last message using the new backend endpoint
  const retry = useCallback(() => {
    if (!retryState.canRetry || !retryState.lastUserMessage) return;

    setRetryState((prev) => ({ ...prev, isRetrying: true }));

    // Send retry request via WebSocket using the new message type
    // Send retry request via WebSocket
    wsService.retryLastMessage(snap.project_id, snap.conversation_id);

    // Reset retry state after a short delay
    setTimeout(() => {
      setRetryState((prev) => ({ ...prev, isRetrying: false }));
    }, 1000);
  }, [retryState.canRetry, retryState.lastUserMessage, snap.project_id, snap.conversation_id]);

  // Alternative retry method: Resend the last user message without adding it to the UI
  const resendLastMessage = useCallback(() => {
    if (!retryState.lastUserMessage || !snap.project_id || !snap.conversation_id) return;

    setRetryState((prev) => ({ ...prev, isRetrying: true }));

    // Send the message directly without adding to the UI
    // Extract file IDs from file_attachments if they exist
    const fileIds = retryState.lastUserMessage.file_attachments?.map(f => f.id);
    wsService.sendChatMessage(
      snap.project_id,
      snap.conversation_id,
      retryState.lastUserMessage.content,
      fileIds
    );

    // Reset retry state after a short delay
    setTimeout(() => {
      setRetryState((prev) => ({ ...prev, isRetrying: false }));
    }, 1000);
  }, [retryState.lastUserMessage, snap.project_id, snap.conversation_id]);

  return {
    ...retryState,
    retry,
    resendLastMessage,
  };
};