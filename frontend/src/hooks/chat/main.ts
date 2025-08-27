import { useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useConversationInit } from "./conversation-init";
import { useConversationStateSelector } from "./state-selector";
import { useMessageSender } from "./message-sender";
import { useQueueManager } from "./queue-manager";
import { useStreamResume } from "./stream-resume";
import { useConversationContext, useProjectContext } from "./use-context";
import {
  forgetMessagesFrom as forgetMessages,
  restoreForgottenMessages as restoreMessages,
} from "./message-utils";
import { getConversationAbortController } from "../../store/chat-store";
import type { Message } from "../../types/chat";

// Re-export types
export type {
  FileAttachment,
  Message,
  ConversationContext,
  ConversationSummary,
  DataSourceContext,
  ToolContext,
  ProjectSettings,
  AnalysisPreferences,
  ProjectContextResponse,
  RecentActivity,
} from "../../types/chat";

/**
 * Main chat hook with Valtio state management and streaming support
 */
export function useValtioChat(projectId: string, conversationId?: string) {
  const navigate = useNavigate();
  
  // Initialize conversation
  const currentConversationId = useConversationInit({
    projectId,
    conversationId,
    navigate,
  });

  // Select state from store
  const state = useConversationStateSelector(conversationId, currentConversationId);

  // Load context
  const { context: conversationContext } = useConversationContext(
    conversationId && conversationId !== "new" ? conversationId : null
  );
  const { projectContext } = useProjectContext(projectId);

  // Create a mutable reference that will be shared
  let sendMessageRef: any = null;

  // Queue management
  const queueManager = useQueueManager({
    currentConversationId,
    isStreaming: state.isStreaming,
    messageQueue: state.messageQueue.map(m => ({
      id: m.id,
      content: m.content,
      files: [...m.files] as File[],
      timestamp: new Date(m.timestamp),
    })),
    sendMessage: (...args: any[]) => sendMessageRef?.(...args),
  });

  // Message sending
  const { sendMessage, resendMessage, stopMessage } = useMessageSender({
    projectId,
    currentConversationId,
    forgottenAfterMessageId: state.forgottenAfterMessageId,
    addMessageToQueue: queueManager.addMessageToQueue,
  });

  // Update the reference
  sendMessageRef = sendMessage;

  // Stream resume after page refresh
  useStreamResume({
    projectId,
    conversationId,
    currentConversationId,
    resendMessage,
  });

  // Function to restore forgotten messages
  const restoreForgottenMessages = useCallback(async () => {
    if (!currentConversationId || !state.forgottenAfterMessageId) return;
    await restoreMessages(currentConversationId);
  }, [currentConversationId, state.forgottenAfterMessageId]);

  // Function to forget messages from a point
  const forgetMessagesFrom = useCallback(
    async (messageId: string) => {
      if (!currentConversationId) return;

      const controller = getConversationAbortController(currentConversationId);
      if (controller) {
        controller.abort();
      }

      await forgetMessages(currentConversationId, messageId, state.messages as Message[]);
    },
    [currentConversationId, state.messages]
  );

  return {
    // Messages and state
    messages: state.messages,
    isLoading: state.isLoading,
    isLoadingMessages: state.isLoadingMessages,
    isStreaming: state.isStreaming,
    error: state.error,
    conversationId: state.conversationId,
    uploadedFiles: state.uploadedFiles,
    
    // Actions
    sendMessage,
    resendMessage,
    stopMessage,
    forgetMessagesFrom,
    restoreForgottenMessages,
    
    // Control states
    canStop: state.canStop,
    
    // Forgotten messages
    forgottenAfterMessageId: state.forgottenAfterMessageId,
    forgottenCount: state.forgottenCount,
    hasForgottenMessages: state.hasForgottenMessages,
    
    // Queue management
    messageQueue: queueManager.messageQueue,
    isProcessingQueue: queueManager.isProcessingQueue,
    editQueuedMessage: queueManager.editQueuedMessage,
    cancelQueuedMessage: queueManager.cancelQueuedMessage,
    
    // Tool usage
    activeTools: state.activeTools,
    
    // Context
    conversationContext,
    projectContext,
    
    // Smart context features
    hasDataSources: (projectContext?.data_sources.length || 0) > 0,
    availableTools:
      conversationContext?.available_tools.filter((t: any) => t.applicable) ||
      projectContext?.available_tools.filter((t: any) => t.applicable) ||
      [],
    contextStrategy: conversationContext?.context_strategy,
  };
}