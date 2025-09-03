import { useCallback, useEffect, useRef } from "react";
import {
  getConversationState,
  addToMessageQueue,
  removeFromMessageQueue,
  updateMessageInQueue,
  QueuedMessage,
} from "@/store/chat-store";

interface QueueManagerProps {
  currentConversationId: string;
  isStreaming: boolean;
  messageQueue: QueuedMessage[];
  sendMessage: (content: string, files?: File[], isFromQueue?: boolean) => Promise<void>;
}

export function useQueueManager({
  currentConversationId,
  isStreaming,
  messageQueue,
  sendMessage,
}: QueueManagerProps) {
  // Store sendMessage in a ref to avoid dependency issues
  const sendMessageRef = useRef(sendMessage);
  useEffect(() => {
    sendMessageRef.current = sendMessage;
  }, [sendMessage]);

  // Queue management functions
  const editQueuedMessage = useCallback(
    (messageId: string, newContent: string) => {
      updateMessageInQueue(currentConversationId, messageId, {
        content: newContent,
      });
    },
    [currentConversationId]
  );

  const cancelQueuedMessage = useCallback(
    (messageId: string) => {
      removeFromMessageQueue(currentConversationId, messageId);
    },
    [currentConversationId]
  );

  const addMessageToQueue = useCallback(
    (content: string, files: File[] = []) => {
      addToMessageQueue(currentConversationId, {
        id: `queue-${Date.now()}`,
        content,
        files,
        timestamp: new Date(),
      });
    },
    [currentConversationId]
  );

  // Auto-process queue after streaming completes
  useEffect(() => {
    const state = getConversationState(currentConversationId);
    if (
      !state.isStreaming &&
      !state.isProcessingQueue &&
      state.messageQueue.length > 0
    ) {
      // Process next message in queue
      const nextMessage = state.messageQueue[0];
      if (nextMessage) {
        // Remove from queue first
        removeFromMessageQueue(currentConversationId, nextMessage.id);

        // Send the queued message with isFromQueue flag
        const processQueuedMessage = async () => {
          await sendMessageRef.current(
            nextMessage.content,
            nextMessage.files,
            true
          );
        };

        processQueuedMessage();
      }
    }
  }, [currentConversationId, isStreaming, messageQueue.length]);

  return {
    editQueuedMessage,
    cancelQueuedMessage,
    addMessageToQueue,
    messageQueue,
    isProcessingQueue: getConversationState(currentConversationId)?.isProcessingQueue || false,
  };
}