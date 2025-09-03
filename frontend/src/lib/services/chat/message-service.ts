import { api } from "@/lib/utils/api";
import { logger } from "@/lib/utils/logger";
import { ConversationManager } from "@/store/chat/conversation-manager";
import { conversationStore } from "@/store/chat/conversation-store";
import { chatEventBus } from "./event-bus";
import { WebSocketService } from "./websocket-service";
import type { Message } from "@/types/chat";
import type { QueuedMessage } from "@/store/chat/types";

export class MessageService {
  private static instance: MessageService;
  private conversationManager: ConversationManager;
  private sendingMessages = new Set<string>(); // Track messages being sent

  private constructor() {
    this.conversationManager = ConversationManager.getInstance();
  }

  static getInstance(): MessageService {
    if (!MessageService.instance) {
      MessageService.instance = new MessageService();
    }
    return MessageService.instance;
  }

  // Send a message with queue management
  async sendMessage(
    projectId: string,
    conversationId: string,
    content: string,
    files?: File[],
    isFromQueue: boolean = false
  ): Promise<void> {
    // Create unique key for deduplication
    const messageKey = `${conversationId}-${content.substring(
      0,
      50
    )}-${Date.now()}`;

    // Check if already sending this message
    if (this.sendingMessages.has(messageKey)) {
      logger.warn(
        "MessageService: Duplicate send attempt blocked:",
        messageKey
      );
      return;
    }

    // Handle 'new' conversation - create it first
    let effectiveConversationId = conversationId;

    try {
      this.sendingMessages.add(messageKey);
      if (conversationId === 'new') {
        try {
          const response = await api.fetchStream("/conversations", {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
            },
            body: JSON.stringify({
              project_id: projectId,
            }),
          });

          if (!response.ok) {
            throw new Error("Failed to create new conversation");
          }

          const newConversation = await response.json();
          effectiveConversationId = newConversation.id;

          // Update the conversation store to use the new ID
          conversationStore.activeConversationId = effectiveConversationId;

          logger.debug(`Created new conversation ${effectiveConversationId} for project ${projectId}`);
        } catch (error) {
          logger.error("Failed to create conversation:", error);
          throw new Error("Failed to create new conversation");
        }
      }

      const state = conversationStore.conversations[effectiveConversationId];

      // Queue message if busy (unless it's from queue to prevent infinite loop)
      if (
        !isFromQueue &&
        state &&
        (state.status === "streaming" || state.status === "processing_queue")
      ) {
        const queuedMessage: QueuedMessage = {
          id: `queue-${Date.now()}`,
          content,
          files: files || [],
          timestamp: new Date(),
        };

        await this.conversationManager.addToQueue(
          effectiveConversationId,
          queuedMessage
        );
        return;
      }

      // Note: WebSocket doesn't need abort controllers like SSE did

      // Update status
      await this.conversationManager.updateStatus(effectiveConversationId, "streaming");
      await this.conversationManager.setError(effectiveConversationId, null);

      // Upload files if any
      let uploadedFilePaths: string[] = [];
      if (files && files.length > 0) {
        uploadedFilePaths = await this.uploadFiles(
          files,
          projectId,
          effectiveConversationId
        );
      }

      // Prepare message content
      let messageContent = content;
      if (uploadedFilePaths.length > 0) {
        messageContent += `\n\nAttached files:\n${uploadedFilePaths
          .map((f) => `- ${f}`)
          .join("\n")}`;
      }

      // Add user message to state
      const userMessage: Message = {
        id: `temp-${Date.now()}`,
        role: "user",
        content: messageContent,
        createdAt: new Date().toISOString(),
      };
      await this.conversationManager.addMessage(effectiveConversationId, userMessage);

      // Start streaming
      await chatEventBus.emit({
        type: "STREAMING_STARTED",
        conversationId: effectiveConversationId,
      });

      // Use WebSocket to send message instead of SSE
      const wsService = WebSocketService.getInstance();
      await wsService.connect();

      // Only subscribe if not already subscribed to this conversation
      // The useChat hook should have already established the subscription
      wsService.subscribe(projectId, effectiveConversationId);
      wsService.setCurrentConversation(effectiveConversationId);

      // Send the message via WebSocket
      wsService.sendChatMessage(
        projectId,
        effectiveConversationId,
        content,
        uploadedFilePaths
      );

      // If this was a 'new' conversation, emit a redirect event to update the URL
      if (conversationId === 'new') {
        await chatEventBus.emit({
          type: 'CONVERSATION_REDIRECT',
          oldConversationId: 'new',
          newConversationId: effectiveConversationId,
        });
      }
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : "Failed to send message";
      await this.conversationManager.setError(effectiveConversationId, errorMessage);
      // Only set to idle if there was an error
      await this.conversationManager.updateStatus(effectiveConversationId, "idle");
      throw error;
    } finally {
      this.sendingMessages.delete(messageKey);
      // Don't set status to idle here - let streaming service handle it
      // The streaming has already been handled, so we can process queue
      await this.processQueue(projectId, effectiveConversationId);
    }
  }

  // Process message queue
  private async processQueue(
    projectId: string,
    conversationId: string
  ): Promise<void> {
    const nextMessage = await this.conversationManager.getNextQueuedMessage(
      conversationId
    );

    if (!nextMessage) {
      return;
    }

    // Update status to show queue processing
    await this.conversationManager.updateStatus(
      conversationId,
      "processing_queue"
    );

    // Send the queued message
    await this.sendMessage(
      projectId,
      conversationId,
      nextMessage.content,
      nextMessage.files,
      true // Mark as from queue
    );
  }

  // Load messages (only from WebSocket cache)
  async loadMessages(
    conversationId: string,
    _projectId: string
  ): Promise<Message[]> {
    // IMPORTANT: Verify we're loading messages for the right conversation
    // This prevents message bleeding when multiple conversations are being loaded
    const activeConversationId = conversationStore.activeConversationId;

    // Only proceed if this is the active conversation or if there's no active conversation
    if (activeConversationId && activeConversationId !== conversationId) {
      logger.warn(
        `MessageService: Skipping loadMessages for inactive conversation ${conversationId}, active is ${activeConversationId}`
      );
      return [];
    }

    // Messages should already be loaded via WebSocket ConversationHistory message
    const existingState = conversationStore.conversations[conversationId];
    if (existingState && existingState.messages) {
      return existingState.messages;
    }

    // If no messages, return empty array (WebSocket will send them when ready)
    return [];
  }

  // Forget messages from a point
  async forgetMessagesFrom(
    conversationId: string,
    messageId: string
  ): Promise<void> {
    // Stop any ongoing streaming

    try {
      // Get current messages for optimistic update
      const state = conversationStore.conversations[conversationId];
      if (state && state.messages) {
        const messageIndex = state.messages.findIndex(
          (m) => m.id === messageId
        );
        if (messageIndex !== -1) {
          // Optimistically update UI to prevent flickering
          const filteredMessages = state.messages.slice(0, messageIndex + 1);
          await this.conversationManager.setMessages(
            conversationId,
            filteredMessages
          );

          // Set forgotten state optimistically
          const forgottenCount = state.messages.length - messageIndex - 1;
          await this.conversationManager.setForgottenState(
            conversationId,
            messageId,
            forgottenCount
          );
        }
      }

      // Make the API call
      const response = await api.fetchStream(
        `/conversations/${conversationId}/forget-after`,
        {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ message_id: messageId }),
        }
      );

      if (!response.ok) {
        // On error, reload to restore correct state
        await this.loadMessages(
          conversationId,
          conversationStore.currentProjectId || ""
        );
        throw new Error(`Failed to forget messages: ${response.status}`);
      }

      const data = await response.json();

      // Update with actual count from server if different
      if (data.forgotten_count !== undefined) {
        await this.conversationManager.setForgottenState(
          conversationId,
          messageId,
          data.forgotten_count
        );
      }

      // Don't reload messages - we already updated optimistically
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : "Failed to forget messages";
      await this.conversationManager.setError(conversationId, errorMessage);
      throw error;
    }
  }

  // Restore forgotten messages
  async restoreForgottenMessages(conversationId: string): Promise<void> {
    try {
      const response = await api.fetchStream(
        `/conversations/${conversationId}/forget-after`,
        {
          method: "DELETE",
        }
      );

      if (!response.ok) {
        throw new Error(`Failed to restore messages: ${response.status}`);
      }

      // Clear forgotten state
      await this.conversationManager.setForgottenState(conversationId, null, 0);

      // Reload messages
      await this.loadMessages(
        conversationId,
        conversationStore.currentProjectId || ""
      );
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : "Failed to restore messages";
      await this.conversationManager.setError(conversationId, errorMessage);
      throw error;
    }
  }

  // Upload files
  private async uploadFiles(
    files: File[],
    projectId: string,
    conversationId: string
  ): Promise<string[]> {
    const formData = new FormData();
    files.forEach((file) => formData.append("files", file));

    const clientId = localStorage.getItem("activeClientId");
    if (clientId) {
      formData.append("client_id", clientId);
    }
    formData.append("project_id", projectId);
    formData.append("conversation_id", conversationId);

    const response = await api.fetchStream("/upload", {
      method: "POST",
      body: formData,
    });

    if (!response.ok) {
      throw new Error(`File upload failed: ${response.status}`);
    }

    const data = await response.json();
    return data.file_paths || [];
  }

  // Stop current message
  async stopMessage(conversationId: string): Promise<void> {
    // Send stop streaming message via WebSocket
    const wsService = WebSocketService.getInstance();
    wsService.stopStreaming(conversationId);

    await this.conversationManager.updateStatus(conversationId, "idle");
    await this.conversationManager.clearQueue(conversationId);
  }
}
