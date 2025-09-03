import { api } from "@/lib/utils/api";
import {
  getConversationState,
  updateConversationMessages,
  setConversationError,
  setConversationForgotten,
  setConversationUploadedFiles,
  addConversationUploadedFile,
} from "@/store/chat-store";
import type { Message } from "../../types/chat";

/**
 * Load messages for a conversation from the API
 */
export async function loadConversationMessages(
  conversationId: string,
  projectId: string,
  navigate: (path: string) => void
): Promise<void> {
  const state = getConversationState(conversationId);
  state.messages = [];
  state.error = null;
  state.isLoadingMessages = true;

  try {
    const response = await api.fetchStream(
      `/conversations/${conversationId}/messages`
    );

    if (!response.ok) {
      let errorMessage = "Failed to load conversation";

      switch (response.status) {
        case 404:
          errorMessage = "This conversation doesn't exist or has been deleted";
          // Clear from localStorage if this was the last viewed conversation
          const lastConversationKey = `last_conversation_${projectId}`;
          const lastConversationId = localStorage.getItem(lastConversationKey);
          if (lastConversationId === conversationId) {
            localStorage.removeItem(lastConversationKey);
          }
          // Navigate to new conversation when not found
          if (projectId) {
            navigate(`/chat/${projectId}/new`);
          }
          break;
        case 403:
          errorMessage = "You don't have permission to access this conversation";
          break;
        case 500:
          errorMessage = "Server error while loading the conversation. Please try again";
          break;
        default:
          try {
            const errorData = await response.json();
            errorMessage = errorData.message || errorMessage;
          } catch {
            errorMessage = `Failed to load conversation (${response.status})`;
          }
      }

      throw new Error(errorMessage);
    }

    const data = await response.json();
    updateConversationMessages(conversationId, [...data]);
    setConversationError(conversationId, null);
    
    const convState = getConversationState(conversationId);
    convState.isLoadingMessages = false;
    
    // Check if we need to resume streaming (page refresh during streaming)
    checkStreamResume(conversationId, data, convState);
    
  } catch (err) {
    setConversationError(
      conversationId,
      err instanceof Error ? err.message : "Failed to load conversation"
    );
    const convState = getConversationState(conversationId);
    convState.isLoadingMessages = false;
  }
}

/**
 * Check if we need to resume streaming after page refresh
 */
function checkStreamResume(conversationId: string, messages: Message[], state: any): void {
  let shouldResumeStreaming = false;
  let lastUserMessage = null;
  
  if (messages.length > 0) {
    const lastMessage = messages[messages.length - 1];
    
    if (lastMessage.role === 'user') {
      // The assistant hasn't responded yet - likely interrupted by refresh
      shouldResumeStreaming = true;
      lastUserMessage = lastMessage;
    }
  }
  
  if (shouldResumeStreaming && lastUserMessage) {
    // Extract the content without file attachments
    let content = lastUserMessage.content;
    const attachedFilesIndex = content.indexOf("\n\nAttached files:");
    if (attachedFilesIndex > -1) {
      content = content.substring(0, attachedFilesIndex);
    }
    
    state.needsStreamResume = true;
    state.pendingResumeContent = content.trim();
    state.resumeWithoutRemovingMessage = true;
    state.conversationIdForResume = conversationId;
    state.isLoading = true;
    state.isStreaming = true;
  } else {
    // Clear loading states - conversation is complete
    state.isLoading = false;
    state.isStreaming = false;
  }
}

/**
 * Load uploaded files for a conversation
 */
export async function loadUploadedFiles(
  projectId: string,
  conversationId: string
): Promise<void> {
  if (!projectId || !conversationId || conversationId === "new") {
    setConversationUploadedFiles(conversationId, []);
    return;
  }

  try {
    const clientId = localStorage.getItem("activeClientId");
    if (!clientId) return;

    const response = await api.fetchStream(
      `/uploads?client_id=${clientId}&project_id=${projectId}&conversation_id=${conversationId}`
    );

    if (response.ok) {
      const files = await response.json();
      setConversationUploadedFiles(conversationId, files);
    }
  } catch (err) {
    // Failed to load uploaded files - silently continue
  }
}

/**
 * Check forgotten status for a conversation
 */
