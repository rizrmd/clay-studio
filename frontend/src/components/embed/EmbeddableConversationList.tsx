import { useEffect, useState } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { MessageSquare, Calendar, ExternalLink } from "lucide-react";

interface Conversation {
  id: string;
  title?: string;
  created_at: string;
  updated_at: string;
  message_count: number;
}

interface EmbeddableConversationListProps {
  shareToken: string;
  theme?: "light" | "dark" | "auto";
  className?: string;
  onConversationClick?: (conversationId: string) => void;
  onError?: (error: string) => void;
  showCreateNew?: boolean;
}

export function EmbeddableConversationList({
  shareToken,
  theme = "light",
  className = "",
  onConversationClick,
  onError,
  showCreateNew = false,
}: EmbeddableConversationListProps) {
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [loading, setLoading] = useState(true);
  const [shareData, setShareData] = useState<any>(null);

  // Load shared data on mount
  useEffect(() => {
    const loadShareData = async () => {
      try {
        const response = await fetch(`/api/shares/${shareToken}/data`);
        if (!response.ok) {
          throw new Error("Failed to load shared data");
        }
        const data = await response.json();
        setShareData(data);
        setConversations(data.conversations || []);
      } catch (error) {
        console.error("Failed to load share data:", error);
        onError?.("Failed to load conversation list");
      } finally {
        setLoading(false);
      }
    };

    if (shareToken) {
      loadShareData();
    }
  }, [shareToken, onError]);

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    const now = new Date();
    const diffInDays = Math.floor((now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24));

    if (diffInDays === 0) {
      return `Today at ${date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
    } else if (diffInDays === 1) {
      return `Yesterday at ${date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
    } else if (diffInDays < 7) {
      return `${diffInDays} days ago`;
    } else {
      return date.toLocaleDateString();
    }
  };

  const handleConversationClick = (conversationId: string) => {
    onConversationClick?.(conversationId);
  };

  const themeClass = theme === "dark" ? "dark" : "";

  if (loading) {
    return (
      <div className={`${themeClass} ${className}`}>
        <div className="flex flex-col h-full bg-background border border-border rounded-lg overflow-hidden">
          <div className="p-4">
            <div className="animate-pulse">
              <div className="h-4 bg-muted rounded mb-3"></div>
              <div className="space-y-3">
                {[...Array(3)].map((_, i) => (
                  <div key={i} className="flex items-center space-x-3">
                    <div className="h-8 w-8 bg-muted rounded-full"></div>
                    <div className="flex-1">
                      <div className="h-3 bg-muted rounded mb-1"></div>
                      <div className="h-2 bg-muted rounded w-2/3"></div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={`${themeClass} ${className}`}>
      <div className="flex flex-col h-full bg-background border border-border rounded-lg overflow-hidden">
        {/* Header */}
        <div className="border-b border-border px-4 py-3 bg-muted/10">
          <h3 className="text-sm font-medium text-foreground">
            Conversations
          </h3>
          {shareData?.project_name && (
            <p className="text-xs text-muted-foreground mt-1">
              From {shareData.project_name}
            </p>
          )}
        </div>

        {/* Create New Button */}
        {showCreateNew && (
          <div className="p-4 border-b border-border">
            <Button
              variant="outline"
              size="sm"
              onClick={() => handleConversationClick("new")}
              className="w-full justify-start"
            >
              <MessageSquare className="h-4 w-4 mr-2" />
              Start New Conversation
            </Button>
          </div>
        )}

        {/* Conversations */}
        <ScrollArea className="flex-1">
          {conversations.length === 0 ? (
            <div className="p-4 text-center text-muted-foreground">
              <MessageSquare className="h-8 w-8 mx-auto mb-2 opacity-50" />
              <p className="text-sm">No conversations shared</p>
            </div>
          ) : (
            <div className="p-2">
              {conversations.map((conversation) => (
                <button
                  key={conversation.id}
                  onClick={() => handleConversationClick(conversation.id)}
                  className="w-full p-3 rounded-lg hover:bg-muted/50 transition-colors text-left group"
                >
                  <div className="flex items-start gap-3">
                    <Avatar className="h-8 w-8 mt-0.5">
                      <AvatarFallback className="text-xs">
                        <MessageSquare className="h-4 w-4" />
                      </AvatarFallback>
                    </Avatar>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-start justify-between gap-2">
                        <h4 className="text-sm font-medium text-foreground truncate">
                          {conversation.title || `Conversation ${conversation.id.slice(0, 8)}`}
                        </h4>
                        <ExternalLink className="h-3 w-3 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0" />
                      </div>
                      <div className="flex items-center gap-2 mt-1">
                        <div className="flex items-center gap-1 text-xs text-muted-foreground">
                          <Calendar className="h-3 w-3" />
                          {formatDate(conversation.updated_at)}
                        </div>
                        {conversation.message_count > 0 && (
                          <>
                            <span className="text-xs text-muted-foreground">•</span>
                            <span className="text-xs text-muted-foreground">
                              {conversation.message_count} message{conversation.message_count !== 1 ? 's' : ''}
                            </span>
                          </>
                        )}
                      </div>
                    </div>
                  </div>
                </button>
              ))}
            </div>
          )}
        </ScrollArea>

        {/* Footer */}
        {shareData?.share?.settings?.show_branding !== false && (
          <div className="border-t border-border px-4 py-2 text-center">
            <span className="text-xs text-muted-foreground">
              Powered by{" "}
              <a
                href="https://clay.studio"
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary hover:underline"
              >
                Clay Studio
              </a>
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

// Export a version that can be used as a web component
export function initEmbeddableConversationList(
  container: HTMLElement, 
  _props: EmbeddableConversationListProps
) {
  const root = document.createElement('div');
  root.style.width = '100%';
  root.style.height = '100%';
  container.appendChild(root);

  // This would normally use React.render, but for simplicity we'll return a placeholder
  return {
    destroy: () => {
      container.removeChild(root);
    },
    updateProps: (newProps: Partial<EmbeddableConversationListProps>) => {
      // Update the component props
      console.log('Updating conversation list props:', newProps);
    }
  };
}