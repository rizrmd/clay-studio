import { useChat } from "@/lib/hooks/use-chat";
import { sidebarActions, sidebarStore } from "@/lib/store/chat/sidebar-store";
import { datasourcesStore, datasourcesActions } from "@/lib/store/datasources-store";
import { cn } from "@/lib/utils";
import { useCallback, useEffect } from "react";
import { useSnapshot } from "valtio";
import { useNavigate } from "react-router-dom";
import { ConversationSidebarFooter } from "./components/footer";
import { ConversationSidebarHeader } from "./components/header";
import { ConversationList } from "./components/list";
import { DatasourceList } from "./components/datasource-list";
import { AccordionSection } from "./components/accordion-section";
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
  const datasourcesSnapshot = useSnapshot(datasourcesStore);
  const { deleteConversation, bulkDeleteConversations } = useChat();
  const navigate = useNavigate();
  const chat = useChat();

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

  // Load datasources when component mounts or projectId changes
  useEffect(() => {
    if (projectId && !isCollapsed) {
      datasourcesActions.loadDatasources(projectId);
    }
  }, [projectId, isCollapsed]);


  const handleDatasourceClick = (datasourceId: string) => {
    sidebarActions.selectDatasource(datasourceId);
    sidebarActions.setMobileMenuOpen(false);
    // TODO: Navigate to data browser or open data browser modal
    console.log('Open data browser for datasource:', datasourceId);
  };

  const handleAddDatasource = () => {
    if (projectId) {
      navigate(`/p/${projectId}/datasources`);
    }
    sidebarActions.setMobileMenuOpen(false);
  };

  const handleAddConversation = () => {
    // TODO: Create new conversation
    console.log('Create new conversation');
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

        {/* Accordion content */}
        {(!isCollapsed || sidebarSnapshot.isMobileMenuOpen) && (
          <div className="flex-1 overflow-y-auto">
            <div className="group">
              <AccordionSection
                title="Conversations"
                isOpen={sidebarSnapshot.accordionValue.includes("conversations")}
                onToggle={() => sidebarActions.toggleAccordionSection("conversations")}
                count={chat.conversationList?.length || 0}
                onAdd={handleAddConversation}
                addButtonTitle="New conversation"
                isCollapsed={isCollapsed}
              >
                <ConversationList
                  currentConversationId={currentConversationId}
                  onConversationClick={handleConversationClick}
                  onRenameConversation={openRenameDialog}
                  onDeleteConversation={handleDeleteConversation}
                />
              </AccordionSection>

              <AccordionSection
                title="Datasources"
                isOpen={sidebarSnapshot.accordionValue.includes("datasources")}
                onToggle={() => sidebarActions.toggleAccordionSection("datasources")}
                count={datasourcesSnapshot.datasources?.length || 0}
                onAdd={handleAddDatasource}
                addButtonTitle="Add datasource"
                isCollapsed={isCollapsed}
              >
                <DatasourceList
                  projectId={projectId}
                  onDatasourceClick={handleDatasourceClick}
                />
              </AccordionSection>
            </div>
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
