import { useEffect, useRef, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useSnapshot } from "valtio";
import {
  ChevronDown,
  PanelLeftClose,
  PanelLeftOpen,
  MessageSquare,
  MoreHorizontal,
  Edit,
  Trash2,
  Loader2,
  ChevronLeft,
  X,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
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
import { api } from "@/lib/utils/api";
import { cn } from "@/lib/utils";
import { logger } from "@/lib/utils/logger";
import { useValtioAuth } from "@/hooks/use-valtio-auth";
import { ConversationManager } from "@/store/chat/conversation-manager";
import { conversationStore } from "@/store/chat/conversation-store";
import { sidebarStore, sidebarActions } from "@/store/sidebar-store";
import { chatEventBus } from "@/lib/services/chat/event-bus";
import { MessageCacheService } from "@/lib/services/chat/message-cache";

interface Conversation {
  id: string;
  project_id: string;
  title: string;
  message_count: number;
  created_at: string;
  updated_at: string;
  is_title_manually_set?: boolean;
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
  const sidebarSnapshot = useSnapshot(sidebarStore);
  const conversationStoreSnapshot = useSnapshot(conversationStore);
  
  const actualConversationId = currentConversationId;
  const { isAuthenticated, isSetupComplete, user, logout } = useValtioAuth();
  const navigate = useNavigate();
  const fetchTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const lastFetchTimeRef = useRef<number>(0);
  const lastMessageCountRef = useRef<Record<string, number>>({});
  const messageCache = MessageCacheService.getInstance();
  const prefetchedConversations = useRef<Set<string>>(new Set());


  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (fetchTimeoutRef.current) {
        clearTimeout(fetchTimeoutRef.current);
      }
    };
  }, []);
  
  // Track new messages in non-active conversations
  useEffect(() => {
    // Check all conversations for new messages
    Object.keys(conversationStoreSnapshot.conversations).forEach(conversationId => {
      const state = conversationStoreSnapshot.conversations[conversationId];
      if (!state) return;
      
      // Count messages in this conversation
      const currentMessageCount = state.messages?.length || 0;
      const previousMessageCount = lastMessageCountRef.current[conversationId];
      
      // Only mark as updated if:
      // 1. We've seen this conversation before (not undefined)
      // 2. It's not the active conversation
      // 3. It has new messages
      // 4. The conversation is currently streaming (real-time update) or
      //    the last message is recent (within last 10 seconds)
      if (previousMessageCount !== undefined && 
          conversationId !== actualConversationId && 
          currentMessageCount > previousMessageCount) {
        
        // Check if this is a real-time update or just loading old messages
        const lastMessage = state.messages[state.messages.length - 1];
        const isRealTimeUpdate = state.status === 'streaming' || 
          (lastMessage && lastMessage.createdAt &&
           new Date().getTime() - new Date(lastMessage.createdAt).getTime() < 10000);
        
        if (isRealTimeUpdate) {
          sidebarActions.addRecentlyUpdated(conversationId);
        }
      }
      
      // Update the count
      lastMessageCountRef.current[conversationId] = currentMessageCount;
    });
  }, [conversationStoreSnapshot, actualConversationId]);

  // Auto-close mobile menu on larger screens
  useEffect(() => {
    const handleResize = () => {
      if (window.innerWidth >= 768) {
        sidebarActions.setMobileMenuOpen(false);
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

    // Only show sidebarSnapshot.loading state if requested and we don't have conversations yet
    if (showLoadingState && sidebarSnapshot.conversations.length === 0) {
      sidebarActions.setLoading(true);
    }
    sidebarActions.setError(null);
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
      sidebarActions.setConversations(data);

      // Validate current conversation ID against fetched conversations
      // Only redirect if we have an invalid conversation ID and there are existing conversations
      // OR if there are no conversations at all (empty state should go to /new)
      if (
        currentConversationId &&
        !data.find((conv: Conversation) => conv.id === currentConversationId)
      ) {
        // Current conversation ID doesn't exist in the list, redirect to new
        navigate("/projects", { replace: true });
      }
    } catch (err) {
      if (showLoadingState) {
        sidebarActions.setError(
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
        sidebarActions.setLoading(false);
      }
    }
  }, [projectId, isAuthenticated, isSetupComplete, currentConversationId, navigate, sidebarSnapshot.conversations.length]);

  // Initial fetch when component mounts or auth changes
  useEffect(() => {
    if (!projectId || !isAuthenticated || !isSetupComplete) return;
    fetchConversations(true, true); // true = show sidebarSnapshot.loading, true = force immediate
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, isAuthenticated, isSetupComplete]); // Removed currentConversationId from deps to prevent refetch on navigation

  // Set up event listeners for real-time sidebar updates
  useEffect(() => {
    if (!projectId) return;

    // Listen for conversation creation events to refresh the list
    const handleConversationCreated = (event: CustomEvent) => {
      if (event.detail?.projectId === projectId) {
        // Just fetch the real data from server - don't add placeholder
        // This ensures we get the correct count and avoid race conditions
        fetchConversations(false); // false = no sidebarSnapshot.loading state for background refresh
      }
    };

    // Listen for message sent events to update message counts
    const handleMessageSent = (event: CustomEvent) => {
      if (
        event.detail?.projectId === projectId &&
        event.detail?.conversationId
      ) {
        // Add to recently updated set
        sidebarActions.addRecentlyUpdated(event.detail.conversationId);

        // Update the message count for the specific conversation
        const conversations = sidebarSnapshot.conversations.map((conv) => {
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
            );
        sidebarActions.setConversations(conversations);
      }
    };

    // Listen for streaming events (no need to update state, valtio handles it)
    const handleStreamingStarted = (event: CustomEvent) => {
      if (event.detail?.projectId === projectId) {
        // Streaming started
      }
    };

    const handleStreamingStopped = (event: CustomEvent) => {
      if (event.detail?.projectId === projectId) {
        // Streaming stopped
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

  // Subscribe to title update events from chatEventBus
  useEffect(() => {
    const unsubscribe = chatEventBus.subscribe(
      'CONVERSATION_TITLE_UPDATED',
      async (event: any) => {
        if (event.type === 'CONVERSATION_TITLE_UPDATED') {
          
          // Update the conversation in the local state
          sidebarActions.updateConversation(event.conversationId, { title: event.title });
          
          // Add to recently updated set for visual feedback
          sidebarActions.addRecentlyUpdated(event.conversationId);
        }
      }
    );

    return () => {
      unsubscribe();
    };
  }, []);

  // Handle conversation click
  const handleConversationClick = (
    conversationId: string,
    e: React.MouseEvent
  ) => {
    e.preventDefault();

    // If in delete mode, toggle selection instead of navigating
    if (sidebarSnapshot.isDeleteMode) {
      sidebarActions.toggleConversationSelection(conversationId);
      return;
    }

    // Remove from recently updated set when clicked
    sidebarStore.recentlyUpdatedConversations.delete(conversationId);

    // Don't navigate if already on this conversation
    if (actualConversationId === conversationId) {
      // Close mobile menu if open
      sidebarActions.setMobileMenuOpen(false);
      return;
    }

    // Navigate to the conversation
    navigate(`/chat/${projectId}/${conversationId}`);

    // Close mobile menu after navigation
    sidebarActions.setMobileMenuOpen(false);
  };

  // Prefetch conversation on hover for instant switching
  const handleConversationHover = useCallback((conversationId: string) => {
    // Skip if already prefetched or if it's the current conversation
    if (prefetchedConversations.current.has(conversationId) || 
        conversationId === actualConversationId) {
      return;
    }

    // Check if already cached
    const cached = messageCache.getCachedMessages(conversationId);
    if (cached && cached.length > 0) {
      // Already cached, just mark as prefetched
      prefetchedConversations.current.add(conversationId);
      return;
    }

    // Mark as prefetched to avoid duplicate requests
    prefetchedConversations.current.add(conversationId);
    logger.debug(`Prefetch hover for conversation ${conversationId}`);
    // Actual prefetching will be triggered by WebSocket subscription when clicked
  }, [actualConversationId, messageCache]);

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

  // Handle conversation rename
  const handleRenameConversation = async () => {
    if (!sidebarSnapshot.renamingConversation || !sidebarSnapshot.newTitle.trim()) return;

    try {
      const response = await api.fetchStream(
        `/conversations/${sidebarSnapshot.renamingConversation.id}`,
        {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            title: sidebarSnapshot.newTitle.trim(),
          }),
        }
      );

      if (!response.ok) {
        throw new Error("Failed to rename conversation");
      }

      const updatedConversation = await response.json();

      // Update local state
      const updatedConversations = sidebarSnapshot.conversations.map((c) =>
        c.id === sidebarSnapshot.renamingConversation!.id ? updatedConversation : c
      );
      sidebarActions.setConversations(updatedConversations);

      // Close dialog and reset state
      sidebarActions.closeRenameDialog();
    } catch (err) {
      sidebarActions.setError("Failed to rename conversation");
    }
  };



  // Handle creating new conversation
  const handleCreateNewConversation = () => {
    if (!projectId) return;

    // Navigate to new conversation state without creating conversation yet
    navigate(`/chat/${projectId}/new`);

    // Close mobile menu after navigation
    sidebarActions.setMobileMenuOpen(false);
  };

  // Open rename dialog
  const openRenameDialog = (conversation: Conversation) => {
    sidebarActions.openRenameDialog(conversation);
  };

  return (
    <>
      {/* Mobile overlay */}
      {sidebarSnapshot.isMobileMenuOpen && (
        <div
          className="fixed inset-0 bg-black/50 z-40 md:hidden"
          onClick={() => sidebarActions.setMobileMenuOpen(false)}
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
          sidebarSnapshot.isMobileMenuOpen ? "w-64" : "w-0 md:w-auto",
          !sidebarSnapshot.isMobileMenuOpen && "overflow-hidden md:overflow-visible"
        )}
      >
        {/* Header with back to projects and new chat */}
        <div className="px-1 py-2 border-b">
           <div className="flex items-center justify-between">
             {!sidebarSnapshot.isDeleteMode && (
               <Button
                 variant="ghost"
                 size="sm"
                 onClick={() => {
                   // Navigate back to projects
                   navigate("/projects");
                   // Also close mobile menu if open
                   sidebarActions.setMobileMenuOpen(false);
                 }}
                 className="pl-1 gap-1 h-[25px] border border-transparent hover:border-gray-200"
               >
                 <ChevronLeft size={10} />
                 <span className="text-xs">Projects</span>
               </Button>
             )}

             {sidebarSnapshot.isDeleteMode ? (
              <div className="flex items-center gap-1">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => sidebarActions.exitDeleteMode()}
                  className="gap-1 h-[25px] border border-transparent hover:border-gray-200"
                  title="Cancel"
                >
                  <X size={10} />
                  <span className="text-xs">Cancel</span>
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={async () => {
                    // Handle bulk delete
                    const selectedCount = sidebarSnapshot.selectedConversations.size;
                    if (selectedCount === 0) {
                      alert("No conversations selected");
                      return;
                    }
                    
                    if (!confirm(`Are you sure you want to delete ${selectedCount} conversation${selectedCount > 1 ? 's' : ''}?`)) {
                      return;
                    }
                    
                    // Delete each selected conversation
                    const deletePromises = Array.from(sidebarSnapshot.selectedConversations).map(async (conversationId) => {
                      try {
                        const response = await api.fetchStream(`/conversations/${conversationId}`, {
                          method: "DELETE",
                        });
                        if (!response.ok) {
                          throw new Error("Failed to delete conversation");
                        }
                        // Remove from store
                        sidebarActions.removeConversation(conversationId);
                        // Clean up from conversation store
                        if (conversationStoreSnapshot.conversations[conversationId]) {
                          ConversationManager.getInstance().clearConversation(conversationId);
                        }
                        return { success: true, id: conversationId };
                      } catch (error) {
                        logger.error(`Failed to delete conversation ${conversationId}:`, error);
                        return { success: false, id: conversationId };
                      }
                    });
                    
                    const results = await Promise.all(deletePromises);
                    const failedCount = results.filter(r => !r.success).length;
                    
                    if (failedCount > 0) {
                      sidebarActions.setError(`Failed to delete ${failedCount} conversation${failedCount > 1 ? 's' : ''}`);
                    }
                    
                    // Check if current conversation was deleted
                    if (actualConversationId && sidebarSnapshot.selectedConversations.has(actualConversationId)) {
                      navigate("/projects");
                    }
                    
                    // Exit delete mode
                    sidebarActions.exitDeleteMode();
                  }}
                  className="gap-1 h-[25px] border border-transparent hover:border-red-500 hover:text-red-600"
                  title="Delete Selected"
                >
                  <Trash2 size={10} />
                  <span className="text-xs">Delete ({sidebarSnapshot.selectedConversations.size})</span>
                </Button>
              </div>
             ) : (
               <div className="flex items-center gap-1">
                 <Button
                   variant="ghost"
                   size="sm"
                   onClick={handleCreateNewConversation}
                   className="gap-1 h-[25px] border border-transparent hover:border-gray-200"
                   title="New Chat"
                 >
                   <MessageSquare size={10} />
                   <span className="text-xs">New</span>
                 </Button>
               </div>
             )}
          </div>
        </div>

        {/* Conversations area */}
        {(!isCollapsed || sidebarSnapshot.isMobileMenuOpen) && (
          <div className="flex-1 overflow-y-auto relative">
            {sidebarSnapshot.loading ? (
              <div className="p-4">
                <div className="animate-pulse">
                  <div className="h-4 bg-gray-200 rounded mb-2"></div>
                  <div className="h-4 bg-gray-200 rounded mb-2"></div>
                  <div className="h-4 bg-gray-200 rounded"></div>
                </div>
              </div>
            ) : sidebarSnapshot.error ? (
              <div className="p-4">
                <p className="text-sm text-red-500">{sidebarSnapshot.error}</p>
              </div>
            ) : sidebarSnapshot.conversations.length === 0 ? (
              <div className="p-4">
                <p className="text-xs text-muted-foreground">Ummm... </p>
                <p className="text-xs text-muted-foreground">Let's talk ? </p>
              </div>
            ) : (
              <div className="p-2 min-w-[130px] absolute inset-0 ">
                {sidebarSnapshot.conversations.map((conversation) => (
                  <div
                    key={conversation.id}
                     className={cn(
                       "block w-full group text-left p-2 rounded-md hover:bg-muted border border-transparent transition-colors mb-1 group cursor-pointer relative",
                       actualConversationId === conversation.id &&
                         "bg-muted border-blue-700/30",
                       sidebarSnapshot.isDeleteMode &&
                         sidebarSnapshot.selectedConversations.has(conversation.id) &&
                         "bg-red-50 dark:bg-red-900/20 border-red-500/30",
                       sidebarSnapshot.isDeleteMode && "hover:bg-red-50 dark:hover:bg-red-900/10"
                     )}
                    onClick={(e) => handleConversationClick(conversation.id, e)}
                    onMouseEnter={() => !sidebarSnapshot.isDeleteMode && handleConversationHover(conversation.id)}
                  >
                     <div
                       className={cn(
                         "flex items-start gap-2 overflow-hidden",
                         !sidebarSnapshot.isDeleteMode && "group-hover:pr-8"
                       )}
                     >
                       {/* Icon section - always on left */}
                       <div className="relative flex flex-col items-center pt-1">
                         {/* Check streaming state from conversationStore */}
                         {conversationStoreSnapshot.conversations[conversation.id]?.status === 'streaming' ? (
                           <Loader2 className="h-4 w-4 text-blue-500 animate-spin" />
                         ) : conversationStoreSnapshot.conversations[conversation.id]?.status === 'loading' ? (
                           <Loader2 className="h-4 w-4 text-muted-foreground animate-spin" />
                         ) : (
                           <MessageSquare
                             className={cn(
                               "h-4 w-4",
                               sidebarSnapshot.recentlyUpdatedConversations.has(conversation.id) &&
                               actualConversationId !== conversation.id
                                 ? "text-green-500"
                                 : "text-muted-foreground"
                             )}
                           />
                         )}
                         {/* Green notification dot for new messages in non-active conversations */}
                         {sidebarSnapshot.recentlyUpdatedConversations.has(conversation.id) &&
                          actualConversationId !== conversation.id && (
                           <div className="h-[6px] w-[6px] rounded-full bg-green-500 mt-1 animate-pulse" />
                         )}
                       </div>

                       {/* Content section - flexible width */}
                       <div className="flex-1 min-w-0">
                         <p
                           className={cn(
                             "text-sm font-medium truncate",
                             sidebarSnapshot.recentlyUpdatedConversations.has(conversation.id) &&
                             actualConversationId !== conversation.id &&
                               "text-green-500 font-semibold"
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

                       {/* Checkbox on the right in delete mode */}
                       {sidebarSnapshot.isDeleteMode && (
                         <div className="pt-1 flex-shrink-0">
                           <Checkbox
                             checked={sidebarSnapshot.selectedConversations.has(conversation.id)}
                             onCheckedChange={() => sidebarActions.toggleConversationSelection(conversation.id)}
                             onClick={(e: React.MouseEvent) => e.stopPropagation()}
                           />
                         </div>
                       )}
                     </div>

                    {/* Actions dropdown - hidden in delete mode */}
                    {!sidebarSnapshot.isDeleteMode && (
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
                               sidebarActions.enterDeleteMode(conversation.id);
                             }}
                             className="text-red-600 focus:text-red-600"
                           >
                             <Trash2 className="h-4 w-4 mr-2" />
                             Delete
                           </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </div>
                  )}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Bottom user section - visible on desktop and when mobile menu is open */}
        {(!isCollapsed || sidebarSnapshot.isMobileMenuOpen) && (
          <div className="border-t p-3 relative z-10">
            {isCollapsed && !sidebarSnapshot.isMobileMenuOpen ? (
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
                  onClick={handleLogout}
                  className="cursor-pointer"
                >
                  Sign out
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          )}
          </div>
        )}


        {/* Rename Dialog */}
        <Dialog open={sidebarSnapshot.renameDialogOpen} onOpenChange={(open) => !open && sidebarActions.closeRenameDialog()}>
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
                  value={sidebarSnapshot.newTitle}
                  onChange={(e) => sidebarActions.setNewTitle(e.target.value)}
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
                onClick={() => sidebarActions.closeRenameDialog()}
              >
                Cancel
              </Button>
              <Button
                type="button"
                onClick={handleRenameConversation}
                disabled={!sidebarSnapshot.newTitle.trim()}
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
        onClick={() => sidebarActions.toggleMobileMenu()}
        className="fixed top-4 left-4 z-40 h-10 w-10 p-0 md:hidden rounded-full shadow-lg bg-background border"
      >
        {sidebarSnapshot.isMobileMenuOpen ? (
          <PanelLeftClose className="h-5 w-5" />
        ) : (
          <PanelLeftOpen className="h-5 w-5" />
        )}
      </Button>
    </>
  );
}
