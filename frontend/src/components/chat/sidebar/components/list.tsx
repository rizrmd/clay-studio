import { useSnapshot } from "valtio";
import { sidebarStore } from "@/store/sidebar-store";
import { ConversationItem } from "./item";

interface Conversation {
  id: string;
  project_id: string;
  title: string;
  message_count: number;
  created_at: string;
  updated_at: string;
  is_title_manually_set?: boolean;
}

interface ConversationListProps {
  conversations: Conversation[];
  currentConversationId?: string;
  onConversationClick: (conversationId: string, e: React.MouseEvent) => void;
  onConversationHover: (conversationId: string) => void;
  onRenameConversation: (conversation: Conversation) => void;
  onDeleteConversation: (conversationId: string) => void;
}

export function ConversationList({
  conversations,
  currentConversationId,
  onConversationClick,
  onConversationHover,
  onRenameConversation,
  onDeleteConversation,
}: ConversationListProps) {
  const sidebarSnapshot = useSnapshot(sidebarStore);

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
    <div className="p-2 min-w-[130px] absolute inset-0">
      {conversations.map((conversation) => (
        <ConversationItem
          key={conversation.id}
          conversation={conversation}
          isActive={currentConversationId === conversation.id}
          onClick={onConversationClick}
          onHover={onConversationHover}
          onRename={onRenameConversation}
          onDelete={onDeleteConversation}
        />
      ))}
    </div>
  );
}