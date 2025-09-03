import { useSnapshot } from "valtio";
import { sidebarStore, sidebarActions } from "@/store/sidebar-store";
import { api } from "@/lib/utils/api";

interface Conversation {
  id: string;
  project_id: string;
  title: string;
  message_count: number;
  created_at: string;
  updated_at: string;
  is_title_manually_set?: boolean;
}

export function useRenameDialog() {
  const sidebarSnapshot = useSnapshot(sidebarStore);

  const handleRenameConversation = async () => {
    if (!sidebarSnapshot.renamingConversation || !sidebarSnapshot.newTitle.trim()) return;

    try {
      const response = await api.fetchStream(
        `/conversations/${sidebarSnapshot.renamingConversation.id}`,
        {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            title: sidebarSnapshot.newTitle.trim(),
          }),
        }
      );

      if (!response.ok) {
        throw new Error("Failed to rename conversation");
      }

      const updatedConversation = await response.json();

      // Update local state
      const updatedConversations = sidebarSnapshot.conversations.map((c) =>
        c.id === sidebarSnapshot.renamingConversation!.id ? updatedConversation : c
      );
      sidebarActions.setConversations(updatedConversations);

      // Close dialog and reset state
      sidebarActions.closeRenameDialog();
    } catch (err) {
      sidebarActions.setError("Failed to rename conversation");
    }
  };

  const openRenameDialog = (conversation: Conversation) => {
    sidebarActions.openRenameDialog(conversation);
  };

  return {
    handleRenameConversation,
    openRenameDialog,
  };
}