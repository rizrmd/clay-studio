import { useSnapshot } from "valtio";
import { store, getConversationAbortController } from "../../store/chat-store";

/**
 * Hook to select and compute derived state from the store
 */
export function useConversationStateSelector(
  conversationId?: string,
  currentConversationId: string = "new"
) {
  const snapshot = useSnapshot(store);
  
  // Get conversation state from store
  const conversationState = snapshot.conversations[currentConversationId];
  const messages = conversationState?.messages || [];
  const isLoadingMessages = conversationState?.isLoadingMessages || false;
  const error = conversationState?.error || null;
  const uploadedFiles = conversationState?.uploadedFiles || [];
  const forgottenAfterMessageId = conversationState?.forgottenAfterMessageId || null;
  const forgottenCount = conversationState?.forgottenCount || 0;
  const messageQueue = conversationState?.messageQueue || [];
  const isProcessingQueue = conversationState?.isProcessingQueue || false;

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

  const effectiveIsLoading =
    snapshot.conversations[effectiveConversationId]?.isLoading || false;
  const effectiveIsStreaming =
    snapshot.conversations[effectiveConversationId]?.isStreaming || false;
  const effectiveActiveTools =
    snapshot.conversations[effectiveConversationId]?.activeTools || [];

  const canStop = currentConversationId
    ? getConversationAbortController(currentConversationId) !== null
    : false;

  return {
    // Basic state
    messages: returnedMessages,
    isLoading: effectiveIsLoading,
    isLoadingMessages,
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