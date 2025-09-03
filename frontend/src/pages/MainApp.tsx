import { Chat, ConversationSidebar } from "@/components/chat";
import { useEffect, memo } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { useSnapshot } from "valtio";
import { useLoggerDebug } from "@/hooks/use-logger-debug";
import { uiStore, uiActions } from "@/store/ui-store";

// Memoize the Chat component to prevent unnecessary re-renders
// But ensure it re-renders when conversationId or sidebar state changes
const MemoizedChat = memo(Chat, (prevProps, nextProps) => {
  return (
    prevProps.projectId === nextProps.projectId &&
    prevProps.conversationId === nextProps.conversationId &&
    prevProps.isSidebarCollapsed === nextProps.isSidebarCollapsed &&
    prevProps.onToggleSidebar === nextProps.onToggleSidebar
  );
});

export function MainApp() {
  const uiSnapshot = useSnapshot(uiStore);
  const { projectId, conversationId } = useParams<{
    projectId: string;
    conversationId?: string;
  }>();
  const navigate = useNavigate();

  // Enable debug logging hooks
  useLoggerDebug();

  // Handle redirection when visiting /chat/:projectId without conversation ID
  useEffect(() => {
    if (projectId && !conversationId) {
      // Try to get the last conversation from localStorage
      const lastConversationKey = `last_conversation_${projectId}`;
      const lastConversationId = localStorage.getItem(lastConversationKey);
      
      if (lastConversationId) {
        // Redirect to the last conversation
        navigate(`/chat/${projectId}/${lastConversationId}`, { replace: true });
      } else {
        // No last conversation, redirect to projects
        navigate("/projects", { replace: true });
      }
    }
  }, [projectId, conversationId, navigate]);

  // Save current conversation ID to localStorage when it changes
  useEffect(() => {
    if (projectId && conversationId) {
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
      navigate(`/chat/${projectId}/${newConversationId}`);
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
        currentConversationId={conversationId}
        onConversationSelect={handleConversationSelect}
      />
      <div className="flex flex-1 flex-col min-w-0">
        <MemoizedChat 
          projectId={projectId} 
          conversationId={conversationId}
          onToggleSidebar={toggleSidebar}
          isSidebarCollapsed={uiSnapshot.isMobile ? true : uiSnapshot.isSidebarCollapsed}
        />
      </div>
    </div>
  );
}
