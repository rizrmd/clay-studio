import { useChat } from "@/lib/hooks/use-chat";
import { sidebarActions, sidebarStore } from "@/lib/store/chat/sidebar-store";
import {
  datasourcesStore,
  datasourcesActions,
} from "@/lib/store/datasources-store";
import { cn } from "@/lib/utils";
import { useCallback, useEffect } from "react";
import { useSnapshot } from "valtio";
import {
  useNavigate,
  useParams,
  useSearchParams,
  useLocation,
} from "react-router-dom";
import { uiStore, uiActions } from "@/lib/store/chat/ui-store";
import { api } from "@/lib/utils/api";
import { chatStore } from "@/lib/store/chat/chat-store";
import { ConversationSidebarFooter } from "./components/footer";
import { ConversationSidebarHeader } from "./components/header";
import { ConversationList } from "./components/list";
import { DatasourceList } from "./components/datasource-list";
import { MobileMenuToggle } from "./components/toggle";
import {
  Accordion,
  AccordionItem,
  AccordionTrigger,
  AccordionContent,
} from "@/components/ui/accordion";
import { Badge } from "@/components/ui/badge";

interface ProjectSidebarProps {
  isCollapsed: boolean;
  onToggle: () => void;
  projectId?: string;
  currentConversationId?: string;
  onConversationSelect?: (conversationId: string) => void;
}

