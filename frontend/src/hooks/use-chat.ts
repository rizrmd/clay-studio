import { useEffect, useCallback, useMemo, useRef } from "react";
import { useSnapshot } from "valtio";
import { useNavigate, useLocation } from "react-router-dom";
import { logger } from "../lib/utils/logger";
import {
  conversationStore,
  getOrCreateConversationState,
} from "../store/chat/conversation-store";
import { ConversationManager } from "../store/chat/conversation-manager";
import { MessageService } from "../lib/services/chat/message-service";
import { MessageCacheService } from "../lib/services/chat/message-cache";
import { chatEventBus } from "../lib/services/chat/event-bus";
import { abortControllerManager } from "../utils/chat/abort-controller-manager";
import { WebSocketService } from "../lib/services/chat/websocket-service";
import { useConversationContext, useProjectContext } from "./chat/use-context";
import type { Message } from "../types/chat";

/**
 * Simplified chat hook using the new ConversationManager architecture
 */
export function useChat(projectId: string, conversationId: string) {
  const navigate = useNavigate();
  const location = useLocation();
  const snapshot = useSnapshot(conversationStore, { sync: true });

  // Service instances
  const conversationManager = useMemo(
    () => ConversationManager.getInstance(),
    []
  );
  const messageService = useMemo(() => MessageService.getInstance(), []);
  const messageCache = useMemo(() => MessageCacheService.getInstance(), []);

  // Current conversation ID (required)
  const currentConversationId = conversationId;
  const previousConversationId = useRef<string | null>(null);

  // Ensure conversation state exists
  useEffect(() => {
    if (!conversationStore.conversations[currentConversationId]) {
      getOrCreateConversationState(currentConversationId);
    }
  }, [currentConversationId]);

  // Get conversation state from snapshot (ensure it exists)
  const conversationState = snapshot.conversations[currentConversationId] || getOrCreateConversationState(currentConversationId);

  // Load context
  const { context: conversationContext } = useConversationContext(conversationId);
  const { projectContext } = useProjectContext(projectId);

  // Set project ID
  useEffect(() => {
    conversationStore.currentProjectId = projectId;
  }, [projectId]);

  // Handle conversation switching and WebSocket subscription
  useEffect(() => {
    if (previousConversationId.current !== currentConversationId) {
      // Set active conversation first to prevent message bleeding
      conversationStore.activeConversationId = currentConversationId;
      
      // Handle 'new' conversations differently - don't set up WebSocket or load messages
      if (currentConversationId === 'new') {
        conversationManager.switchConversation(currentConversationId);
        conversationManager.updateStatus(currentConversationId, 'idle');
        previousConversationId.current = currentConversationId;
        return;
      }

      // For real conversations, set up WebSocket and load messages
      const wsService = WebSocketService.getInstance();
      wsService.setCurrentConversation(currentConversationId);
      wsService.subscribe(projectId, currentConversationId);
      conversationManager.switchConversation(currentConversationId);

      // Check for transition from new chat first
      const state = location.state as {
        fromNewChat?: boolean;
        existingMessages?: Message[];
      } | null;
      
      if (state?.fromNewChat && state.existingMessages) {
        // Set existing messages immediately to prevent loading state
        conversationManager.setMessages(currentConversationId, state.existingMessages);
        conversationManager.updateStatus(currentConversationId, 'idle');
        logger.debug(`Loaded ${state.existingMessages.length} messages from new chat transition`);
        previousConversationId.current = currentConversationId;
        return;
      }

      // Check for cached messages first for instant loading
      const cachedMessages = messageCache.getCachedMessages(currentConversationId);

      if (cachedMessages && cachedMessages.length > 0) {
        conversationManager.setMessages(currentConversationId, cachedMessages);
        conversationManager.updateStatus(currentConversationId, 'idle');
        logger.debug(`Loaded ${cachedMessages.length} cached messages for conversation ${currentConversationId}`);
      } else {
        // No cache available - check existing state
        const existingState = conversationStore.conversations[currentConversationId];
        const hasMessages = existingState?.messages?.length > 0;
        const isStreaming = existingState?.status === 'streaming';

        // Only show loading if truly necessary
        if (!hasMessages && !isStreaming) {
          conversationManager.updateStatus(currentConversationId, 'loading');

          const timeoutId = setTimeout(() => {
            const currentState = conversationStore.conversations[currentConversationId];
            if (currentState?.status === 'loading') {
              logger.warn('WebSocket response timeout for conversation:', currentConversationId);
              conversationManager.updateStatus(currentConversationId, 'idle');
            }
          }, 2000);

          return () => clearTimeout(timeoutId);
        }
      }

      previousConversationId.current = currentConversationId;
    }
  }, [currentConversationId, projectId, conversationManager, location.state]);


  // Handle conversation redirect (should be rare now that we pre-create conversations)
  useEffect(() => {
    const unsubscribe = chatEventBus.subscribe(
      "CONVERSATION_REDIRECT",
      async (event: any) => {
        if (
          event.type === "CONVERSATION_REDIRECT" &&
          event.oldConversationId === conversationId
        ) {
          navigate(`/p/${projectId}/c/${event.newConversationId}`, { replace: true });
        }
      }
    );

    return unsubscribe;
  }, [conversationId, projectId, navigate]);


  // Cleanup on unmount
  useEffect(() => {
    return () => {
      // Don't abort on unmount - let messages complete
    };
  }, []);

  const effectiveConversationId = currentConversationId;

  // Send message
  const sendMessage = useCallback(
    async (content: string, files?: File[]) => {
      // Use the effective conversation ID for sending messages
      // This ensures we always send to the correct conversation
      const targetConversationId = effectiveConversationId || currentConversationId;
      
      await messageService.sendMessage(
        projectId,
        targetConversationId,
        content,
        files
      );
    },
    [messageService, projectId, currentConversationId, conversationId, effectiveConversationId]
  );

  // Trigger response without adding user message (for resend scenarios)
  const triggerResponse = useCallback(
    async (content: string) => {
      // Remove last assistant message if exists
      const messages = conversationState?.messages || [];
      if (
        messages.length > 0 &&
        messages[messages.length - 1].role === "assistant"
      ) {
        const newMessages = messages.slice(0, -1);
        // Deep clone to remove readonly
        const clonedMessages = JSON.parse(JSON.stringify(newMessages));
        await conversationManager.setMessages(
          currentConversationId,
          clonedMessages
        );
      }

      // Send the message again
      await messageService.sendMessage(
        projectId,
        currentConversationId,
        content
      );
    },
    [
      conversationManager,
      messageService,
      projectId,
      currentConversationId,
      conversationState?.messages,
    ]
  );

  // Resend message (retry last assistant response) - alias for triggerResponse
  const resendMessage = useCallback(
    async (content: string) => {
      await triggerResponse(content);
    },
    [triggerResponse]
  );

  // Stop message
  const stopMessage = useCallback(() => {
    // Stop the current conversation
    messageService.stopMessage(currentConversationId);
    // Also try to stop the active conversation if it's different
    const activeId = snapshot.activeConversationId;
    if (activeId && activeId !== currentConversationId) {
      messageService.stopMessage(activeId);
    }
  }, [messageService, currentConversationId, snapshot.activeConversationId]);

  // Forget messages from a point
  const forgetMessagesFrom = useCallback(
    async (messageId: string) => {
      await messageService.forgetMessagesFrom(currentConversationId, messageId);
    },
    [messageService, currentConversationId]
  );

  // Restore forgotten messages
  const restoreForgottenMessages = useCallback(async () => {
    await messageService.restoreForgottenMessages(currentConversationId);
  }, [messageService, currentConversationId]);

  // Edit queued message
  const editQueuedMessage = useCallback(
    async (messageId: string, newContent: string) => {
      const queue = conversationState?.messageQueue || [];
      const message = queue.find((m) => m.id === messageId);
      if (message) {
        // Remove old message and add updated one
        await conversationManager.removeFromQueue(
          currentConversationId,
          messageId
        );
        await conversationManager.addToQueue(currentConversationId, {
          ...message,
          content: newContent,
          files: [...(message.files || [])] as File[]
        });
      }
    },
    [conversationManager, currentConversationId, conversationState?.messageQueue]
  );

  // Cancel queued message
  const cancelQueuedMessage = useCallback(
    async (messageId: string) => {
      await conversationManager.removeFromQueue(
        currentConversationId,
        messageId
      );
    },
    [conversationManager, currentConversationId]
  );

  // Get effective state - ensure it exists in the store
  useEffect(() => {
    if (
      effectiveConversationId &&
      !conversationStore.conversations[effectiveConversationId]
    ) {
      getOrCreateConversationState(effectiveConversationId);
    }
  }, [effectiveConversationId]);

  const effectiveState = conversationState || {
    id: currentConversationId || '',
    status: "idle" as const,
    messages: [] as Message[],
    error: null,
    uploadedFiles: [],
    forgottenAfterMessageId: null,
    forgottenCount: 0,
    messageQueue: [],
    activeTools: [],
    lastUpdated: Date.now(),
    version: 0,
  };

  // Debug logging for message state
  useEffect(() => {
    // Message state tracking
  }, [conversationId, currentConversationId, effectiveConversationId, effectiveState.messages?.length, effectiveState.status]);

  return {
    // Messages and state
    messages: (effectiveState.messages || []) as Message[],
    isLoading:
      effectiveState.status === "loading" ||
      effectiveState.status === "streaming",
    isLoadingMessages: effectiveState.status === "loading",
    isStreaming: effectiveState.status === "streaming",
    error: effectiveState.error,
    conversationId: effectiveConversationId,
    uploadedFiles: effectiveState.uploadedFiles,

    // Actions
    sendMessage,
    resendMessage,
    triggerResponse,
    stopMessage,
    forgetMessagesFrom,
    restoreForgottenMessages,

    // Control states
    canStop:
      effectiveState.status === "streaming" &&
      (abortControllerManager.has(effectiveConversationId) ||
        abortControllerManager.has(currentConversationId)),

    // Forgotten messages
    forgottenAfterMessageId: effectiveState.forgottenAfterMessageId,
    forgottenCount: effectiveState.forgottenCount,
    hasForgottenMessages: effectiveState.forgottenAfterMessageId !== null,

    // Queue management
    messageQueue: effectiveState.messageQueue,
    isProcessingQueue: effectiveState.status === "processing_queue",
    editQueuedMessage,
    cancelQueuedMessage,

    // Tool usage
    activeTools: effectiveState.activeTools,

    // Context usage
    contextUsage: effectiveState.contextUsage,

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
