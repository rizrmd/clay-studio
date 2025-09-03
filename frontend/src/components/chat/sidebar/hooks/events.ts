import { useEffect } from "react";
import { useSnapshot } from "valtio";
import { sidebarStore, sidebarActions } from "@/store/sidebar-store";
import { chatEventBus } from "@/lib/services/chat/event-bus";
import { useAuth } from "@/hooks/use-auth";

export function useConversationEvents(projectId?: string) {
  const sidebarSnapshot = useSnapshot(sidebarStore);
  const { isAuthenticated, isSetupComplete } = useAuth();

  // Set up event listeners for real-time sidebar updates
  useEffect(() => {
    if (!projectId) return;

    // Listen for conversation creation events to refresh the list
    const handleConversationCreated = (event: CustomEvent) => {
      if (event.detail?.projectId === projectId) {
        // Just fetch the real data from server - don't add placeholder
        // This ensures we get the correct count and avoid race conditions
        // fetchConversations will be called from the parent component
      }
    };

    // Listen for message sent events to update message counts
    const handleMessageSent = (event: CustomEvent) => {
      if (
        event.detail?.projectId === projectId &&
        event.detail?.conversationId
      ) {
        // Add to recently updated set
        sidebarActions.addRecentlyUpdated(event.detail.conversationId);

        // Update the message count for the specific conversation
        const conversations = sidebarSnapshot.conversations.map((conv) => {
          if (conv.id === event.detail.conversationId) {
            return {
              ...conv,
              message_count: conv.message_count + 2, // +1 for user message, +1 for assistant response
              updated_at: new Date().toISOString(),
            };
          }
          return conv;
        })
          .sort(
            (a, b) =>
              new Date(b.created_at).getTime() -
              new Date(a.created_at).getTime()
          );
        sidebarActions.setConversations(conversations);
      }
    };

    // Listen for streaming events (no need to update state, valtio handles it)
    const handleStreamingStarted = (event: CustomEvent) => {
      if (event.detail?.projectId === projectId) {
        // Streaming started
      }
    };

    const handleStreamingStopped = (event: CustomEvent) => {
      if (event.detail?.projectId === projectId) {
        // Streaming stopped
      }
    };

    window.addEventListener(
      "conversation-created",
      handleConversationCreated as EventListener
    );
    window.addEventListener("message-sent", handleMessageSent as EventListener);
    window.addEventListener(
      "streaming-started",
      handleStreamingStarted as EventListener
    );
    window.addEventListener(
      "streaming-stopped",
      handleStreamingStopped as EventListener
    );

    return () => {
      window.removeEventListener(
        "conversation-created",
        handleConversationCreated as EventListener
      );
      window.removeEventListener(
        "message-sent",
        handleMessageSent as EventListener
      );
      window.removeEventListener(
        "streaming-started",
        handleStreamingStarted as EventListener
      );
      window.removeEventListener(
        "streaming-stopped",
        handleStreamingStopped as EventListener
      );
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, isAuthenticated, isSetupComplete]);

  // Subscribe to title update events from chatEventBus
  useEffect(() => {
    const unsubscribe = chatEventBus.subscribe(
      'CONVERSATION_TITLE_UPDATED',
      async (event: any) => {
        if (event.type === 'CONVERSATION_TITLE_UPDATED') {

          // Update the conversation in the local state
          sidebarActions.updateConversation(event.conversationId, { title: event.title });

          // Add to recently updated set for visual feedback
          sidebarActions.addRecentlyUpdated(event.conversationId);
        }
      }
    );

    return () => {
      unsubscribe();
    };
  }, []);
}