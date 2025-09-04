import { lazy, Suspense, useEffect } from "react";
// Lazy load major components for better code splitting
const Chat = lazy(() =>
  import("@/components/chat").then((m) => ({ default: m.Chat }))
);
const ConversationSidebar = lazy(() =>
  import("@/components/chat").then((m) => ({ default: m.ConversationSidebar }))
);
const NewChat = lazy(() =>
  import("@/components/chat/main/new-chat").then((m) => ({
    default: m.NewChat,
  }))
);
import { useParams, useNavigate, useLocation } from "react-router-dom";
import { useSnapshot } from "valtio";
import { uiStore, uiActions } from "@/lib/store/chat/ui-store";
import { api } from "@/lib/utils/api";
import { useChat } from "@/lib/hooks/use-chat";
import { wsService } from "@/lib/services/ws-service";

// Stub implementations
const useLoggerDebug = () => ({ isDebugMode: false });

export function MainApp() {
  const uiSnapshot = useSnapshot(uiStore);
  const { projectId, conversationId } = useParams<{
    projectId: string;
    conversationId?: string;
  }>();
  const navigate = useNavigate();
  const location = useLocation();
  const chat = useChat();

  // Check if we're on the new conversation route
  const isNewRoute = location.pathname.endsWith("/new");

  // Enable debug logging hooks
  useLoggerDebug();

  // Update valtio store with current route params
  useEffect(() => {
    if (projectId) {
      uiActions.setCurrentProject(projectId);
    }
    if (conversationId) {
      uiActions.setCurrentConversation(conversationId);
      // Also update the chat store's active conversation
      chat.setConversationId(conversationId);
    }
  }, [projectId, conversationId, location.state]);

  // Handle redirection when visiting /p/:projectId without conversation ID
  // Don't redirect if we're on the /new route
  useEffect(() => {
    if (projectId && !conversationId && !isNewRoute) {
      // Try to get the last conversation from localStorage
      const lastConversationKey = `last_conversation_${projectId}`;
      const lastConversationId = localStorage.getItem(lastConversationKey);

      if (lastConversationId) {
        // Redirect to the last conversation
        navigate(`/p/${projectId}/c/${lastConversationId}`, { replace: true });
      } else {
        // No last conversation, create a new one immediately
        createNewConversationAndRedirect();
      }
    }
  }, [projectId, conversationId, navigate]);

  useEffect(() => {
    if (projectId && projectId !== chat.projectId) {
      chat.setProjectId(projectId);
      wsService.listConversations(projectId);
    }
  }, [projectId]);

  // Auto-subscribe to conversation when project and conversation IDs are available
  useEffect(() => {
    if (projectId && conversationId && conversationId !== "new") {
      // Only subscribe if we're not already subscribed to this project/conversation
      if (!wsService.isSubscribed(projectId, conversationId)) {
        wsService.subscribe(projectId, conversationId);
      }
    }
  }, [projectId, conversationId]);

  // Create new conversation immediately instead of using 'new' pseudo-state
  const createNewConversationAndRedirect = async () => {
    if (!projectId) return;

    try {
      const response = await api.fetchStream("/conversations", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          project_id: projectId,
        }),
      });

      if (!response.ok) {
        throw new Error("Failed to create new conversation");
      }

      const newConversation = await response.json();
      navigate(`/p/${projectId}/c/${newConversation.id}`, { replace: true });
    } catch (error) {
      console.error("Failed to create new conversation:", error);
      // Fallback to 'new' pseudo-conversation
      navigate(`/p/${projectId}/new`, { replace: true });
    }
  };

  // Handle 'new' conversation ID - don't save it to localStorage
  const effectiveConversationId = isNewRoute ? undefined : conversationId;

  // Save current conversation ID to localStorage when it changes
  useEffect(() => {
    if (projectId && conversationId && conversationId !== "new") {
      const lastConversationKey = `last_conversation_${projectId}`;
      localStorage.setItem(lastConversationKey, conversationId);
    }
  }, [projectId, conversationId]);

  // Handle responsive behavior
  useEffect(() => {
    const handleResize = () => {
      const mobile = window.innerWidth < 768;
      uiActions.setMobile(mobile);
      // Auto-collapse sidebar on mobile
      if (mobile && !uiSnapshot.isSidebarCollapsed) {
        uiActions.setSidebarCollapsed(true);
      }
    };

    handleResize(); // Check initial size
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [uiSnapshot.isSidebarCollapsed]);

  const toggleSidebar = () => {
    uiActions.toggleSidebar();
  };

  const handleConversationSelect = (newConversationId: string) => {
    if (projectId) {
      navigate(`/p/${projectId}/c/${newConversationId}`);
    }
  };

  // Don't render until we have a projectId
  if (!projectId) {
    return null;
  }

  return (
    <div className="flex-1 flex relative h-full w-full">
      <Suspense fallback={<div className="w-64 bg-gray-50 animate-pulse" />}>
        <ConversationSidebar
          isCollapsed={
            uiSnapshot.isMobile ? true : uiSnapshot.isSidebarCollapsed
          }
          onToggle={toggleSidebar}
          projectId={projectId}
          currentConversationId={effectiveConversationId}
          onConversationSelect={handleConversationSelect}
        />
      </Suspense>
      <div className="flex flex-1 flex-col min-w-0">
        {isNewRoute ? (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <NewChat />
          </Suspense>
        ) : (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <Chat />
          </Suspense>
        )}
      </div>
    </div>
  );
}
