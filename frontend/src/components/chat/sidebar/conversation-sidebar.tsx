import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import {
  ChevronDown,
  Plus,
  PanelLeftClose,
  PanelLeftOpen,
  MessageSquare,
  MoreHorizontal,
  Edit,
  Trash2,
  FileText,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { API_BASE_URL } from "@/lib/url";
import { cn } from "@/lib/utils";
import { useAuth } from "@/contexts/AuthContext";
import { ClaudeMdModal } from "./claude-md-modal";

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
  onConversationSelect: _onConversationSelect,
  refreshTrigger,
}: ConversationSidebarProps) {
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [renameDialogOpen, setRenameDialogOpen] = useState(false);
  const [renamingConversation, setRenamingConversation] = useState<Conversation | null>(null);
  const [newTitle, setNewTitle] = useState("");
  const [claudeMdModalOpen, setClaudeMdModalOpen] = useState(false);
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
          throw new Error(
            `Failed to fetch conversations: ${response.status} - ${response.statusText}`
          );
        }

        const data = await response.json();
        setConversations(data);
      } catch (err) {
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
    try {
      await logout();
      navigate("/auth");
    } catch (error) {
      // Logout failed
    }
  };

  // Handle profile click
  const handleProfile = () => {
    // TODO: Navigate to profile page or open profile modal
    // For now, just show an alert
    alert("Profile functionality not yet implemented");
  };

  // Handle settings click
  const handleSettings = () => {
    // TODO: Navigate to settings page or open settings modal
    // For now, just show an alert
    alert("Settings functionality not yet implemented");
  };

  // Handle conversation rename
  const handleRenameConversation = async () => {
    if (!renamingConversation || !newTitle.trim()) return;

    try {
      const response = await fetch(
        `${API_BASE_URL}/conversations/${renamingConversation.id}`,
        {
          method: "PUT",
          credentials: "include",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            title: newTitle.trim(),
          }),
        }
      );

      if (!response.ok) {
        throw new Error("Failed to rename conversation");
      }

      const updatedConversation = await response.json();
      
      // Update local state
      setConversations(convs => 
        convs.map(c => c.id === renamingConversation.id ? updatedConversation : c)
      );

      // Close dialog and reset state
      setRenameDialogOpen(false);
      setRenamingConversation(null);
      setNewTitle("");
    } catch (err) {
      setError("Failed to rename conversation");
    }
  };

  // Handle conversation delete
  const handleDeleteConversation = async (conversation: Conversation) => {
    if (!confirm(`Are you sure you want to delete "${conversation.title || 'New Conversation'}"?`)) {
      return;
    }

    try {
      const response = await fetch(
        `${API_BASE_URL}/conversations/${conversation.id}`,
        {
          method: "DELETE",
          credentials: "include",
        }
      );

      if (!response.ok) {
        throw new Error("Failed to delete conversation");
      }

      // Update local state
      setConversations(convs => convs.filter(c => c.id !== conversation.id));
      
      // If we're currently viewing this conversation, navigate away
      if (currentConversationId === conversation.id) {
        navigate(`/chat/${projectId}/new`);
      }
    } catch (err) {
      setError("Failed to delete conversation");
    }
  };

  // Open rename dialog
  const openRenameDialog = (conversation: Conversation) => {
    setRenamingConversation(conversation);
    setNewTitle(conversation.title || "");
    setRenameDialogOpen(true);
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
                  className={cn(
                    "block w-full text-left p-2 rounded-md hover:bg-muted transition-colors mb-1 group cursor-pointer relative",
                    currentConversationId === conversation.id && "bg-muted"
                  )}
                >
                  <div 
                    onClick={(e) => handleConversationClick(conversation.id, e)}
                    className="flex items-start gap-2 pr-8"
                  >
                    <MessageSquare className="h-4 w-4 mt-0.5 text-muted-foreground" />
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium truncate">
                        {conversation.title || "New Conversation"}
                      </p>
                      <p className="text-xs text-muted-foreground">
                        {conversation.message_count}{" "}
                        chat{conversation.message_count !== 1 ? "s" : ""} •{" "}
                        {new Date(conversation.updated_at).toLocaleDateString()} {new Date(conversation.updated_at).toLocaleTimeString([], {hour: '2-digit', minute:'2-digit', hour12: false})}
                      </p>
                    </div>
                  </div>
                  
                  {/* Actions dropdown */}
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
                            openRenameDialog(conversation);
                          }}
                        >
                          <Edit className="h-4 w-4 mr-2" />
                          Rename
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onClick={(e) => {
                            e.stopPropagation();
                            handleDeleteConversation(conversation);
                          }}
                          className="text-red-600 focus:text-red-600"
                        >
                          <Trash2 className="h-4 w-4 mr-2" />
                          Delete
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* CLAUDE.md Button */}
      {projectId && (
        <div className="p-3 border-t">
          <Button
            onClick={() => setClaudeMdModalOpen(true)}
            variant="ghost"
            size="sm"
            className={`w-full ${isCollapsed ? 'justify-center' : 'justify-start'} gap-2`}
          >
            <FileText className="h-4 w-4" />
            {!isCollapsed && "Edit CLAUDE.md"}
          </Button>
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
      
      {/* CLAUDE.md Modal */}
      {projectId && (
        <ClaudeMdModal
          projectId={projectId}
          isOpen={claudeMdModalOpen}
          onOpenChange={setClaudeMdModalOpen}
        />
      )}

      {/* Rename Dialog */}
      <Dialog open={renameDialogOpen} onOpenChange={setRenameDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Rename Conversation</DialogTitle>
          </DialogHeader>
          <div className="grid gap-4 py-4">
            <div className="grid grid-cols-4 items-center gap-4">
              <Label htmlFor="title" className="text-right">
                Title
              </Label>
              <Input
                id="title"
                value={newTitle}
                onChange={(e) => setNewTitle(e.target.value)}
                className="col-span-3"
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    handleRenameConversation();
                  }
                }}
                placeholder="Enter conversation title"
                autoFocus
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setRenameDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button
              type="button"
              onClick={handleRenameConversation}
              disabled={!newTitle.trim()}
            >
              Save
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
