import { useState, useEffect, useRef, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useSnapshot } from "valtio";
import {
  ChevronDown,
  Plus,
  PanelLeftClose,
  PanelLeftOpen,
  MessageSquare,
  MoreHorizontal,
  Edit,
  Trash2,
  Loader2,
  ChevronLeft,
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
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";
import { logger } from "@/lib/logger";
import { useValtioAuth } from "@/hooks/use-valtio-auth";
import { ClaudeMdModal } from "./claude-md-modal";
import { store, cleanupDeletedConversation } from "@/store/chat-store";
import { ConversationManager } from "@/store/chat/conversation-manager";
import { conversationStore } from "@/store/chat/conversation-store";

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
}

export function ConversationSidebar({
  isCollapsed,
  onToggle: _onToggle,
  projectId,
  currentConversationId,
  onConversationSelect: _onConversationSelect,
}: ConversationSidebarProps) {
  // Get the active conversation ID from store to handle /new -> real ID transition
  const actualConversationId =
    currentConversationId === "new" && store.activeConversationId
      ? store.activeConversationId
      : currentConversationId;
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [renameDialogOpen, setRenameDialogOpen] = useState(false);
  const [renamingConversation, setRenamingConversation] =
    useState<Conversation | null>(null);
  const [newTitle, setNewTitle] = useState("");
  const [claudeMdModalOpen, setClaudeMdModalOpen] = useState(false);
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
  const [recentlyUpdatedConversations, setRecentlyUpdatedConversations] =
    useState<Set<string>>(new Set());
  const { isAuthenticated, isSetupComplete, user, logout } = useValtioAuth();
  const navigate = useNavigate();
  const fetchTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const lastFetchTimeRef = useRef<number>(0);

  // Track loading state for conversations
  const snapshot = useSnapshot(store.conversations);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (fetchTimeoutRef.current) {
        clearTimeout(fetchTimeoutRef.current);
      }
    };
  }, []);

  // Auto-close mobile menu on larger screens
  useEffect(() => {
    const handleResize = () => {
      if (window.innerWidth >= 768) {
        setIsMobileMenuOpen(false);
      }
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  // Shared function to fetch conversations with debouncing
  const fetchConversations = useCallback(async (showLoadingState = true, forceImmediate = false) => {
    if (!projectId || !isAuthenticated || !isSetupComplete) return;

    // Implement debouncing for background refreshes (not for initial load)
    if (!forceImmediate && !showLoadingState) {
      const now = Date.now();
      const timeSinceLastFetch = now - lastFetchTimeRef.current;
      
      // If we fetched less than 1 second ago, debounce
      if (timeSinceLastFetch < 1000) {
        // Clear any existing timeout
        if (fetchTimeoutRef.current) {
          clearTimeout(fetchTimeoutRef.current);
        }
        
        // Set a new timeout to fetch after the debounce period
        fetchTimeoutRef.current = setTimeout(() => {
          fetchConversations(false, true); // Force immediate after debounce
        }, 1000 - timeSinceLastFetch);
        
        return;
      }
    }

    // Record the fetch time
    lastFetchTimeRef.current = Date.now();

    // Only show loading state if requested and we don't have conversations yet
    if (showLoadingState && conversations.length === 0) {
      setLoading(true);
    }
    setError(null);
    try {
      const url = `/conversations?project_id=${encodeURIComponent(
        projectId
      )}`;

      const response = await api.fetchStream(url);

      if (!response.ok) {
        throw new Error(
          `Failed to fetch conversations: ${response.status} - ${response.statusText}`
        );
      }

      const data = await response.json();
      setConversations(data);

      // Validate current conversation ID against fetched conversations
      // Only redirect if we have an invalid conversation ID and there are existing conversations
      // OR if there are no conversations at all (empty state should go to /new)
      if (
        currentConversationId &&
        currentConversationId !== "new" &&
        !data.find((conv: Conversation) => conv.id === currentConversationId)
      ) {
        // Current conversation ID doesn't exist in the list, redirect to new
        navigate(`/chat/${projectId}/new`, { replace: true });
      }
    } catch (err) {
      if (showLoadingState) {
        setError(
          err instanceof Error ? err.message : "Failed to load conversations"
        );
      } else {
        logger.error(
          "ConversationSidebar: Failed to refresh conversations:",
          err
        );
      }
    } finally {
      if (showLoadingState) {
        setLoading(false);
      }
    }
  }, [projectId, isAuthenticated, isSetupComplete, currentConversationId, navigate, conversations.length]);

  // Initial fetch when component mounts or auth changes
  useEffect(() => {
    if (!projectId || !isAuthenticated || !isSetupComplete) return;
    fetchConversations(true, true); // true = show loading, true = force immediate
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, isAuthenticated, isSetupComplete]); // Removed currentConversationId from deps to prevent refetch on navigation

  // Set up event listeners for real-time sidebar updates
  useEffect(() => {
    if (!projectId) return;

    // Listen for conversation creation events to refresh the list
    const handleConversationCreated = (event: CustomEvent) => {
      if (event.detail?.projectId === projectId) {
        logger.info(
          "ConversationSidebar: New conversation created, refreshing list",
          event.detail
        );

        // Just fetch the real data from server - don't add placeholder
        // This ensures we get the correct count and avoid race conditions
        fetchConversations(false); // false = no loading state for background refresh
      }
    };

    // Listen for message sent events to update message counts
    const handleMessageSent = (event: CustomEvent) => {
      if (
        event.detail?.projectId === projectId &&
        event.detail?.conversationId
      ) {
        logger.info(
          "ConversationSidebar: Message sent, updating conversation list"
        );

        // Add to recently updated set (no auto-removal, will be removed on click)
        setRecentlyUpdatedConversations(
          (prev) => new Set([...prev, event.detail.conversationId])
        );

        // Update the message count for the specific conversation
        setConversations((prevConversations) =>
          prevConversations
            .map((conv) => {
              if (conv.id === event.detail.conversationId) {
                return {
                  ...conv,
                  message_count: conv.message_count + 2, // +1 for user message, +1 for assistant response
                  updated_at: new Date().toISOString(),
                };
              }
              return conv;
            })
            .sort(
              (a, b) =>
                new Date(b.created_at).getTime() -
                new Date(a.created_at).getTime()
            )
        );
      }
    };

    // Listen for streaming events (no need to update state, valtio handles it)
    const handleStreamingStarted = (event: CustomEvent) => {
      if (event.detail?.projectId === projectId) {
        logger.debug(
          "ConversationSidebar: Streaming started for",
          event.detail.conversationId
        );
      }
    };

    const handleStreamingStopped = (event: CustomEvent) => {
      if (event.detail?.projectId === projectId) {
        logger.debug(
          "ConversationSidebar: Streaming stopped for",
          event.detail.conversationId
        );
      }
    };

    window.addEventListener(
      "conversation-created",
      handleConversationCreated as EventListener
    );
    window.addEventListener("message-sent", handleMessageSent as EventListener);
    window.addEventListener(
      "streaming-started",
      handleStreamingStarted as EventListener
    );
    window.addEventListener(
      "streaming-stopped",
      handleStreamingStopped as EventListener
    );

    return () => {
      window.removeEventListener(
        "conversation-created",
        handleConversationCreated as EventListener
      );
      window.removeEventListener(
        "message-sent",
        handleMessageSent as EventListener
      );
      window.removeEventListener(
        "streaming-started",
        handleStreamingStarted as EventListener
      );
      window.removeEventListener(
        "streaming-stopped",
        handleStreamingStopped as EventListener
      );
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, isAuthenticated, isSetupComplete]);

  // Handle conversation click
  const handleConversationClick = (
    conversationId: string,
    e: React.MouseEvent
  ) => {
    e.preventDefault();

    // Remove from recently updated set when clicked
    setRecentlyUpdatedConversations((prev) => {
      const newSet = new Set(prev);
      newSet.delete(conversationId);
      return newSet;
    });

    // Don't navigate if already on this conversation
    if (actualConversationId === conversationId) {
      // Close mobile menu if open
      setIsMobileMenuOpen(false);
      return;
    }

    // Navigate to the conversation
    navigate(`/chat/${projectId}/${conversationId}`);

    // Close mobile menu after navigation
    setIsMobileMenuOpen(false);
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
      const response = await api.fetchStream(
        `/conversations/${renamingConversation.id}`,
        {
          method: "PUT",
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
      setConversations((convs) =>
        convs.map((c) =>
          c.id === renamingConversation.id ? updatedConversation : c
        )
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
    if (
      !confirm(
        `Are you sure you want to delete "${
          conversation.title || "New Conversation"
        }"?`
      )
    ) {
      return;
    }

    try {
      const response = await api.fetchStream(
        `/conversations/${conversation.id}`,
        {
          method: "DELETE",
        }
      );

      if (!response.ok) {
        throw new Error("Failed to delete conversation");
      }

      // Update local state
      setConversations((convs) =>
        convs.filter((c) => c.id !== conversation.id)
      );

      // Clean up the deleted conversation from store
      cleanupDeletedConversation(conversation.id);

      // Clear from localStorage if this was the last viewed conversation
      const lastConversationKey = `last_conversation_${projectId}`;
      const lastConversationId = localStorage.getItem(lastConversationKey);
      if (lastConversationId === conversation.id) {
        localStorage.removeItem(lastConversationKey);
      }

      // If we're currently viewing this conversation, navigate away
      if (actualConversationId === conversation.id) {
        // Navigate to new conversation
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
    <>
      {/* Mobile overlay */}
      {isMobileMenuOpen && (
        <div
          className="fixed inset-0 bg-black/50 z-40 md:hidden"
          onClick={() => setIsMobileMenuOpen(false)}
        />
      )}

      {/* Sidebar */}
      <div
        className={cn(
          "border-r bg-background flex flex-col transition-all duration-300",
          // Desktop width
          isCollapsed ? "md:w-12" : "md:max-w-64 md:min-w-64",
          // Mobile: full height overlay or hidden
          "fixed md:relative inset-y-0 left-0 z-50",
          isMobileMenuOpen ? "w-64" : "w-0 md:w-auto",
          !isMobileMenuOpen && "overflow-hidden md:overflow-visible"
        )}
      >
        {/* Header with back to projects and new chat */}
        <div className="px-1 py-2 border-b">
          <div className="flex items-center justify-between">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                // On mobile, close the menu
                if (window.innerWidth < 768) {
                  setIsMobileMenuOpen(false);
                } else {
                  // Navigate back to projects
                  navigate("/projects");
                }
              }}
              className="pl-1 gap-1 h-[25px] border border-transparent hover:border-gray-200"
            >
              <ChevronLeft size={10} />
              <span className="text-xs">Projects</span>
            </Button>

            {!isCollapsed && projectId && (
              <Button
                variant="ghost"
                size="sm"
                className="pl-1 gap-1 h-[25px] border border-transparent hover:border-gray-200"
                onClick={async () => {
                  const conversationManager = ConversationManager.getInstance();
                  
                  // Get the current active conversation ID from both stores
                  const previousActiveId = store.activeConversationId || conversationStore.activeConversationId;
                  
                  // Clear active conversation IDs in both stores
                  store.activeConversationId = null;
                  conversationStore.activeConversationId = null;

                  // Clear any existing 'new' conversation state in both stores
                  if (store.conversations.new) {
                    delete store.conversations.new;
                    delete store.inputs.new;
                  }
                  if (conversationStore.conversations.new) {
                    await conversationManager.clearConversation('new');
                  }

                  // If we were on a newly created conversation (that now has a real ID), 
                  // clear its state too to prevent message bleeding
                  if (previousActiveId && previousActiveId !== 'new') {
                    // Clear the conversation state in old store
                    if (store.conversations[previousActiveId]) {
                      store.conversations[previousActiveId].messages = [];
                      store.conversations[previousActiveId].isStreaming = false;
                      store.conversations[previousActiveId].isLoading = false;
                      store.conversations[previousActiveId].error = null;
                    }
                    
                    // Clear the conversation state in new store
                    if (conversationStore.conversations[previousActiveId]) {
                      await conversationManager.clearConversation(previousActiveId);
                    }
                  }

                  // Navigate to new chat
                  navigate(`/chat/${projectId}/new`);
                  // Close mobile menu after navigation
                  setIsMobileMenuOpen(false);
                }}
                type="button"
              >
                <Plus className="h-4 w-4" />
                New Chat
              </Button>
            )}
          </div>
        </div>

        {/* Conversations area */}
        {(!isCollapsed || isMobileMenuOpen) && (
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
                <p className="text-xs text-muted-foreground">Ummm... </p>
                <p className="text-xs text-muted-foreground">Let's talk ? </p>
              </div>
            ) : (
              <div className="p-2 min-w-[130px] ">
                {conversations.map((conversation) => (
                  <div
                    key={conversation.id}
                    className={cn(
                      "block w-full group text-left p-2 rounded-md hover:bg-muted border border-transparent transition-colors mb-1 group cursor-pointer relative",
                      actualConversationId === conversation.id &&
                        "bg-muted border-blue-700/30"
                    )}
                    onClick={(e) => handleConversationClick(conversation.id, e)}
                  >
                    <div
                      className={cn(
                        "flex items-start gap-2 overflow-hidden group-hover:pr-8"
                      )}
                    >
                      <div className="relative flex flex-col items-center pt-1">
                        {snapshot[conversation.id]?.isLoading ||
                        snapshot[conversation.id]?.isStreaming ? (
                          <Loader2 className="h-4 w-4 text-muted-foreground animate-spin" />
                        ) : (
                          <MessageSquare
                            className={cn(
                              "h-4 w-4",
                              recentlyUpdatedConversations.has(conversation.id)
                                ? "text-green-500"
                                : " text-muted-foreground"
                            )}
                          />
                        )}
                        {/* Green notification dot below icon */}
                        {recentlyUpdatedConversations.has(conversation.id) && (
                          <div className="h-[6px] w-[6px] rounded-full bg-green-500 mt-1" />
                        )}
                      </div>
                      <div className="flex-1 min-w-0">
                        <p
                          className={cn(
                            "text-sm font-medium truncate",
                            recentlyUpdatedConversations.has(conversation.id) &&
                              "text-green-500"
                          )}
                        >
                          {conversation.title || "New Conversation"}
                        </p>
                        <p className="text-xs text-muted-foreground">
                          {conversation.message_count} chat
                          {conversation.message_count !== 1 ? "s" : ""} •{" "}
                          {new Date(
                            conversation.updated_at
                          ).toLocaleDateString()}{" "}
                          {new Date(conversation.updated_at).toLocaleTimeString(
                            [],
                            {
                              hour: "2-digit",
                              minute: "2-digit",
                              hour12: false,
                            }
                          )}
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
                    if (e.key === "Enter") {
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

      {/* Mobile menu toggle button */}
      <Button
        variant="ghost"
        size="sm"
        onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
        className="fixed top-4 left-4 z-40 h-10 w-10 p-0 md:hidden rounded-full shadow-lg bg-background border"
      >
        {isMobileMenuOpen ? (
          <PanelLeftClose className="h-5 w-5" />
        ) : (
          <PanelLeftOpen className="h-5 w-5" />
        )}
      </Button>
    </>
  );
}
