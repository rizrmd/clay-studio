import { useEffect, useCallback, useMemo, useRef } from "react";
import { useSnapshot } from "valtio";
import { useNavigate } from "react-router-dom";
import {
  conversationStore,
  getOrCreateConversationState,
} from "../store/chat/conversation-store";
import { ConversationManager } from "../store/chat/conversation-manager";
import { MessageService } from "../services/chat/message-service";
import { chatEventBus } from "../services/chat/event-bus";
import { abortControllerManager } from "../utils/chat/abort-controller-manager";
import { WebSocketService } from "../services/chat/websocket-service";
import { useConversationContext, useProjectContext } from "./chat/use-context";
import type { Message } from "../types/chat";

/**
 * Simplified chat hook using the new ConversationManager architecture
 */
export function useChat(projectId: string, conversationId?: string) {
  const navigate = useNavigate();
  const snapshot = useSnapshot(conversationStore);

  // Service instances
  const conversationManager = useMemo(
    () => ConversationManager.getInstance(),
    []
  );
  const messageService = useMemo(() => MessageService.getInstance(), []);

  // Current conversation ID (default to 'new' if not provided)
  const currentConversationId = conversationId || "new";
  const previousConversationId = useRef<string | null>(null);

  // Ensure conversation state exists
  useEffect(() => {
    if (!conversationStore.conversations[currentConversationId]) {
      getOrCreateConversationState(currentConversationId);
    }
  }, [currentConversationId]);

  // Get conversation state from snapshot
  const conversationState = snapshot.conversations[currentConversationId] || {
    id: currentConversationId,
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

  // Load context
  const { context: conversationContext } = useConversationContext(
    conversationId && conversationId !== "new" ? conversationId : null
  );
  const { projectContext } = useProjectContext(projectId);

  // Set project ID and subscribe to WebSocket
  useEffect(() => {
    conversationStore.currentProjectId = projectId;

    // Subscribe to WebSocket for this project and conversation
    const wsService = WebSocketService.getInstance();
    wsService.subscribe(projectId, currentConversationId);

    return () => {
      // Note: Don't unsubscribe on cleanup as other components might still need it
      // The WebSocket will handle reconnection and resubscription automatically
    };
  }, [projectId, currentConversationId]);

  // Handle conversation switching
  useEffect(() => {
    if (previousConversationId.current !== currentConversationId) {
      // First, ensure the active conversation is properly set
      // This is critical for preventing message bleeding
      conversationStore.activeConversationId = currentConversationId;

      // Update WebSocket service's current conversation
      const wsService = WebSocketService.getInstance();
      wsService.setCurrentConversation(currentConversationId);

      // Switch conversation atomically
      conversationManager.switchConversation(currentConversationId);

      // For existing conversations, messages will be loaded via WebSocket
      // For new conversations, ensure we start with a clean state
      if (currentConversationId === "new") {
        conversationManager.clearConversation(currentConversationId);
      } else {
        // Set loading state while waiting for WebSocket to send messages
        conversationManager.updateStatus(currentConversationId, 'loading');
      }
      // Note: Messages for existing conversations will be sent via WebSocket
      // when we subscribe to the conversation

      previousConversationId.current = currentConversationId;
    }
  }, [
    currentConversationId,
    projectId,
    conversationManager,
    navigate,
  ]);

  // Handle navigation when new conversation gets real ID during streaming
  useEffect(() => {
    // Only redirect if:
    // 1. We're currently on '/new'
    // 2. There's an active conversation that's not 'new'
    // 3. We're currently streaming (indicating this is a transition, not just leftover state)
    const activeState = snapshot.activeConversationId
      ? snapshot.conversations[snapshot.activeConversationId]
      : null;

    if (
      conversationId === "new" &&
      snapshot.activeConversationId &&
      snapshot.activeConversationId !== "new" &&
      activeState &&
      (activeState.status === "streaming" || activeState.status === "loading")
    ) {
      // Navigate to the real conversation
      navigate(`/chat/${projectId}/${snapshot.activeConversationId}`, {
        replace: true,
      });
    }
  }, [
    conversationId,
    snapshot.activeConversationId,
    projectId,
    navigate,
    snapshot.conversations,
  ]);

  // Subscribe to conversation creation events
  useEffect(() => {
    const unsubscribe = chatEventBus.subscribe(
      "CONVERSATION_CREATED",
      async (event) => {
        if (
          event.type === "CONVERSATION_CREATED" &&
          event.projectId === projectId
        ) {
          // Refresh sidebar or do other UI updates
          window.dispatchEvent(
            new CustomEvent("conversation-created", {
              detail: { conversationId: event.conversationId, projectId },
            })
          );
        }
      }
    );

    return unsubscribe;
  }, [projectId]);

  // Subscribe to conversation redirect events
  useEffect(() => {
    const unsubscribe = chatEventBus.subscribe(
      "CONVERSATION_REDIRECT",
      async (event) => {
        if (event.type === "CONVERSATION_REDIRECT") {
          // Navigate to the new conversation ID
          navigate(`/chat/${projectId}/${event.newConversationId}`, {
            replace: true,
          });
        }
      }
    );

    return unsubscribe;
  }, [projectId, navigate]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      // Don't abort on unmount - let messages complete
    };
  }, []);

  // Send message
  const sendMessage = useCallback(
    async (content: string, files?: File[]) => {
      await messageService.sendMessage(
        projectId,
        currentConversationId,
        content,
        files
      );
    },
    [messageService, projectId, currentConversationId]
  );

  // Trigger response without adding user message (for resend scenarios)
  const triggerResponse = useCallback(
    async (content: string) => {
      // Remove last assistant message if exists
      const messages = conversationState.messages;
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
      conversationState.messages,
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
      const queue = conversationState.messageQueue;
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
          files: [...message.files],
        });
      }
    },
    [conversationManager, currentConversationId, conversationState.messageQueue]
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

  // Determine effective conversation ID for display
  // When user is explicitly on /new, stay on new unless we're actively streaming a transition
  const isActivelyStreaming =
    snapshot.activeConversationId &&
    snapshot.conversations[snapshot.activeConversationId] &&
    (snapshot.conversations[snapshot.activeConversationId].status ===
      "streaming" ||
      snapshot.conversations[snapshot.activeConversationId].status ===
        "loading");

  const effectiveConversationId =
    conversationId === "new" &&
    snapshot.activeConversationId &&
    snapshot.activeConversationId !== "new" &&
    isActivelyStreaming
      ? snapshot.activeConversationId
      : currentConversationId;

  // Get effective state - ensure it exists in the store
  useEffect(() => {
    if (
      effectiveConversationId &&
      !conversationStore.conversations[effectiveConversationId]
    ) {
      getOrCreateConversationState(effectiveConversationId);
    }
  }, [effectiveConversationId]);

  const effectiveState =
    snapshot.conversations[effectiveConversationId] || conversationState;

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
