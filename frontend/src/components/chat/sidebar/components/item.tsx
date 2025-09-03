import { MessageSquare, Loader2, MoreHorizontal, Edit, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useSnapshot } from "valtio";
import { sidebarStore, sidebarActions } from "@/store/sidebar-store";
import { conversationStore } from "@/store/chat/conversation-store";
import { cn } from "@/lib/utils";

interface Conversation {
  id: string;
  project_id: string;
  title: string;
  message_count: number;
  created_at: string;
  updated_at: string;
  is_title_manually_set?: boolean;
}

interface ConversationItemProps {
  conversation: Conversation;
  isActive: boolean;
  onClick: (conversationId: string, e: React.MouseEvent) => void;
  onHover: (conversationId: string) => void;
  onRename: (conversation: Conversation) => void;
  onDelete: (conversationId: string) => void;
}

export function ConversationItem({
  conversation,
  isActive,
  onClick,
  onHover,
  onRename,
  onDelete,
}: ConversationItemProps) {
  const sidebarSnapshot = useSnapshot(sidebarStore);
  const conversationStoreSnapshot = useSnapshot(conversationStore);

  return (
    <div
      key={conversation.id}
      className={cn(
        "block w-full group text-left p-2 rounded-md hover:bg-muted border border-transparent transition-colors mb-1 group cursor-pointer relative",
        isActive &&
          "bg-muted border-blue-700/30",
        sidebarSnapshot.isDeleteMode &&
          sidebarSnapshot.selectedConversations.has(conversation.id) &&
          "bg-red-50 dark:bg-red-900/20 border-red-500/30",
        sidebarSnapshot.isDeleteMode && "hover:bg-red-50 dark:hover:bg-red-900/10"
      )}
      onClick={(e) => onClick(conversation.id, e)}
      onMouseEnter={() => !sidebarSnapshot.isDeleteMode && onHover(conversation.id)}
    >
      <div
        className={cn(
          "flex items-start gap-2 overflow-hidden",
          !sidebarSnapshot.isDeleteMode && "group-hover:pr-8"
        )}
      >
        {/* Icon section - always on left */}
        <div className="relative flex flex-col items-center pt-1">
          {/* Check streaming state from conversationStore */}
          {conversationStoreSnapshot.conversations[conversation.id]?.status === 'streaming' ? (
            <Loader2 className="h-4 w-4 text-blue-500 animate-spin" />
          ) : conversationStoreSnapshot.conversations[conversation.id]?.status === 'loading' ? (
            <Loader2 className="h-4 w-4 text-muted-foreground animate-spin" />
          ) : (
            <MessageSquare
              className={cn(
                "h-4 w-4",
                sidebarSnapshot.recentlyUpdatedConversations.has(conversation.id) &&
                !isActive
                  ? "text-green-500"
                  : "text-muted-foreground"
              )}
            />
          )}
          {/* Green notification dot for new messages in non-active conversations */}
          {sidebarSnapshot.recentlyUpdatedConversations.has(conversation.id) &&
           !isActive && (
            <div className="h-[6px] w-[6px] rounded-full bg-green-500 mt-1 animate-pulse" />
          )}
        </div>

        {/* Content section - flexible width */}
        <div className="flex-1 min-w-0">
          <p
            className={cn(
              "text-sm font-medium truncate",
              sidebarSnapshot.recentlyUpdatedConversations.has(conversation.id) &&
              !isActive &&
                "text-green-500 font-semibold"
            )}
          >
            {conversation.title || "New Conversation"}
          </p>
          <p className="text-xs text-muted-foreground">
            {conversation.message_count} chat
            {conversation.message_count !== 1 ? "s" : ""} •{" "}
            {new Date(
              conversation.updated_at
            ).toLocaleDateString()}{" "}
            {new Date(conversation.updated_at).toLocaleTimeString(
              [],
              {
                hour: "2-digit",
                minute: "2-digit",
                hour12: false,
              }
            )}
          </p>
        </div>

        {/* Checkbox on the right in delete mode */}
        {sidebarSnapshot.isDeleteMode && (
          <div className="pt-1 flex-shrink-0">
            <Checkbox
              checked={sidebarSnapshot.selectedConversations.has(conversation.id)}
              onCheckedChange={() => sidebarActions.toggleConversationSelection(conversation.id)}
              onClick={(e: React.MouseEvent) => e.stopPropagation()}
            />
          </div>
        )}
      </div>

      {/* Actions dropdown - hidden in delete mode */}
      {!sidebarSnapshot.isDeleteMode && (
        <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 w-6 p-0"
                onClick={(e) => e.stopPropagation()}
              >
                <MoreHorizontal className="h-3 w-3" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                onClick={(e) => {
                  e.stopPropagation();
                  onRename(conversation);
                }}
              >
                <Edit className="h-4 w-4 mr-2" />
                Rename
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={(e) => {
                  e.stopPropagation();
                  onDelete(conversation.id);
                }}
                className="text-red-600 focus:text-red-600"
              >
                <Trash2 className="h-4 w-4 mr-2" />
                Delete
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      )}
    </div>
  );
}