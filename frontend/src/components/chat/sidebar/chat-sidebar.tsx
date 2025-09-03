import { useEffect, useRef, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useSnapshot } from "valtio";
import { cn } from "@/lib/utils";
import { logger } from "@/lib/utils/logger";
import { useAuth } from "@/hooks/use-auth";
import { conversationStore } from "@/store/chat/conversation-store";
import { sidebarStore, sidebarActions } from "@/store/sidebar-store";
import { MessageCacheService } from "@/lib/services/chat/message-cache";
import { useConversations } from "./hooks/conversations";
import { useConversationEvents } from "./hooks/events";
import { useDeleteMode } from "./hooks/delete";
import { useRenameDialog } from "./hooks/rename";
import { ConversationSidebarHeader } from "./components/header";
import { ConversationList } from "./components/list";
import { ConversationSidebarFooter } from "./components/footer";
import { RenameConversationDialog } from "./components/rename";
import { MobileMenuToggle } from "./components/toggle";

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
  const { logout } = useAuth();
  const navigate = useNavigate();
  const lastMessageCountRef = useRef<Record<string, number>>({});
  const messageCache = MessageCacheService.getInstance();
  const prefetchedConversations = useRef<Set<string>>(new Set());

  // Use extracted hooks
  useConversations(projectId, currentConversationId);
  useConversationEvents(projectId);
  const { handleBulkDelete } = useDeleteMode(
    projectId,
    actualConversationId,
    navigate
  );
  const { handleRenameConversation, openRenameDialog } = useRenameDialog();

  // Track new messages in non-active conversations
  useEffect(() => {
    // Check all conversations for new messages
    Object.keys(conversationStoreSnapshot.conversations).forEach(
      (conversationId) => {
        const state = conversationStoreSnapshot.conversations[conversationId];
        if (!state) return;

        // Count messages in this conversation
        const currentMessageCount = state.messages?.length || 0;
        const previousMessageCount =
          lastMessageCountRef.current[conversationId];

        // Only mark as updated if:
        // 1. We've seen this conversation before (not undefined)
        // 2. It's not the active conversation
        // 3. It has new messages
        // 4. The conversation is currently streaming (real-time update) or
        //    the last message is recent (within last 10 seconds)
        if (
          previousMessageCount !== undefined &&
          conversationId !== actualConversationId &&
          currentMessageCount > previousMessageCount
        ) {
          // Check if this is a real-time update or just loading old messages
          const lastMessage = state.messages[state.messages.length - 1];
          const isRealTimeUpdate =
            state.status === "streaming" ||
            (lastMessage &&
              lastMessage.createdAt &&
              new Date().getTime() - new Date(lastMessage.createdAt).getTime() <
                10000);

          if (isRealTimeUpdate) {
            sidebarActions.addRecentlyUpdated(conversationId);
          }
        }

        // Update the count
        lastMessageCountRef.current[conversationId] = currentMessageCount;
      }
    );
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
    navigate(`/p/${projectId}/c/${conversationId}`);

    // Close mobile menu after navigation
    sidebarActions.setMobileMenuOpen(false);
  };

  // Prefetch conversation on hover for instant switching
  const handleConversationHover = useCallback(
    (conversationId: string) => {
      // Skip if already prefetched or if it's the current conversation
      if (
        prefetchedConversations.current.has(conversationId) ||
        conversationId === actualConversationId
      ) {
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
    },
    [actualConversationId, messageCache]
  );

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

  // Handle creating new conversation
  const handleCreateNewConversation = async () => {
    if (!projectId) return;
    navigate(`/p/${projectId}/new`);

    // try {
    //   // Create real conversation immediately
    //   const response = await fetch("/api/conversations", {
    //     method: "POST",
    //     headers: {
    //       "Content-Type": "application/json",
    //     },
    //     body: JSON.stringify({
    //       project_id: projectId,
    //     }),
    //   });

    //   if (!response.ok) {
    //     throw new Error("Failed to create new conversation");
    //   }

    //   const newConversation = await response.json();

    //   // Navigate directly to the real conversation

    //   // Close mobile menu after navigation
    //   sidebarActions.setMobileMenuOpen(false);
    // } catch (error) {
    //   console.error("Failed to create new conversation:", error);
    //   // Fallback to old behavior
    //   navigate(`/p/${projectId}/new`);
    //   sidebarActions.setMobileMenuOpen(false);
    // }
  };

  // Handle delete conversation
  const handleDeleteConversation = (conversationId: string) => {
    sidebarActions.enterDeleteMode(conversationId);
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
          !sidebarSnapshot.isMobileMenuOpen &&
            "overflow-hidden md:overflow-visible"
        )}
      >
        {/* Header */}
        <ConversationSidebarHeader
          onCreateNewConversation={handleCreateNewConversation}
          onNavigateToProjects={() => {
            navigate("/projects");
            sidebarActions.setMobileMenuOpen(false);
          }}
          projectId={projectId}
          onBulkDelete={handleBulkDelete}
        />

        {/* Conversations area */}
        {(!isCollapsed || sidebarSnapshot.isMobileMenuOpen) && (
          <div className="flex-1 overflow-y-auto relative">
            <ConversationList
              conversations={sidebarSnapshot.conversations as any}
              currentConversationId={actualConversationId}
              onConversationClick={handleConversationClick}
              onConversationHover={handleConversationHover}
              onRenameConversation={openRenameDialog}
              onDeleteConversation={handleDeleteConversation}
            />
          </div>
        )}

        {/* Bottom user section */}
        {(!isCollapsed || sidebarSnapshot.isMobileMenuOpen) && (
          <div className="border-t p-3 relative z-10">
            <ConversationSidebarFooter
              isCollapsed={isCollapsed}
              onLogout={handleLogout}
              onProfile={handleProfile}
            />
          </div>
        )}

        {/* Rename Dialog */}
        <RenameConversationDialog onRename={handleRenameConversation} />
      </div>

      {/* Mobile menu toggle button */}
      <MobileMenuToggle />
    </>
  );
}
