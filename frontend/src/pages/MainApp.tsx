import { Chat, ConversationSidebar } from "@/components/chat";
import { NewChat } from "@/components/chat/main/new-chat";
import { useEffect } from "react";
import { useParams, useNavigate, useLocation } from "react-router-dom";
import { useSnapshot } from "valtio";
import { useLoggerDebug } from "@/hooks/use-logger-debug";
import { uiStore, uiActions } from "@/store/ui-store";
import { api } from "@/lib/utils/api";


export function MainApp() {
  const uiSnapshot = useSnapshot(uiStore);
  const { projectId, conversationId } = useParams<{
    projectId: string;
    conversationId?: string;
  }>();
  const navigate = useNavigate();
  const location = useLocation();
  
  // Check if we're on the new conversation route
  const isNewRoute = location.pathname.endsWith('/new');

  // Enable debug logging hooks
  useLoggerDebug();

  // Update valtio store with current route params
  useEffect(() => {
    if (projectId) {
      uiActions.setCurrentProject(projectId);
    }
    if (conversationId) {
      uiActions.setCurrentConversation(conversationId);
    }
    // Set transition flag based on navigation state
    if (location.state?.fromNewChat) {
      uiActions.setTransitioningFromNew(true);
      // Clear the flag after a short delay to reset state
      const timer = setTimeout(() => {
        uiActions.setTransitioningFromNew(false);
      }, 100);
      return () => clearTimeout(timer);
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
    if (projectId && conversationId && conversationId !== 'new') {
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
      <ConversationSidebar
        isCollapsed={uiSnapshot.isMobile ? true : uiSnapshot.isSidebarCollapsed}
        onToggle={toggleSidebar}
        projectId={projectId}
        currentConversationId={effectiveConversationId}
        onConversationSelect={handleConversationSelect}
      />
      <div className="flex flex-1 flex-col min-w-0">
        {isNewRoute ? (
          <NewChat />
        ) : (
          <Chat />
        )}
      </div>
    </div>
  );
}
