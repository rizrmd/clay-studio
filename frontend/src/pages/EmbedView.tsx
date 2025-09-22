import { useEffect, useState } from "react";
import { useParams, useSearchParams } from "react-router-dom";
import { EmbeddableWidget } from "@/components/embed/EmbeddableWidget";
import { EmbeddableChat } from "@/components/embed/EmbeddableChat";
import { EmbeddableConversationList } from "@/components/embed/EmbeddableConversationList";

// This page handles embedded views via iframe or direct link
// URL format: /embed/:shareToken?type=chat|list|widget&theme=light|dark&readonly=true
export function EmbedView() {
  const { shareToken } = useParams<{ shareToken: string }>();
  const [searchParams] = useSearchParams();
  const [error, setError] = useState<string | null>(null);

  const type = searchParams.get("type") || "widget";
  const theme = (searchParams.get("theme") as "light" | "dark") || "light";
  const readOnly = searchParams.get("readonly") === "true";
  const layout = searchParams.get("layout") || "combined";

  useEffect(() => {
    // Apply theme to body for iframe embedding
    document.documentElement.classList.toggle("dark", theme === "dark");
    
    // Remove default margins for iframe embedding
    document.body.style.margin = "0";
    document.body.style.padding = "0";
    document.body.style.height = "100vh";
    document.body.style.overflow = "hidden";

    return () => {
      // Cleanup on unmount
      document.documentElement.classList.remove("dark");
    };
  }, [theme]);

  const handleMessage = (message: any) => {
    // Send message to parent window if embedded in iframe
    if (window.parent !== window) {
      window.parent.postMessage({
        type: "clay-studio-message",
        data: message,
      }, "*");
    }
  };

  const handleError = (error: string) => {
    setError(error);
    
    // Send error to parent window if embedded in iframe
    if (window.parent !== window) {
      window.parent.postMessage({
        type: "clay-studio-error",
        data: error,
      }, "*");
    }
  };

  if (!shareToken) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-center space-y-2">
          <h2 className="text-lg font-semibold text-red-600">Invalid Share Link</h2>
          <p className="text-sm text-muted-foreground">
            This embed link is missing required parameters.
          </p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-screen p-4">
        <div className="text-center space-y-2">
          <h2 className="text-lg font-semibold text-red-600">Error Loading Chat</h2>
          <p className="text-sm text-muted-foreground max-w-md">
            {error}
          </p>
          <button
            onClick={() => window.location.reload()}
            className="text-sm text-blue-600 hover:underline"
          >
            Try again
          </button>
        </div>
      </div>
    );
  }

  // Render the appropriate embed component based on type
  switch (type) {
    case "chat":
      return (
        <div className="h-screen">
          <EmbeddableChat
            shareToken={shareToken}
            theme={theme}
            readOnly={readOnly}
            onMessage={handleMessage}
            onError={handleError}
            className="h-full"
          />
        </div>
      );

    case "list":
      return (
        <div className="h-screen">
          <EmbeddableConversationList
            shareToken={shareToken}
            theme={theme}
            onConversationClick={(id) => {
              // Handle conversation clicks - could navigate or send to parent
              if (window.parent !== window) {
                window.parent.postMessage({
                  type: "clay-studio-conversation-click",
                  data: { conversationId: id },
                }, "*");
              }
            }}
            onError={handleError}
            showCreateNew={!readOnly}
            className="h-full"
          />
        </div>
      );

    case "widget":
    default:
      return (
        <div className="h-screen">
          <EmbeddableWidget
            shareToken={shareToken}
            theme={theme}
            layout={layout as any}
            onMessage={handleMessage}
            onError={handleError}
            className="h-full"
          />
        </div>
      );
  }
}

// Also provide a version that can be used outside of React Router
export function StandaloneEmbedView({
  shareToken,
  type = "widget",
  theme = "light",
  readOnly = false,
  layout = "combined",
}: {
  shareToken: string;
  type?: string;
  theme?: "light" | "dark";
  readOnly?: boolean;
  layout?: string;
}) {
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
    document.body.style.margin = "0";
    document.body.style.padding = "0";
    document.body.style.height = "100vh";
    document.body.style.overflow = "hidden";

    return () => {
      document.documentElement.classList.remove("dark");
    };
  }, [theme]);

  const handleMessage = (message: any) => {
    if (window.parent !== window) {
      window.parent.postMessage({
        type: "clay-studio-message",
        data: message,
      }, "*");
    }
  };

  const handleError = (error: string) => {
    setError(error);
    if (window.parent !== window) {
      window.parent.postMessage({
        type: "clay-studio-error",
        data: error,
      }, "*");
    }
  };

  if (error) {
    return (
      <div className="flex items-center justify-center h-screen p-4">
        <div className="text-center space-y-2">
          <h2 className="text-lg font-semibold text-red-600">Error Loading Chat</h2>
          <p className="text-sm text-muted-foreground max-w-md">{error}</p>
        </div>
      </div>
    );
  }

  switch (type) {
    case "chat":
      return (
        <EmbeddableChat
          shareToken={shareToken}
          theme={theme}
          readOnly={readOnly}
          onMessage={handleMessage}
          onError={handleError}
          className="h-screen"
        />
      );

    case "list":
      return (
        <EmbeddableConversationList
          shareToken={shareToken}
          theme={theme}
          onError={handleError}
          showCreateNew={!readOnly}
          className="h-screen"
        />
      );

    case "widget":
    default:
      return (
        <EmbeddableWidget
          shareToken={shareToken}
          theme={theme}
          layout={layout as any}
          onMessage={handleMessage}
          onError={handleError}
          className="h-screen"
        />
      );
  }
}