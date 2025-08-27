import { useCallback } from "react";
import {
  store,
  getConversationState,
  setConversationLoading,
  setConversationError,
  setConversationAbortController,
  getConversationAbortController,
  setConversationForgotten,
  clearActiveTools,
  addMessage,
} from "../../store/chat-store";
import { StreamingHandler } from "./streaming-handler";
import { uploadFiles } from "./message-utils";

interface MessageSenderOptions {
  projectId: string;
  currentConversationId: string;
  forgottenAfterMessageId: string | null;
  addMessageToQueue: (content: string, files?: File[]) => void;
}

/**
 * Hook for sending and resending messages
 */
export function useMessageSender({
  projectId,
  currentConversationId,
  forgottenAfterMessageId,
  addMessageToQueue,
}: MessageSenderOptions) {
  
  // Function to send message
  const sendMessage = useCallback(
    async (content: string, files?: File[], isFromQueue: boolean = false) => {
      if (!projectId) {
        setConversationError(currentConversationId, "Project ID is required");
        return;
      }

      const state = getConversationState(currentConversationId);
      
      // Check if we should queue the message
      if (!isFromQueue && (state.isStreaming || state.isProcessingQueue)) {
        addMessageToQueue(content, files || []);
        return;
      }

      const targetConversationId = currentConversationId;

      // Clear forgotten state if needed
      if (forgottenAfterMessageId) {
        setConversationForgotten(targetConversationId, null, 0);
      }

      // Upload files first if any
      let uploadedFilePaths: string[] = [];
      if (files && files.length > 0) {
        try {
          uploadedFilePaths = await uploadFiles(files, projectId, currentConversationId);
        } catch (err) {
          setConversationError(
            currentConversationId,
            `File upload failed: ${err instanceof Error ? err.message : "Unknown error"}`
          );
          setConversationLoading(currentConversationId, false);
          return;
        }
      }

      // Create AbortController for this request
      const abortController = new AbortController();
      setConversationAbortController(targetConversationId, abortController);

      setConversationLoading(targetConversationId, true);
      setConversationError(targetConversationId, null);

      try {
        // Add user message to local state
        let messageContent = content;
        if (uploadedFilePaths.length > 0) {
          messageContent += `\n\nAttached files:\n${uploadedFilePaths
            .map((f) => `- ${f}`)
            .join("\n")}`;
        }

        const userMessage = {
          id: `temp-${Date.now()}`,
          role: "user" as const,
          content: messageContent,
          createdAt: new Date().toISOString(),
        };
        addMessage(targetConversationId, userMessage);

        // Handle streaming
        await StreamingHandler.handleStream({
          projectId,
          conversationId: targetConversationId,
          content,
          uploadedFilePaths,
          abortController,
        });

      } catch (err) {
        if (err instanceof DOMException && err.name === "AbortError") {
          // Request was cancelled
        } else {
          setConversationError(
            targetConversationId,
            err instanceof Error ? err.message : "An error occurred"
          );
        }
      } finally {
        const finalActiveId = targetConversationId === "new" && store.activeConversationId && store.activeConversationId !== "new"
          ? store.activeConversationId
          : targetConversationId;
        
        setConversationLoading(finalActiveId, false);
        setConversationAbortController(finalActiveId, null);
        clearActiveTools(finalActiveId);
        
        if (targetConversationId === "new" && finalActiveId !== "new") {
          setConversationLoading("new", false);
          setConversationAbortController("new", null);
          clearActiveTools("new");
        }
      }
    },
    [projectId, currentConversationId, forgottenAfterMessageId, addMessageToQueue]
  );

  // Function to resend message
  const resendMessage = useCallback(
    async (content: string) => {
      if (!projectId) {
        setConversationError(currentConversationId, "Project ID is required");
        return;
      }

      const state = getConversationState(currentConversationId);
      
      if (state.isStreaming || state.isProcessingQueue) {
        addMessageToQueue(content, []);
        return;
      }

      const targetConversationId = currentConversationId;

      // Remove last assistant message if exists
      const currentMessages = state.messages;
      if (
        currentMessages.length > 0 &&
        currentMessages[currentMessages.length - 1].role === "assistant"
      ) {
        state.messages = currentMessages.slice(0, -1);
      }

      // Create AbortController
      const abortController = new AbortController();
      setConversationAbortController(targetConversationId, abortController);

      setConversationLoading(targetConversationId, true);
      setConversationError(targetConversationId, null);

      try {
        await StreamingHandler.handleStream({
          projectId,
          conversationId: targetConversationId,
          content,
          abortController,
          isResend: true,
        });
      } catch (err) {
        if (!(err instanceof DOMException && err.name === "AbortError")) {
          setConversationError(
            targetConversationId,
            err instanceof Error ? err.message : "An error occurred"
          );
        }
      } finally {
        setConversationLoading(targetConversationId, false);
        setConversationAbortController(targetConversationId, null);
        clearActiveTools(targetConversationId);
      }
    },
    [projectId, currentConversationId, addMessageToQueue]
  );

  // Function to stop/cancel current request
  const stopMessage = useCallback(() => {
    if (currentConversationId) {
      const controller = getConversationAbortController(currentConversationId);
      if (controller) {
        controller.abort();
      }
    }
  }, [currentConversationId]);

  return {
    sendMessage,
    resendMessage,
    stopMessage,
  };
}