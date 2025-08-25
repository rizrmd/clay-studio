import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import {
  ChevronDown,
  Plus,
  PanelLeftClose,
  PanelLeftOpen,
  MessageSquare,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { API_BASE_URL } from "@/lib/url";
import { cn } from "@/lib/utils";
import { useAuth } from "@/contexts/AuthContext";

interface Conversation {
  id: string;
  project_id: string;
  title: string | null;
  message_count: number;
  created_at: string;
  updated_at: string;
}

interface ConversationSidebarProps {
  isCollapsed: boolean;
  onToggle: () => void;
  projectId?: string;
  currentConversationId?: string;
  onConversationSelect?: (conversationId: string) => void;
  refreshTrigger?: number; // Can be used to trigger refresh from parent
}

export function ConversationSidebar({
  isCollapsed,
  onToggle,
  projectId,
  currentConversationId,
  onConversationSelect,
  refreshTrigger,
}: ConversationSidebarProps) {
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { isAuthenticated, isSetupComplete, user, logout } = useAuth();
  const navigate = useNavigate();

  // Fetch conversations when projectId changes and user is authenticated
  useEffect(() => {
    if (!projectId || !isAuthenticated || !isSetupComplete) return;

    const fetchConversations = async () => {
      // Only show loading state if we don't have conversations yet
      if (conversations.length === 0) {
        setLoading(true);
      }
      setError(null);
      try {
        const url = `${API_BASE_URL}/conversations?project_id=${encodeURIComponent(
          projectId
        )}`;

        const response = await fetch(url, {
          credentials: "include",
        });

        if (!response.ok) {
          const errorText = await response.text();
          console.error("Conversations fetch failed:", {
            status: response.status,
            statusText: response.statusText,
            url: url.toString(),
            errorText,
          });
          throw new Error(
            `Failed to fetch conversations: ${response.status} - ${response.statusText}`
          );
        }

        const data = await response.json();
        setConversations(data);
      } catch (err) {
        console.error("Failed to fetch conversations:", err);
        setError(
          err instanceof Error ? err.message : "Failed to load conversations"
        );
      } finally {
        setLoading(false);
      }
    };

    fetchConversations();
  }, [projectId, refreshTrigger, isAuthenticated, isSetupComplete]);

  // Handle conversation click
  const handleConversationClick = (
    conversationId: string,
    e: React.MouseEvent
  ) => {
    e.preventDefault();

    // Don't navigate if already on this conversation
    if (currentConversationId === conversationId) return;

    // Navigate to the conversation
    navigate(`/chat/${projectId}/${conversationId}`);
  };


  // Handle logout
  const handleLogout = async () => {
    console.log("Logout clicked!"); // Debug log
    try {
      await logout();
      navigate("/auth");
    } catch (error) {
      console.error("Logout failed:", error);
    }
  };

  // Handle profile click
  const handleProfile = () => {
    console.log("Profile clicked!");
    // TODO: Navigate to profile page or open profile modal
    // For now, just show an alert
    alert("Profile functionality not yet implemented");
  };

  // Handle settings click
  const handleSettings = () => {
    console.log("Settings clicked!");
    // TODO: Navigate to settings page or open settings modal
    // For now, just show an alert
    alert("Settings functionality not yet implemented");
  };

  return (
    <div
      className={`${
        isCollapsed ? "w-12" : "w-64"
      } border-r bg-background flex flex-col transition-all duration-300`}
    >
      {/* Header with toggle and new chat */}
      <div className="p-3 border-b">
        <div className="flex items-center justify-between">
          <Button
            variant="ghost"
            size="sm"
            onClick={onToggle}
            className="h-8 w-8 p-0"
          >
            {isCollapsed ? (
              <PanelLeftOpen className="h-4 w-4" />
            ) : (
              <PanelLeftClose className="h-4 w-4" />
            )}
          </Button>

          {!isCollapsed && projectId && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                navigate(`/chat/${projectId}/new`);
              }}
              className="h-8 px-3 gap-1"
              type="button"
            >
              <Plus className="h-4 w-4" />
              New Chat
            </Button>
          )}
        </div>
      </div>

      {/* Conversations area */}
      {!isCollapsed && (
        <div className="flex-1 overflow-y-auto">
          {loading ? (
            <div className="p-4">
              <div className="animate-pulse">
                <div className="h-4 bg-gray-200 rounded mb-2"></div>
                <div className="h-4 bg-gray-200 rounded mb-2"></div>
                <div className="h-4 bg-gray-200 rounded"></div>
              </div>
            </div>
          ) : error ? (
            <div className="p-4">
              <p className="text-sm text-red-500">{error}</p>
            </div>
          ) : conversations.length === 0 ? (
            <div className="p-4">
              <p className="text-sm text-muted-foreground">
                Your conversations will appear here once you start chatting!
              </p>
            </div>
          ) : (
            <div className="p-2">
              {conversations.map((conversation) => (
                <div
                  key={conversation.id}
                  onClick={(e) => handleConversationClick(conversation.id, e)}
                  className={cn(
                    "block w-full text-left p-2 rounded-md hover:bg-muted transition-colors mb-1 group cursor-pointer",
                    currentConversationId === conversation.id && "bg-muted"
                  )}
                >
                  <div className="flex items-start gap-2">
                    <MessageSquare className="h-4 w-4 mt-0.5 text-muted-foreground" />
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium truncate">
                        {conversation.title || "New Conversation"}
                      </p>
                      <p className="text-xs text-muted-foreground">
                        {conversation.message_count}{" "}
                        message{conversation.message_count !== 1 ? "s" : ""} •{" "}
                        {new Date(conversation.updated_at).toLocaleDateString()} {new Date(conversation.updated_at).toLocaleTimeString([], {hour: '2-digit', minute:'2-digit', hour12: false})}
                      </p>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Bottom user section */}
      <div className="border-t p-3 relative z-10">
        {isCollapsed ? (
          <button
            className="h-8 w-8 p-0 cursor-pointer hover:bg-accent rounded-md flex items-center justify-center pointer-events-auto"
            onClick={handleLogout}
            type="button"
          >
            <div className="h-6 w-6 rounded-full bg-primary/10 flex items-center justify-center text-primary font-medium text-xs pointer-events-none">
              {(user?.username || "G").charAt(0).toUpperCase()}
            </div>
          </button>
        ) : (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                className="w-full justify-between p-2 cursor-pointer hover:bg-accent rounded-md flex items-center pointer-events-auto"
                type="button"
              >
                <div className="flex items-center gap-2 pointer-events-none">
                  <div className="h-6 w-6 rounded-full bg-primary/10 flex items-center justify-center text-primary font-medium text-xs">
                    {(user?.username || "G").charAt(0).toUpperCase()}
                  </div>
                  <span className="text-sm">{user?.username || "Guest"}</span>
                </div>
                <ChevronDown className="h-4 w-4 pointer-events-none" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start" className="w-56 z-50">
              <DropdownMenuItem
                onClick={handleProfile}
                className="cursor-pointer"
              >
                Profile
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={handleSettings}
                className="cursor-pointer"
              >
                Settings
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={handleLogout}
                className="cursor-pointer"
              >
                Sign out
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        )}
      </div>
    </div>
  );
}
