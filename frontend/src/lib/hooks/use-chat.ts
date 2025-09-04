import { useCallback, useEffect } from "react";
import { useSnapshot } from "valtio";
import { wsService } from "../services/ws-service";
import { chatStore, chatActions } from "../store/chat/chat-store";
import { sidebarActions } from "../store/chat/sidebar-store";
import type { CONVERSATION_ID, Message, PROJECT_ID } from "../types/chat";
import type { ServerMessage } from "../types/ws";

export const useChat = () => {
  const snap = useSnapshot(chatStore);

  // Auto-connect WebSocket and setup event listeners
  useEffect(() => {
    // Handle conversation messages (used both for initial load and explicit requests)
    const handleConversationMessages = (
      message: ServerMessage & { type: "conversation_messages" }
    ) => {
      // Check if conversation exists in store
      let conversation = chatStore.map[message.conversation_id];

      if (!conversation) {
        // Create new conversation if it doesn't exist
        conversation = {
          id: message.conversation_id,
          project_id: chatStore.project_id, // Use chatStore directly instead of snap
          title: `Conversation ${message.conversation_id}`,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          message_count: message.messages.length,
          messages: message.messages,
        };
        chatStore.map[message.conversation_id] = conversation;
        if (!chatStore.list.includes(message.conversation_id)) {
          chatStore.list.push(message.conversation_id);
        }
      } else {
        // Update existing conversation messages
        conversation.messages = message.messages;
        conversation.message_count = message.messages.length;
      }
    };

    // Handle conversation redirect (new -> actual ID)
    const handleConversationRedirect = (
      message: ServerMessage & { type: "conversation_redirect" }
    ) => {
      if (snap.active === message.old_conversation_id) {
        chatStore.active = message.new_conversation_id;
      }
    };

    // Handle streaming progress - update message content
    const handleProgress = (message: ServerMessage & { type: "progress" }) => {
      chatActions.updateStreamingContent(message.conversation_id, message.content);
    };

    // Handle stream start - add new assistant message
    const handleStart = (message: ServerMessage & { type: "start" }) => {
      const conversation = chatStore.map[message.conversation_id];
      if (conversation) {
        const assistantMessage: Message = {
          id: message.id,
          content: "",
          role: "assistant",
          createdAt: new Date().toISOString(),
        };
        conversation.messages.push(assistantMessage);
        
        // Start streaming state tracking
        chatActions.startStreaming(message.conversation_id, message.id);
      }
    };

    // Handle stream complete - finalize message
    const handleComplete = (message: ServerMessage & { type: "complete" }) => {
      const conversation = chatStore.map[message.conversation_id];
      if (conversation) {
        const lastMessage =
          conversation.messages[conversation.messages.length - 1];
        if (lastMessage && lastMessage.id === message.id) {
          lastMessage.processing_time_ms = message.processing_time_ms;
          lastMessage.tool_usages = message.tool_usages;
        }
        conversation.message_count = conversation.messages.length;
        conversation.updated_at = new Date().toISOString();
        
        // Complete streaming state
        chatActions.completeStreaming(message.conversation_id);
      }
    };

    // Handle errors
    const handleError = (message: ServerMessage & { type: "error" }) => {
      console.error("Chat error:", message.error);
      // Could add error state to store if needed
    };

    // Handle tool start events
    const handleToolStarted = ({ tool, toolUsageId, conversationId }: { tool: string; toolUsageId: string; conversationId: string }) => {
      chatActions.addActiveToolToStream(conversationId, tool, toolUsageId);
    };

    // Handle tool completion events  
    const handleToolCompleted = ({ toolUsageId, conversationId }: { toolUsageId: string; conversationId: string }) => {
      chatActions.removeActiveToolFromStream(conversationId, toolUsageId);
    };

    // Handle new conversation management responses
    const handleConversationList = (
      message: ServerMessage & { type: "conversation_list" }
    ) => {
      // Update conversation list and map
      chatStore.list = message.conversations.map((c) => c.id);
      message.conversations.forEach((conversation) => {
        chatStore.map[conversation.id] = conversation;
      });
    };

    const handleConversationCreated = (
      message: ServerMessage & { type: "conversation_created" }
    ) => {
      // Add new conversation to store
      chatStore.map[message.conversation.id] = message.conversation;
      if (!chatStore.list.includes(message.conversation.id)) {
        chatStore.list.push(message.conversation.id);
      }
      // Optionally switch to the new conversation
      chatStore.active = message.conversation.id;
    };

    const handleConversationDetails = (
      message: ServerMessage & { type: "conversation_details" }
    ) => {
      // Update single conversation
      chatStore.map[message.conversation.id] = message.conversation;
      if (!chatStore.list.includes(message.conversation.id)) {
        chatStore.list.push(message.conversation.id);
      }
    };

    const handleConversationUpdated = (
      message: ServerMessage & { type: "conversation_updated" }
    ) => {
      // Update conversation in store
      chatStore.map[message.conversation.id] = message.conversation;
    };

    const handleConversationDeleted = (
      message: ServerMessage & { type: "conversation_deleted" }
    ) => {
      // Remove from store
      delete chatStore.map[message.conversation_id];
      chatStore.list = chatStore.list.filter(
        (id) => id !== message.conversation_id
      );

      // If we're viewing the deleted conversation, switch to another one
      if (chatStore.active === message.conversation_id) {
        chatStore.active = chatStore.list[0] || "";
      }
    };

    const handleConversationsBulkDeleted = (
      message: ServerMessage & { type: "conversations_bulk_deleted" }
    ) => {
      // Remove successfully deleted conversations from store
      message.conversation_ids.forEach(conversationId => {
        delete chatStore.map[conversationId];
      });
      
      chatStore.list = chatStore.list.filter(
        (id) => !message.conversation_ids.includes(id)
      );

      // If we're viewing one of the deleted conversations, switch to another one
      if (chatStore.active && message.conversation_ids.includes(chatStore.active)) {
        chatStore.active = chatStore.list[0] || "";
      }

      // Log any failed deletions
      if (message.failed_ids.length > 0) {
        console.warn("Failed to delete conversations:", message.failed_ids);
      }

      // Exit delete mode after bulk deletion is complete
      sidebarActions.exitDeleteMode();
    };

    // Add event listeners
    wsService.on("conversation_messages", handleConversationMessages); // Used for both initial load and explicit requests
    wsService.on("conversation_redirect", handleConversationRedirect);
    wsService.on("progress", handleProgress);
    wsService.on("start", handleStart);
    wsService.on("complete", handleComplete);
    wsService.on("error", handleError);
    
    // Tool event listeners
    wsService.on("tool_started", handleToolStarted);
    wsService.on("tool_completed", handleToolCompleted);

    // New conversation management listeners
    wsService.on("conversation_list", handleConversationList);
    wsService.on("conversation_created", handleConversationCreated);
    wsService.on("conversation_details", handleConversationDetails);
    wsService.on("conversation_updated", handleConversationUpdated);
    wsService.on("conversation_deleted", handleConversationDeleted);
    wsService.on("conversations_bulk_deleted", handleConversationsBulkDeleted);

    // Cleanup listeners
    return () => {
      wsService.off("conversation_messages", handleConversationMessages);
      wsService.off("conversation_redirect", handleConversationRedirect);
      wsService.off("progress", handleProgress);
      wsService.off("start", handleStart);
      wsService.off("complete", handleComplete);
      wsService.off("error", handleError);
      
      // Tool event cleanup
      wsService.off("tool_started", handleToolStarted);
      wsService.off("tool_completed", handleToolCompleted);

      // New conversation management cleanup
      wsService.off("conversation_list", handleConversationList);
      wsService.off("conversation_created", handleConversationCreated);
      wsService.off("conversation_details", handleConversationDetails);
      wsService.off("conversation_updated", handleConversationUpdated);
      wsService.off("conversation_deleted", handleConversationDeleted);
      wsService.off("conversations_bulk_deleted", handleConversationsBulkDeleted);
    };
  }, [snap.project_id]);

  // // Auto-subscribe when project/conversation changes
  // useEffect(() => {
  //   if (snap.project_id) {
  //     wsService.subscribe(snap.project_id, snap.active || undefined);
  //   }
  // }, [snap.project_id, snap.active]);

  const sendMessage = useCallback(
    (content: string, files?: string[]) => {
      if (!snap.project_id || !snap.active) return;

      // Add user message to store immediately
      const conversation = chatStore.map[snap.active];
      if (conversation) {
        // Ensure messages array exists
        if (!conversation.messages) {
          conversation.messages = [];
        }

        const userMessage: Message = {
          id: crypto.randomUUID(),
          content,
          role: "user",
          createdAt: new Date().toISOString(),
        };
        conversation.messages.push(userMessage);
      }

      // Send via WebSocket
      wsService.sendChatMessage(snap.project_id, snap.active, content, files);
    },
    [snap.project_id, snap.active]
  );

  const stopStreaming = useCallback(() => {
    if (snap.active) {
      wsService.stopStreaming(snap.active);
    }
  }, [snap.active]);

  // Conversation management methods
  const createConversation = useCallback(
    (title?: string) => {
      console.log(title, snap.project_id);
      if (snap.project_id) {
        wsService.createConversation(snap.project_id, title);
      }
    },
    [snap.project_id]
  );

  const listConversations = useCallback(() => {
    if (snap.project_id) {
      wsService.listConversations(snap.project_id);
    }
  }, [snap.project_id]);

  const getConversation = useCallback((conversationId: string) => {
    wsService.getConversation(conversationId);
  }, []);

  const updateConversation = useCallback(
    (conversationId: string, title?: string) => {
      wsService.updateConversation(conversationId, title);
    },
    []
  );

  const deleteConversation = useCallback((conversationId: string) => {
    wsService.deleteConversation(conversationId);
  }, []);

  const bulkDeleteConversations = useCallback((conversationIds: string[]) => {
    wsService.bulkDeleteConversations(conversationIds);
  }, []);

  const getConversationMessages = useCallback((conversationId: string) => {
    wsService.getConversationMessages(conversationId);
  }, []);

  // Helper to get current conversation messages
  const currentMessages =
    snap.active && snap.map[snap.active]
      ? snap.map[snap.active].messages || []
      : [];

  return {
    // Current state
    projectId: snap.project_id,
    conversationId: snap.active,
    conversationMap: snap.map,
    conversationList: snap.list,
    currentMessages,

    // Status
    isConnected: wsService.isConnected(),
    isStreaming: wsService.isStreaming(snap.active || ""),

    // Actions
    sendMessage,
    stopStreaming,

    // Conversation management
    createConversation,
    listConversations,
    getConversation,
    updateConversation,
    deleteConversation,
    bulkDeleteConversations,
    getConversationMessages,

    setProjectId(id: PROJECT_ID) {
      chatStore.project_id = id;
    },

    setConversationId(id: CONVERSATION_ID) {
      chatStore.active = id;
    },
  };
};
