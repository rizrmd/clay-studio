import { useSnapshot } from "valtio";
import { sidebarStore, sidebarActions } from "@/store/sidebar-store";
import { conversationStore } from "@/store/chat/conversation-store";
import { ConversationManager } from "@/store/chat/conversation-manager";
import { api } from "@/lib/utils/api";
import { logger } from "@/lib/utils/logger";

export function useDeleteMode(projectId?: string, actualConversationId?: string, navigate?: (path: string) => void) {
  const sidebarSnapshot = useSnapshot(sidebarStore);
  const conversationStoreSnapshot = useSnapshot(conversationStore);

  const handleBulkDelete = async () => {
    const selectedCount = sidebarSnapshot.selectedConversations.size;
    if (selectedCount === 0) {
      alert("No conversations selected");
      return;
    }

    if (!confirm(`Are you sure you want to delete ${selectedCount} conversation${selectedCount > 1 ? 's' : ''}?`)) {
      return;
    }

    // Delete each selected conversation
    const deletePromises = Array.from(sidebarSnapshot.selectedConversations).map(async (conversationId) => {
      try {
        const response = await api.fetchStream(`/conversations/${conversationId}`, {
          method: "DELETE",
        });
        if (!response.ok) {
          throw new Error("Failed to delete conversation");
        }
        // Remove from store
        sidebarActions.removeConversation(conversationId);
        // Clean up from conversation store
        if (conversationStoreSnapshot.conversations[conversationId]) {
          ConversationManager.getInstance().clearConversation(conversationId);
        }
        return { success: true, id: conversationId };
      } catch (error) {
        logger.error(`Failed to delete conversation ${conversationId}:`, error);
        return { success: false, id: conversationId };
      }
    });

    const results = await Promise.all(deletePromises);
    const failedCount = results.filter(r => !r.success).length;

    if (failedCount > 0) {
      sidebarActions.setError(`Failed to delete ${failedCount} conversation${failedCount > 1 ? 's' : ''}`);
    }

    const wasCurrentConversationDeleted = actualConversationId && sidebarSnapshot.selectedConversations.has(actualConversationId);

    // Exit delete mode
    sidebarActions.exitDeleteMode();

    // If the current conversation was deleted, navigate to a new one
    if (wasCurrentConversationDeleted && navigate && projectId) {
      const latestConversation = sidebarSnapshot.conversations[0];
      if (latestConversation) {
        navigate(`/p/${projectId}/c/${latestConversation.id}`);
      } else {
        navigate(`/p/${projectId}/new`);
      }
    }
  };

  return {
    handleBulkDelete,
  };
}