export function ProjectSidebar({
  isCollapsed,
  projectId,
  currentConversationId,
  onConversationSelect,
}: ProjectSidebarProps) {
  const sidebarSnapshot = useSnapshot(sidebarStore);
  const datasourcesSnapshot = useSnapshot(datasourcesStore);
  const uiSnapshot = useSnapshot(uiStore);
  const { deleteConversation, bulkDeleteConversations } = useChat();
  const navigate = useNavigate();
  const chat = useChat();
  const params = useParams();
  const [searchParams] = useSearchParams();
  const location = useLocation();

  const { datasourceId } = params;
  const tableFromUrl = searchParams.get("table");

  // Check if we're on a datasource browse route (not just the datasources list page)
  const isOnDatasourceBrowseRoute =
    location.pathname.includes("/datasources/") &&
    location.pathname.includes("/browse");

  // Update uiStore when URL params change
  useEffect(() => {
    uiActions.setCurrentDatasource(datasourceId || null);
  }, [datasourceId]);

  useEffect(() => {
    uiActions.setCurrentTable(tableFromUrl);
  }, [tableFromUrl]);

  // Set accordion to show datasources when on a datasource browse route
  useEffect(() => {
    if (isOnDatasourceBrowseRoute) {
      sidebarActions.setAccordionValue(["datasources"]);
    } else {
      sidebarActions.setAccordionValue(["conversations"]);
    }
  }, [isOnDatasourceBrowseRoute]);

  // Load datasources when on a datasource browse route (removed - consolidated below)

  // Always load conversations when we have a projectId (regardless of route)
  useEffect(() => {
    if (projectId) {
      console.log("ProjectSidebar: Loading conversations", {
        projectId,
        chatProjectId: chat.projectId,
        conversationListLength: chat.conversationList?.length,
        isConnected: chat.isConnected,
      });

      // The conversations are managed by the chat hook and wsService
      // This should trigger the conversation list to load
      if (projectId !== chat.projectId) {
        console.log("ProjectSidebar: Setting project ID");
        chat.setProjectId(projectId);
      }

      // Use REST API to load conversations
      const loadConversationsViaAPI = async () => {
        try {
          console.log(
            "ProjectSidebar: Loading conversations via API for project:",
            projectId
          );

          const response = await api.get(
            `/conversations?project_id=${projectId}`
          );
          console.log("ProjectSidebar: API client response:", {
            response,
            type: typeof response,
            isArray: Array.isArray(response),
            length: Array.isArray(response) ? response.length : "not array",
          });

          // Check if response is valid
          if (!Array.isArray(response)) {
            console.error(
              "ProjectSidebar: API response is not an array:",
              response
            );
            console.log("ProjectSidebar: Falling back to WebSocket approach");

            // Fallback to WebSocket
            setTimeout(() => {
              console.log(
                "ProjectSidebar: WebSocket fallback - calling listConversations"
              );
              chat.listConversations();
            }, 500);
            return;
          }

          // Update chat store directly
          chatStore.map = {};
          chatStore.list = response.map((c: any) => c.id);
          response.forEach((conversation: any) => {
            chatStore.map[conversation.id] = conversation;
          });

          console.log(
            "ProjectSidebar: Chat store updated with",
            response.length,
            "conversations"
          );
        } catch (error) {
          console.error(
            "ProjectSidebar: Failed to load conversations via API:",
            error
          );

          // Fallback to WebSocket on error
          setTimeout(() => {
            console.log("ProjectSidebar: WebSocket fallback after API error");
            chat.listConversations();
          }, 500);
        }
      };

      loadConversationsViaAPI();
    }
  }, [projectId]);

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

  // Load datasources when needed (either on datasource routes or when sidebar is expanded)
  useEffect(() => {
    if (projectId && (isOnDatasourceBrowseRoute || !isCollapsed)) {
      // Only load if not already loaded or if it's a different project
      if (!datasourcesSnapshot.datasources.length) {
        datasourcesActions.loadDatasources(projectId);
      }
    }
  }, [projectId]);

  const handleDatasourceClick = (datasourceId: string) => {
    sidebarActions.selectDatasource(datasourceId);
    sidebarActions.setMobileMenuOpen(false);
    if (projectId) {
      navigate(`/p/${projectId}/datasources/${datasourceId}/browse`);
    }
  };

  const handleTableClick = (datasourceId: string, tableName: string) => {
    sidebarActions.selectDatasource(datasourceId);
    sidebarActions.setMobileMenuOpen(false);
    if (projectId) {
      navigate(
        `/p/${projectId}/datasources/${datasourceId}/browse?table=${encodeURIComponent(
          tableName
        )}`
      );
    }
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
          <div className="flex-1 flex flex-col overflow-hidden">
            <Accordion
              type="single"
              value={sidebarSnapshot.accordionValue[0] || ""}
              onValueChange={(value) =>
                sidebarActions.setAccordionValue(value ? [value] : [])
              }
              className="flex-1 flex flex-col"
            >
              <AccordionItem
                value="conversations"
                className="flex flex-col data-[state=open]:flex-1"
              >
                <AccordionTrigger className="py-2 px-3 hover:no-underline hover:bg-accent/50 flex-shrink-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium">Conversations</span>
                    <Badge variant="secondary" className="text-xs px-1.5 py-0">
                      {typeof chat.conversationList?.length === "number"
                        ? chat.conversationList?.length
                        : "..."}
                    </Badge>
                  </div>
                </AccordionTrigger>
                <AccordionContent className="!p-0 flex flex-col flex-1 min-h-0">
                  <ConversationList
                    currentConversationId={currentConversationId}
                    onConversationClick={handleConversationClick}
                    onRenameConversation={openRenameDialog}
                    onDeleteConversation={handleDeleteConversation}
                  />
                </AccordionContent>
              </AccordionItem>

              <AccordionItem
                value="datasources"
                className="flex flex-col data-[state=open]:flex-1"
              >
                <AccordionTrigger className="py-2 px-3 hover:no-underline hover:bg-accent/50 flex-shrink-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium">Datasources</span>
                    <Badge variant="secondary" className="text-xs px-1.5 py-0">
                      {datasourcesSnapshot.isLoading
                        ? "..."
                        : datasourcesSnapshot.datasources?.length || 0}
                    </Badge>
                  </div>
                </AccordionTrigger>
                <AccordionContent className="!p-0 flex flex-col flex-1 min-h-0 h-full">
                  <DatasourceList
                    onDatasourceClick={handleDatasourceClick}
                    onTableClick={handleTableClick}
                    activeDatasourceId={
                      uiSnapshot.currentDatasource || undefined
                    }
                    activeTableName={uiSnapshot.currentTable || undefined}
                  />
                </AccordionContent>
              </AccordionItem>
            </Accordion>
          </div>
        )}

        {/* Bottom user section */}
        {(!isCollapsed || sidebarSnapshot.isMobileMenuOpen) && (
          <div className="relative z-10">
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