export async function checkForgottenStatus(conversationId: string): Promise<void> {
  try {
    const response = await api.fetchStream(
      `/conversations/${conversationId}/forget-after`
    );
    
    if (response.ok) {
      const data = await response.json();
      if (data && data.has_forgotten) {
        setConversationForgotten(
          conversationId,
          data.forgotten_after_message_id,
          data.forgotten_count || 0
        );
      }
    }
  } catch {
    // Silently handle forgotten status errors
  }
}

/**
 * Forget messages after a specific message ID
 */
export async function forgetMessagesFrom(
  conversationId: string,
  messageId: string,
  messages: Message[]
): Promise<void> {
  try {
    // First update UI optimistically to prevent flickering
    const messageIndex = messages.findIndex((m) => m.id === messageId);
    if (messageIndex !== -1) {
      // Filter messages locally immediately
      const filteredMessages = messages
        .slice(0, messageIndex + 1)
        .map((msg) => ({
          ...msg,
          file_attachments: msg.file_attachments
            ? [...msg.file_attachments]
            : undefined,
        }));
      
      // Update UI immediately to prevent flicker
      updateConversationMessages(conversationId, filteredMessages);
      
      // Set forgotten state optimistically
      const forgottenCount = messages.length - messageIndex - 1;
      setConversationForgotten(conversationId, messageId, forgottenCount);
    }
    
    // Then make the API call
    const response = await api.fetchStream(
      `/conversations/${conversationId}/forget-after`,
      {
        method: "PUT",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ message_id: messageId }),
      }
    );

    if (!response.ok) {
      // On error, reload to restore correct state
      throw new Error("Failed to forget messages");
    }

    const result = await response.json();
    
    // Update with actual count from server if different
    if (result.forgotten_count !== undefined) {
      setConversationForgotten(
        conversationId,
        messageId,
        result.forgotten_count
      );
    }
  } catch (err) {
    setConversationError(
      conversationId,
      err instanceof Error ? err.message : "Failed to forget messages"
    );
  }
}

/**
 * Restore forgotten messages
 */
export async function restoreForgottenMessages(
  conversationId: string
): Promise<void> {
  try {
    const response = await api.fetchStream(
      `/conversations/${conversationId}/forget-after`,
      {
        method: "DELETE",
      }
    );

    if (!response.ok) {
      throw new Error("Failed to restore messages");
    }

    // Clear forgotten state
    setConversationForgotten(conversationId, null, 0);

    // Reload all messages
    const messagesResponse = await api.fetchStream(
      `/conversations/${conversationId}/messages`
    );

    if (messagesResponse.ok) {
      const allMessages = await messagesResponse.json();
      updateConversationMessages(conversationId, [...allMessages]);
    }
  } catch (err) {
    setConversationError(
      conversationId,
      err instanceof Error ? err.message : "Failed to restore messages"
    );
  }
}

/**
 * Upload files for a conversation
 */
export async function uploadFiles(
  files: File[],
  projectId: string,
  conversationId: string
): Promise<string[]> {
  const uploadedFilePaths: string[] = [];
  const reusedFiles: any[] = [];
  
  const clientId = localStorage.getItem("activeClientId");
  if (!clientId) {
    throw new Error("No active client found");
  }

  for (const file of files) {
    const fileWithMeta = file as any;
    if (fileWithMeta.isExisting && fileWithMeta.filePath) {
      uploadedFilePaths.push(fileWithMeta.filePath);
      reusedFiles.push({
        id: fileWithMeta.fileId,
        file_path: fileWithMeta.filePath,
        original_name: fileWithMeta.name,
        description: fileWithMeta.description,
        auto_description: fileWithMeta.autoDescription,
      });
    } else {
      const formData = new FormData();
      formData.append("file", file);

      const response = await api.fetchStream(
        `/upload?client_id=${clientId}&project_id=${projectId}`,
        {
          method: "POST",
          body: formData,
        }
      );

      if (!response.ok) {
        throw new Error(`Failed to upload ${file.name}`);
      }

      const result = await response.json();
      uploadedFilePaths.push(result.file_path);
      addConversationUploadedFile(conversationId, result);
    }
  }

  if (reusedFiles.length > 0) {
    const existingFiles = getConversationState(conversationId).uploadedFiles;
    const existingIds = new Set(existingFiles.map((f) => f.id));
    const newFiles = reusedFiles.filter((f) => !existingIds.has(f.id));
    newFiles.forEach((file) =>
      addConversationUploadedFile(conversationId, file)
    );
  }

  return uploadedFilePaths;
}