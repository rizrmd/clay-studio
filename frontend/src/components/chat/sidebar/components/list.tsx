import { useSnapshot } from "valtio";
// import { sidebarStore } from "@/store/sidebar-store";
import { useChat } from "@/lib/hooks/use-chat";
import { sidebarStore } from "@/lib/store/chat/sidebar-store";
import { Conversation } from "../types";
import { ConversationItem } from "./item";

interface ConversationListProps {
  currentConversationId?: string;
  onConversationClick: (conversationId: string) => void;
  onRenameConversation: (conversation: Conversation) => void;
  onDeleteConversation: (conversationId: string) => void;
  onShareConversation: (conversation: Conversation) => void;
  projectId?: string;
}

export function ConversationList({
  currentConversationId,
  onConversationClick,
  onRenameConversation,
  onDeleteConversation,
  onShareConversation,
  projectId,
}: ConversationListProps) {
  const sidebarSnapshot = useSnapshot(sidebarStore);
  const chat = useChat();

  // Convert conversation list to array of conversation objects
  const conversations = chat.conversationList
    .map((id) => ({
      ...chat.conversationMap[id],
      title: chat.conversationMap[id]?.title || `Conversation ${id}`,
    }))
    .filter(Boolean);

  if (sidebarSnapshot.loading) {
    return (
      <div className="p-4">
        <div className="animate-pulse">
          <div className="h-4 bg-gray-200 rounded mb-2"></div>
          <div className="h-4 bg-gray-200 rounded mb-2"></div>
          <div className="h-4 bg-gray-200 rounded"></div>
        </div>
      </div>
    );
  }

  if (sidebarSnapshot.error) {
    return (
      <div className="p-4">
        <p className="text-sm text-red-500">{sidebarSnapshot.error}</p>
      </div>
    );
  }

  if (conversations.length === 0) {
    return (
      <div className="p-4">
        <p className="text-xs text-muted-foreground">Ummm... </p>
        <p className="text-xs text-muted-foreground">Let's talk ? </p>
      </div>
    );
  }

  return (
    <div className="overflow-y-auto flex relative flex-1">
      <div className="absolute inset-0 px-2">
        {conversations.map((conversation) => (
          <ConversationItem
            key={conversation.id}
            href={`/p/${chat.projectId}/c/${conversation.id}`}
            conversation={conversation}
            isActive={currentConversationId === conversation.id}
            onClick={onConversationClick}
            onRename={onRenameConversation}
            onDelete={onDeleteConversation}
            onShare={onShareConversation}
            projectId={projectId}
          />
        ))}
      </div>
    </div>
  );
}
