import { useEffect, useCallback, useMemo, useRef } from "react";
import { useSnapshot } from "valtio";
import { useNavigate } from "react-router-dom";
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
  const snapshot = useSnapshot(conversationStore);

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

  // Get conversation state from snapshot
  const conversationState = snapshot.conversations[currentConversationId];

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
      
      // First, ensure the active conversation is properly set
      // This is critical for preventing message bleeding
      conversationStore.activeConversationId = currentConversationId;

      // Update WebSocket service's current conversation
      const wsService = WebSocketService.getInstance();
      wsService.setCurrentConversation(currentConversationId);
      
      // Subscribe to WebSocket for this project and conversation
      // This handles both initial subscription and conversation switches
      wsService.subscribe(projectId, currentConversationId);

      // Switch conversation atomically
      conversationManager.switchConversation(currentConversationId);

      // Check for cached messages first for instant loading
      const cachedMessages = messageCache.getCachedMessages(currentConversationId);
      
      if (cachedMessages && cachedMessages.length > 0) {
        // Load cached messages immediately for instant experience
        conversationManager.setMessages(currentConversationId, cachedMessages);
        conversationManager.updateStatus(currentConversationId, 'idle');
        
        logger.debug(`Loaded ${cachedMessages.length} cached messages for conversation ${currentConversationId}`);
      } else {
        // No cache available - check existing state
        const existingState = conversationStore.conversations[currentConversationId];
        const hasMessages = existingState && existingState.messages && existingState.messages.length > 0;
        const isStreaming = existingState && existingState.status === 'streaming';
        
        // Only show loading if truly necessary (no cache, no messages, not streaming)
        if (!hasMessages && !isStreaming && currentConversationId !== 'new') {
          conversationManager.updateStatus(currentConversationId, 'loading');
          
          // Much shorter timeout since cache miss is rare
          const timeoutId = setTimeout(() => {
            const currentState = conversationStore.conversations[currentConversationId];
            if (currentState && currentState.status === 'loading') {
              logger.warn('WebSocket response timeout for conversation:', currentConversationId);
              conversationManager.updateStatus(currentConversationId, 'idle');
            }
          }, 2000); // Reduced to 2 seconds
          
          return () => clearTimeout(timeoutId);
        }
      }
      
      // WebSocket will still send fresh data and update the cache

      previousConversationId.current = currentConversationId;
    }
  }, [
    currentConversationId,
    projectId,
    conversationManager,
    navigate,
  ]);


  // Handle conversation redirect
  useEffect(() => {
    const unsubscribe = chatEventBus.subscribe(
      "CONVERSATION_REDIRECT",
      async (event: any) => {
        if (
          event.type === "CONVERSATION_REDIRECT" &&
          (event.oldConversationId === conversationId || event.oldConversationId === 'new')
        ) {
          navigate(`/project/${projectId}/chat/${event.newConversationId}`, { replace: true });
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
    messages: [],
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
    messages: effectiveState.messages as Message[],
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
