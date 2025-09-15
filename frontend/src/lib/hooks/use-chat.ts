import { useCallback, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useSnapshot } from "valtio";
import { wsService } from "../services/ws-service";
import { chatStore } from "../store/chat/chat-store";
import { sidebarActions } from "../store/chat/sidebar-store";
import type { CONVERSATION_ID, Message, PROJECT_ID } from "../types/chat";
import type { ServerMessage } from "../types/ws";
import { stream } from "./chat-streaming";
import { messageUIActions } from "../store/chat/message-ui-store";
import { tabsActions, tabsStore } from "../store/tabs-store";

export const useChat = () => {
  const snap = useSnapshot(chatStore);
  const navigate = useNavigate();

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
          messages: message.messages || [],
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
      if (snap.conversation_id === message.old_conversation_id) {
        chatStore.conversation_id = message.new_conversation_id;
      }
    };

    // Handle errors
    const handleError = (message: ServerMessage & { type: "error" }) => {
      console.error("Chat error:", message.error);
      // Could add error state to store if needed
    };

    // Handle new conversation management responses
    const handleConversationList = (
      message: ServerMessage & { type: "conversation_list" }
    ) => {
      // Clear existing conversations and update with new ones
      chatStore.map = {};
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
        chatStore.list.unshift(message.conversation.id); // Add to beginning instead of end
      }
      // Switch to the new conversation
      chatStore.conversation_id = message.conversation.id;
      messageUIActions.setPreviousConversationId("new");

      // Remove all chat tabs with conversationId="new" since we now have a real conversation
      const newChatTabs = tabsStore.tabs.filter(t => 
        t.type === 'chat' && t.metadata.conversationId === 'new'
      );
      newChatTabs.forEach(tab => tabsActions.removeTab(tab.id));
      
      // Create a proper chat tab for the new conversation
      tabsActions.addTab({
        type: "chat",
        title: message.conversation.title || "New Chat",
        metadata: {
          conversationId: message.conversation.id,
          projectId: chatStore.project_id,
          conversationTitle: message.conversation.title,
        },
      });

      if (chatStore.pendingFirstChat) {
        sendMessage(chatStore.pendingFirstChat);
        chatStore.pendingFirstChat = "";
      }

      navigate(`/p/${chatStore.project_id}/c/${message.conversation.id}`);
    };

    const handleConversationDetails = (
      message: ServerMessage & { type: "conversation_details" }
    ) => {
      // Update single conversation
      chatStore.map[message.conversation.id] = message.conversation;
      if (!chatStore.list.includes(message.conversation.id)) {
        chatStore.list.unshift(message.conversation.id); // Add to beginning
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

      // If we're viewing the deleted conversation, switch to another one or navigate to /new
      if (chatStore.conversation_id === message.conversation_id) {
        if (chatStore.list.length > 0) {
          chatStore.conversation_id = chatStore.list[0];
          // Navigate to the first available conversation
          navigate(`/p/${chatStore.project_id}/c/${chatStore.list[0]}`);
        } else {
          chatStore.conversation_id = "";
          // Navigate to new conversation when no conversations exist
          navigate(`/p/${chatStore.project_id}/new`);
        }
      }
    };

    const handleConversationsBulkDeleted = (
      message: ServerMessage & { type: "conversations_bulk_deleted" }
    ) => {
      // Remove successfully deleted conversations from store
      message.conversation_ids.forEach((conversationId) => {
        delete chatStore.map[conversationId];
      });

      chatStore.list = chatStore.list.filter(
        (id) => !message.conversation_ids.includes(id)
      );

      // If we're viewing one of the deleted conversations, switch to another one or navigate to /new
      if (
        chatStore.conversation_id &&
        message.conversation_ids.includes(chatStore.conversation_id)
      ) {
        if (chatStore.list.length > 0) {
          chatStore.conversation_id = chatStore.list[0];
          // Navigate to the first available conversation
          navigate(`/p/${chatStore.project_id}/c/${chatStore.list[0]}`);
        } else {
          chatStore.conversation_id = "";
          // Navigate to new conversation when no conversations exist
          navigate(`/p/${chatStore.project_id}/new`);
        }
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
    wsService.on("progress", stream.progress);
    wsService.on("start", stream.start);
    wsService.on("content", stream.content);
    wsService.on("complete", stream.complete);
    wsService.on("error", handleError);
    wsService.on("subscribed", (msg: { conversation_id: string }) => {
      chatStore.conversation_id = msg.conversation_id;
    });

    // Tool event listeners
    const handleToolStarted = (message: {
      tool: string;
      toolUsageId: string;
      conversationId: string;
    }) => {

      if (!chatStore.streaming[message.conversationId]) {
        chatStore.streaming[message.conversationId] = {
          messageId: "",
          partialContent: "",
          activeTools: [],
          isComplete: false,
        };
      }

      const activeTools =
        chatStore.streaming[message.conversationId].activeTools;
      if (!activeTools.find((t) => t.toolUsageId === message.toolUsageId)) {
        activeTools.push({
          tool: message.tool,
          toolUsageId: message.toolUsageId,
          startTime: Date.now(),
          status: "active",
        });
      } else {
      }

    };

    const handleToolCompleted = (message: {
      tool: string;
      toolUsageId: string;
      executionTimeMs?: number;
      conversationId: string;
    }) => {
      if (chatStore.streaming[message.conversationId]) {
        const activeTools =
          chatStore.streaming[message.conversationId].activeTools;
        const toolIndex = activeTools.findIndex(
          (t) => t.toolUsageId === message.toolUsageId
        );
        if (toolIndex !== -1) {
          // Mark tool as completed instead of removing it
          activeTools[toolIndex].status = "completed";
          activeTools[toolIndex].completedAt = Date.now();
          if (message.executionTimeMs) {
            activeTools[toolIndex].executionTime = message.executionTimeMs;
          }
        }
      }
    };

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
      wsService.off("progress", stream.progress);
      wsService.off("start", stream.start);
      wsService.off("content", stream.content);
      wsService.off("complete", stream.complete);
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
      wsService.off(
        "conversations_bulk_deleted",
        handleConversationsBulkDeleted
      );
    };
  }, [snap.project_id, snap.conversation_id]);

  // // Auto-subscribe when project/conversation changes
  // useEffect(() => {
  //   if (snap.project_id) {
  //     wsService.subscribe(snap.project_id, snap.active || undefined);
  //   }
  // }, [snap.project_id, snap.active]);

  const sendMessage = useCallback(
    (content: string, files?: string[]) => {
      if (!snap.project_id || !snap.conversation_id) return;

      // Add user message to store immediately
      const conversation = chatStore.map[snap.conversation_id];
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
      wsService.sendChatMessage(
        snap.project_id,
        snap.conversation_id,
        content,
        files
      );
    },
    [snap.project_id, snap.conversation_id]
  );

  const stopStreaming = useCallback(() => {
    if (snap.conversation_id) {
      wsService.stopStreaming(snap.conversation_id);
    }
  }, [snap.conversation_id]);

  // Conversation management methods
  const createConversation = useCallback(
    (message: string) => {
      if (snap.project_id) {
        chatStore.pendingFirstChat = message;
        const conversationTitle =
          message.slice(0, 50).trim() + (message.length > 50 ? "..." : "");
        wsService.createConversation(snap.project_id, conversationTitle);
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
    snap.conversation_id && snap.map[snap.conversation_id]
      ? snap.map[snap.conversation_id].messages || []
      : [];

  // Debug logging for conversation changes
  useEffect(() => {
  }, [snap.conversation_id, snap.map, currentMessages.length]);

  // Helper to get active tools for current conversation
  const currentActiveTools =
    snap.conversation_id && snap.streaming[snap.conversation_id]
      ? snap.streaming[snap.conversation_id].activeTools || []
      : [];

  return {
    // Current state
    projectId: snap.project_id,
    conversationId: snap.conversation_id,
    conversationMap: snap.map,
    conversationList: snap.list,
    currentMessages,
    currentActiveTools,

    // Status
    isConnected: wsService.isConnected(),
    isStreaming: wsService.isStreaming(snap.conversation_id || ""),

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
      chatStore.conversation_id = id;
    },
  };
};
