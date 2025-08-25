import { Chat, ConversationSidebar } from "@/components/chat";
import { useState, useEffect, useRef, memo } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { API_BASE_URL } from "@/lib/url";

// Memoize the Chat component to prevent unnecessary re-renders
// But ensure it re-renders when conversationId changes
const MemoizedChat = memo(Chat, (prevProps, nextProps) => {
  return prevProps.projectId === nextProps.projectId && 
         prevProps.conversationId === nextProps.conversationId;
});

export function MainApp() {
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const [refreshTrigger, setRefreshTrigger] = useState(0);
  const { projectId, conversationId } = useParams<{ 
    projectId: string;
    conversationId?: string;
  }>();
  const navigate = useNavigate();
  const hasAutoNavigated = useRef(false);
  const lastConversationId = useRef(conversationId);
  
  // Load latest conversation when navigating to a project without a specific conversation
  // Only auto-navigate when there's no conversationId AND we're not intentionally on /new
  useEffect(() => {
    if (projectId && !conversationId && !hasAutoNavigated.current) {
      hasAutoNavigated.current = true;
      
      const loadLatestConversation = async () => {
        try {
          const response = await fetch(`${API_BASE_URL}/conversations?project_id=${encodeURIComponent(projectId)}`, {
            credentials: 'include'
          });
          
          if (!response.ok) {
            console.error('Failed to fetch conversations');
            // On error, navigate to new chat as fallback
            navigate(`/chat/${projectId}/new`, { replace: true });
            return;
          }
          
          const conversations = await response.json();
          
          if (conversations && conversations.length > 0) {
            // Navigate to the most recent conversation
            const latestConversation = conversations[0];
            navigate(`/chat/${projectId}/${latestConversation.id}`, { replace: true });
          } else {
            // If no conversations exist, navigate to new chat
            navigate(`/chat/${projectId}/new`, { replace: true });
          }
        } catch (error) {
          console.error('Error loading conversations:', error);
          // On error, navigate to new chat as fallback
          navigate(`/chat/${projectId}/new`, { replace: true });
        }
      };
      
      loadLatestConversation();
    }
  }, [projectId, conversationId, navigate]);
  
  // Reset the auto-navigation flag when conversation changes
  useEffect(() => {
    if (conversationId === 'new') {
      // When explicitly navigating to /new, set the flag to prevent auto-navigation
      hasAutoNavigated.current = true;
    } else if (conversationId && conversationId !== 'new') {
      // Reset for real conversations to allow future auto-navigation if needed
      hasAutoNavigated.current = false;
    }
  }, [conversationId])

  // Trigger sidebar refresh when a new conversation is created
  useEffect(() => {
    if (lastConversationId.current === 'new' && conversationId && conversationId !== 'new' && conversationId.startsWith('conv-')) {
      setRefreshTrigger(prev => prev + 1);
    }
    lastConversationId.current = conversationId;
  }, [conversationId])

  const toggleSidebar = () => {
    setIsSidebarCollapsed(!isSidebarCollapsed);
  };

  const handleConversationSelect = (newConversationId: string) => {
    if (projectId) {
      navigate(`/chat/${projectId}/${newConversationId}`);
    }
  };

  return (
    <div className="h-screen flex">
      <ConversationSidebar
        isCollapsed={isSidebarCollapsed}
        onToggle={toggleSidebar}
        projectId={projectId}
        currentConversationId={conversationId}
        onConversationSelect={handleConversationSelect}
        refreshTrigger={refreshTrigger}
      />
      <div className="flex flex-1 flex-col">
        <MemoizedChat 
          projectId={projectId} 
          conversationId={conversationId}
        />
      </div>
    </div>
  );
}
