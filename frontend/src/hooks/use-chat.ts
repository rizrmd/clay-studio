import { useEffect, useCallback, useMemo, useRef } from "react";
import { useSnapshot } from "valtio";
import { useNavigate } from "react-router-dom";
import { logger } from "@/lib/logger";
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
    
    // Subscribe to WebSocket for this project only
    const wsService = WebSocketService.getInstance();
    wsService.subscribe(projectId);
    
    return () => {
      // Note: Don't unsubscribe on cleanup as other components might still need it
      // The WebSocket will handle reconnection and resubscription automatically
    };
  }, [projectId]);


  // Handle conversation switching
  useEffect(() => {
    logger.debug("useChat: Conversation switch check:", {
      previous: previousConversationId.current,
      current: currentConversationId,
      shouldSwitch: previousConversationId.current !== currentConversationId,
    });

    if (previousConversationId.current !== currentConversationId) {
      logger.debug(
        "useChat: SWITCHING conversation to:",
        currentConversationId
      );
      
      // First, ensure the active conversation is properly set
      // This is critical for preventing message bleeding
      conversationStore.activeConversationId = currentConversationId;
      
      // Update WebSocket service's current conversation
      const wsService = WebSocketService.getInstance();
      wsService.setCurrentConversation(currentConversationId);
      
      // Switch conversation atomically
      conversationManager.switchConversation(currentConversationId);

      // Load messages if it's an existing conversation
      // BUT don't load if we're transitioning from a streaming 'new' conversation
      // (the messages are already being streamed)
      if (currentConversationId !== "new") {
        const previousWasNew = previousConversationId.current === "new";
        const newConversationIsStreaming =
          conversationStore.conversations[currentConversationId]?.status ===
          "streaming";

        if (!previousWasNew || !newConversationIsStreaming) {
          logger.debug(
            "useChat: Loading messages for existing conversation:",
            currentConversationId
          );
          messageService
            .loadMessages(currentConversationId, projectId)
            .catch((error) => {
              console.error("useChat: Failed to load messages:", error);

              // Navigate to new if conversation doesn't exist
              if (
                error.message?.includes("404") ||
                error.message?.includes("doesn't exist")
              ) {
                navigate(`/chat/${projectId}/new`);
              }
            });
        } else {
          logger.debug(
            "useChat: Skipping loadMessages for streaming conversation transition from new to:",
            currentConversationId
          );
        }
      } else {
        // For new conversations, always ensure we start with a clean state
        logger.debug("useChat: Clearing state for new conversation");
        conversationManager.clearConversation(currentConversationId);
      }

      previousConversationId.current = currentConversationId;
    }
  }, [
    currentConversationId,
    projectId,
    conversationManager,
    messageService,
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

    logger.debug("useChat: Redirect check:", {
      conversationId,
      activeConversationId: snapshot.activeConversationId,
      activeState: activeState ? { status: activeState.status } : null,
      shouldRedirect:
        conversationId === "new" &&
        snapshot.activeConversationId &&
        snapshot.activeConversationId !== "new" &&
        activeState &&
        (activeState.status === "streaming" ||
          activeState.status === "loading"),
    });

    if (
      conversationId === "new" &&
      snapshot.activeConversationId &&
      snapshot.activeConversationId !== "new" &&
      activeState &&
      (activeState.status === "streaming" || activeState.status === "loading")
    ) {
      logger.debug("useChat: REDIRECTING to:", snapshot.activeConversationId);
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
        logger.debug("useChat: Received CONVERSATION_CREATED event:", event);
        if (
          event.type === "CONVERSATION_CREATED" &&
          event.projectId === projectId
        ) {
          logger.debug(
            "useChat: Dispatching conversation-created event to window"
          );
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

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      // Don't abort on unmount - let messages complete
    };
  }, []);

  // Send message
  const sendMessage = useCallback(
    async (content: string, files?: File[]) => {
      logger.debug(
        "useChat: Sending message to:",
        currentConversationId,
        "project:",
        projectId
      );
      await messageService.sendMessage(
        projectId,
        currentConversationId,
        content,
        files
      );
    },
    [messageService, projectId, currentConversationId]
  );

  // Resend message (retry last assistant response)
  const resendMessage = useCallback(
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

  // Debug logging
  useEffect(() => {
    logger.debug("useChat: State for", effectiveConversationId, ":", {
      status: effectiveState.status,
      messagesCount: effectiveState.messages.length,
      isLoading:
        effectiveState.status === "loading" ||
        effectiveState.status === "streaming",
      isStreaming: effectiveState.status === "streaming",
    });
  }, [
    effectiveState.status,
    effectiveState.messages.length,
    effectiveConversationId,
  ]);

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
