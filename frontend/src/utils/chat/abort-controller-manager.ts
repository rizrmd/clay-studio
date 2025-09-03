// Re-export abort controller functionality from chat store for backward compatibility
import { getConversationAbortController, setConversationAbortController } from "@/store/chat-store";

export const abortControllerManager = {
  has: (conversationId: string): boolean => {
    return getConversationAbortController(conversationId) !== null;
  },
  get: (conversationId: string): AbortController | null => {
    return getConversationAbortController(conversationId);
  },
  set: (conversationId: string, controller: AbortController | null): void => {
    setConversationAbortController(conversationId, controller);
  },
  delete: (conversationId: string): void => {
    setConversationAbortController(conversationId, null);
  }
};