import { chatStore } from '../store/chat/chat-store';
import { wsService } from '../services/ws-service';

/**
 * Universal method to create a new chat conversation with optional prefilled text
 * Can be called from anywhere in the application
 *
 * @param projectId - The project ID to create the chat in
 * @param prefilledText - Optional text to prefill the chat input (won't be sent automatically)
 * @param autoSend - If true, automatically sends the prefilled message
 * @param navigate - React Router navigate function to navigate to the new chat
 * @returns The conversation ID (or 'new' if creating)
 */
export function createNewChat(
  projectId: string,
  options?: {
    prefilledText?: string;
    autoSend?: boolean;
    navigate?: (path: string) => void;
  }
): string {
  const { prefilledText = '', autoSend = false, navigate } = options || {};

  // Set the project in chat store
  chatStore.project_id = projectId;

  if (autoSend && prefilledText) {
    // Create conversation and send message immediately
    chatStore.pendingFirstChat = prefilledText;
    const conversationTitle =
      prefilledText.slice(0, 50).trim() + (prefilledText.length > 50 ? '...' : '');
    wsService.createConversation(projectId, conversationTitle);

    // Navigate to new chat if navigate function provided
    if (navigate) {
      navigate(`/p/${projectId}/new`);
    }

    return 'new';
  } else {
    // Just navigate with prefilled text (user will send manually)
    const conversationId = 'new';
    chatStore.conversation_id = conversationId;

    // Store the prefilled text - it will be applied after navigation
    if (prefilledText) {
      console.log('[createNewChat] Setting pendingInputText:', prefilledText);
      chatStore.pendingInputText = prefilledText;
      console.log('[createNewChat] chatStore.pendingInputText is now:', chatStore.pendingInputText);
    }

    // Navigate to new chat if navigate function provided
    if (navigate) {
      const targetPath = `/p/${projectId}/new`;
      console.log('[createNewChat] Navigating to:', targetPath);
      navigate(targetPath);
      console.log('[createNewChat] Navigation called');
    }

    return conversationId;
  }
}

/**
 * Helper to create a new chat for fixing an analysis error
 *
 * @param projectId - The project ID
 * @param analysisId - The analysis ID that failed
 * @param errorMessage - The error message to include
 * @param navigate - React Router navigate function
 */
export function createChatForAnalysisError(
  projectId: string,
  analysisId: string,
  errorMessage: string,
  navigate: (path: string) => void
): void {
  const prefilledText = `I'm getting an error when running my analysis (ID: ${analysisId}):

Error: ${errorMessage}

Can you help me fix this?`;

  console.log('[createChatForAnalysisError] Creating chat with error:', {
    projectId,
    analysisId,
    prefilledText,
  });

  createNewChat(projectId, {
    prefilledText,
    autoSend: false, // Let user review and edit before sending
    navigate,
  });
}
