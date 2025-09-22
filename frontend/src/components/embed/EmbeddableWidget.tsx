import { useState } from "react";
import { EmbeddableChat } from "./EmbeddableChat";
import { EmbeddableConversationList } from "./EmbeddableConversationList";
import { Button } from "@/components/ui/button";
import { MessageSquare, List, X } from "lucide-react";

interface EmbeddableWidgetProps {
  shareToken: string;
  theme?: "light" | "dark" | "auto";
  layout?: "combined" | "chat-only" | "list-only";
  className?: string;
  onMessage?: (message: any) => void;
  onError?: (error: string) => void;
}

export function EmbeddableWidget({
  shareToken,
  theme = "light",
  layout = "combined",
  className = "",
  onMessage,
  onError,
}: EmbeddableWidgetProps) {
  const [activeView, setActiveView] = useState<"chat" | "list">("chat");
  const [selectedConversationId, setSelectedConversationId] = useState<string | null>(null);

  const handleConversationClick = (conversationId: string) => {
    setSelectedConversationId(conversationId);
    setActiveView("chat");
  };

  const themeClass = theme === "dark" ? "dark" : "";

  // Chat only layout
  if (layout === "chat-only") {
    return (
      <div className={`${themeClass} ${className} h-full`}>
        <EmbeddableChat
          shareToken={shareToken}
          theme={theme}
          onMessage={onMessage}
          onError={onError}
          className="h-full"
        />
      </div>
    );
  }

  // List only layout
  if (layout === "list-only") {
    return (
      <div className={`${themeClass} ${className} h-full`}>
        <EmbeddableConversationList
          shareToken={shareToken}
          theme={theme}
          onConversationClick={handleConversationClick}
          onError={onError}
          showCreateNew={true}
          className="h-full"
        />
      </div>
    );
  }

  // Combined layout
  return (
    <div className={`${themeClass} ${className} h-full`}>
      <div className="flex flex-col h-full bg-background border border-border rounded-lg overflow-hidden">
        {/* Mobile view toggle */}
        <div className="flex md:hidden border-b border-border">
          <Button
            variant={activeView === "list" ? "default" : "ghost"}
            onClick={() => setActiveView("list")}
            className="flex-1 rounded-none border-r border-border"
            size="sm"
          >
            <List className="h-4 w-4 mr-2" />
            Conversations
          </Button>
          <Button
            variant={activeView === "chat" ? "default" : "ghost"}
            onClick={() => setActiveView("chat")}
            className="flex-1 rounded-none"
            size="sm"
          >
            <MessageSquare className="h-4 w-4 mr-2" />
            Chat
          </Button>
        </div>

        <div className="flex flex-1 min-h-0">
          {/* Conversation List - Desktop: sidebar, Mobile: conditional */}
          <div className={`
            ${activeView === "list" ? "flex" : "hidden"} md:flex
            flex-col w-full md:w-80 md:border-r border-border
          `}>
            <EmbeddableConversationList
              shareToken={shareToken}
              theme={theme}
              onConversationClick={handleConversationClick}
              onError={onError}
              showCreateNew={true}
              className="h-full border-none"
            />
          </div>

          {/* Chat - Desktop: main area, Mobile: conditional */}
          <div className={`
            ${activeView === "chat" ? "flex" : "hidden"} md:flex
            flex-col flex-1 min-w-0
          `}>
            {/* Mobile: Back to list button */}
            <div className="flex md:hidden items-center gap-2 p-3 border-b border-border bg-muted/10">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setActiveView("list")}
              >
                <X className="h-4 w-4 mr-1" />
                Back to List
              </Button>
              {selectedConversationId && (
                <span className="text-sm text-muted-foreground">
                  Conversation {selectedConversationId.slice(0, 8)}
                </span>
              )}
            </div>

            <div className="flex-1">
              <EmbeddableChat
                shareToken={shareToken}
                theme={theme}
                onMessage={onMessage}
                onError={onError}
                className="h-full border-none"
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// Export a version that can be used as a web component
export function initEmbeddableWidget(
  container: HTMLElement, 
  _props: EmbeddableWidgetProps
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
    updateProps: (newProps: Partial<EmbeddableWidgetProps>) => {
      // Update the component props
      console.log('Updating widget props:', newProps);
    }
  };
}