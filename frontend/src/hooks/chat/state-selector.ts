import { useSnapshot } from "valtio";
import { conversationStore } from "@/store/chat/conversation-store";
import { abortControllerManager } from "../../utils/chat/abort-controller-manager";

/**
 * Hook to select and compute derived state from the store
 */
export function useConversationStateSelector(
  conversationId?: string,
  currentConversationId: string = "new"
) {
  const snapshot = useSnapshot(conversationStore);
  
  // Get conversation state from store
  const conversationState = snapshot.conversations[currentConversationId];
  const messages = conversationState?.messages || [];
  const isLoading = conversationState?.status === 'loading';
  const error = conversationState?.error || null;
  const uploadedFiles = conversationState?.uploadedFiles || [];
  const forgottenAfterMessageId = conversationState?.forgottenAfterMessageId || null;
  const forgottenCount = conversationState?.forgottenCount || 0;
  const messageQueue = conversationState?.messageQueue || [];
  const isProcessingQueue = conversationState?.status === 'processing_queue';

  // Determine returned values
  const activeConversationId = snapshot.activeConversationId;
  const shouldNavigateFromNew =
    conversationId === "new" &&
    activeConversationId &&
    activeConversationId !== "new" &&
    activeConversationId.startsWith("conv-");

  const returnedConversationId =
    conversationId === "new"
      ? shouldNavigateFromNew
        ? activeConversationId
        : "new"
      : currentConversationId;

  const returnedMessages =
    conversationId === "new" &&
    activeConversationId &&
    activeConversationId !== "new"
      ? snapshot.conversations[activeConversationId]?.messages?.length > 0
        ? snapshot.conversations[activeConversationId].messages
        : messages
      : messages;

  const effectiveConversationId =
    conversationId === "new" &&
    activeConversationId &&
    activeConversationId !== "new"
      ? activeConversationId
      : currentConversationId;

  const effectiveState = snapshot.conversations[effectiveConversationId];
  const effectiveIsLoading = effectiveState?.status === 'loading' || false;
  const effectiveIsStreaming = effectiveState?.status === 'streaming' || false;
  const effectiveActiveTools = effectiveState?.activeTools || [];

  const canStop = currentConversationId
    ? abortControllerManager.has(currentConversationId)
    : false;

  return {
    // Basic state
    messages: returnedMessages,
    isLoading: effectiveIsLoading,
    isLoadingMessages: isLoading,
    isStreaming: effectiveIsStreaming,
    error,
    uploadedFiles,
    
    // Forgotten messages
    forgottenAfterMessageId,
    forgottenCount,
    hasForgottenMessages: forgottenAfterMessageId !== null,
    
    // Queue
    messageQueue,
    isProcessingQueue,
    
    // Tools and controls
    activeTools: effectiveActiveTools,
    canStop,
    
    // Conversation ID
    conversationId: returnedConversationId,
  };
}