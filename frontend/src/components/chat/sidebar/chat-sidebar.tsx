import { useChat } from "@/lib/hooks/use-chat";
import { sidebarActions, sidebarStore } from "@/lib/store/chat/sidebar-store";
import { cn } from "@/lib/utils";
import { useCallback } from "react";
import { useSnapshot } from "valtio";
import { useNavigate } from "react-router-dom";
import { Database } from "lucide-react";
import { ConversationSidebarFooter } from "./components/footer";
import { ConversationSidebarHeader } from "./components/header";
import { ConversationList } from "./components/list";
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
  projectId,
  currentConversationId,
  onConversationSelect,
}: ConversationSidebarProps) {
  const sidebarSnapshot = useSnapshot(sidebarStore);
  const { deleteConversation, bulkDeleteConversations } = useChat();
  const navigate = useNavigate();

  const handleConversationClick = (conversationId: string) => {
    sidebarActions.setMobileMenuOpen(false);
    onConversationSelect?.(conversationId);
  };

  const handleBulkDelete = useCallback(() => {
    if (sidebarSnapshot.selectedConversations.length > 0) {
      if (confirm("Are you sure? there is no undo:")) {
        bulkDeleteConversations([...sidebarSnapshot.selectedConversations]);
      }
      // Note: exitDeleteMode() will be called automatically when server responds with conversations_bulk_deleted
    }
  }, [sidebarSnapshot.selectedConversations]);

  const openRenameDialog = (conversation: any) => {
    // Set the conversation as selected for renaming
    sidebarActions.clearSelection();
    sidebarActions.addToSelection(conversation);
  };

  const handleDeleteConversation = (conversationId: string) => {
    // Delete single conversation
    deleteConversation(conversationId);
  };

  const handleLogout = () => {
    // Implementation for logout
  };

  const handleProfile = () => {
    // Implementation for profile
  };

  const handleDatasourcesClick = () => {
    if (projectId) {
      navigate(`/p/${projectId}/datasources`);
    }
    sidebarActions.setMobileMenuOpen(false);
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
          onNavigateToProjects={() => {
            sidebarActions.setMobileMenuOpen(false);
          }}
          projectId={projectId}
          onBulkDelete={handleBulkDelete}
        />

        {/* Conversations area */}
        {(!isCollapsed || sidebarSnapshot.isMobileMenuOpen) && (
          <div className="flex-1 overflow-y-auto relative">
            <ConversationList
              currentConversationId={currentConversationId}
              onConversationClick={handleConversationClick}
              onRenameConversation={openRenameDialog}
              onDeleteConversation={handleDeleteConversation}
            />
          </div>
        )}

        {/* Datasources button */}
        {(!isCollapsed || sidebarSnapshot.isMobileMenuOpen) && (
          <div className="border-t">
            <button
              onClick={handleDatasourcesClick}
              className="w-full p-2 px-5 hover:bg-accent rounded-none flex items-center gap-2"
            >
              <Database className="h-4 w-4" />
              <span className="text-sm">Datasources</span>
            </button>
          </div>
        )}

        {/* For collapsed state - datasources icon only */}
        {isCollapsed && !sidebarSnapshot.isMobileMenuOpen && (
          <div className="border-t">
            <button
              onClick={handleDatasourcesClick}
              className="h-8 w-8 p-0 hover:bg-accent rounded-md flex items-center justify-center"
            >
              <Database className="h-4 w-4" />
            </button>
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
        {/* <RenameConversationDialog onRename={handleRenameConversation} /> */}
      </div>

      {/* Mobile menu toggle button */}
      <MobileMenuToggle />
    </>
  );
}
