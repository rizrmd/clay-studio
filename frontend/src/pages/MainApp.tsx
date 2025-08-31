import { Chat, ConversationSidebar } from "@/components/chat";
import { useState, useEffect, memo } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { useLoggerDebug } from "@/hooks/use-logger-debug";

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
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const [isMobile, setIsMobile] = useState(window.innerWidth < 768);
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
        // No last conversation, redirect to new
        navigate(`/chat/${projectId}/new`, { replace: true });
      }
    }
  }, [projectId, conversationId, navigate]);

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
      setIsMobile(mobile);
      // Auto-collapse sidebar on mobile
      if (mobile && !isSidebarCollapsed) {
        setIsSidebarCollapsed(true);
      }
    };

    handleResize(); // Check initial size
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [isSidebarCollapsed]);

  const toggleSidebar = () => {
    setIsSidebarCollapsed(!isSidebarCollapsed);
  };

  const handleConversationSelect = (newConversationId: string) => {
    if (projectId) {
      navigate(`/chat/${projectId}/${newConversationId}`);
    }
  };

  return (
    <div className="md:h-screen flex relative md:overflow-hidden">
      <ConversationSidebar
        isCollapsed={isMobile ? true : isSidebarCollapsed}
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
          isSidebarCollapsed={isMobile ? true : isSidebarCollapsed}
        />
      </div>
    </div>
  );
}
