import { useEffect } from "react";
import { logger } from "@/lib/utils/logger";
import {
  getConversationState,
  setConversationLoading,
  setConversationStreaming,
  setConversationError,
} from "@/store/chat-store";

interface StreamResumeOptions {
  projectId: string;
  conversationId?: string;
  currentConversationId: string;
  resendMessage: (content: string) => Promise<void>;
}

/**
 * Hook to handle stream resume after page refresh
 */
export function useStreamResume({
  projectId,
  conversationId,
  currentConversationId,
  resendMessage,
}: StreamResumeOptions) {
  useEffect(() => {
    if (!projectId || !currentConversationId || !conversationId) {
      return;
    }
    
    const checkTimer = setTimeout(() => {
      const state = getConversationState(currentConversationId);
      if (state.needsStreamResume && state.pendingResumeContent) {
        
        setConversationLoading(currentConversationId, true);
        setConversationStreaming(currentConversationId, true);
        
        const timeoutId = setTimeout(() => {
          logger.warn("StreamResume: Resume timeout - clearing loading states");
          setConversationLoading(currentConversationId, false);
          setConversationStreaming(currentConversationId, false);
          setConversationError(currentConversationId, "Failed to resume streaming after page refresh");
        }, 10000);
        
        // Clear the resume flag immediately to prevent multiple triggers
        state.needsStreamResume = false;
        const content = state.pendingResumeContent;
        state.pendingResumeContent = null;
        state.resumeWithoutRemovingMessage = false;
        
        clearTimeout(timeoutId);
        
        if (content) {
          resendMessage(content);
        }
      }
    }, 500); // Wait 500ms for state to be properly set
    
    return () => clearTimeout(checkTimer);
  }, [currentConversationId, projectId, conversationId, resendMessage]);
}