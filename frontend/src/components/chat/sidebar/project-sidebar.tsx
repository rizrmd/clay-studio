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
import { tabsActions } from "@/lib/store/tabs-store";
import { analysisStore, analysisActions, type Analysis } from "@/lib/store/analysis-store";
import { api } from "@/lib/utils/api";
import { chatStore } from "@/lib/store/chat/chat-store";
import { ConversationSidebarFooter } from "./components/footer";
import { ConversationSidebarHeader } from "./components/header";
import { ConversationList } from "./components/list";
import { DatasourceList } from "./components/datasource-list";
import { MobileMenuToggle } from "./components/toggle";
import { AnalysisList } from "./components/analysis-list";
import { useAuth } from "@/hooks/use-auth";
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
  const analysisSnapshot = useSnapshot(analysisStore);
  const { deleteConversation, bulkDeleteConversations } = useChat();
  const navigate = useNavigate();
  const chat = useChat();
  const params = useParams();
  const [searchParams] = useSearchParams();
  const location = useLocation();
  const { logout, user } = useAuth();

  const { datasourceId } = params;
  const analysisId = params.analysisId as string | undefined;
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

  // Sync active analysis with URL
  useEffect(() => {
    if (analysisId && analysisId !== analysisSnapshot.activeAnalysisId) {
      analysisActions.setActiveAnalysis(analysisId);
    }
  }, [analysisId]);

  // Check if we're on an analysis route
  const isOnAnalysisRoute = location.pathname.includes("/analysis");

  // Set accordion based on current route
  useEffect(() => {
    if (isOnAnalysisRoute) {
      sidebarActions.setAccordionValue(["analysis"]);
    } else if (isOnDatasourceBrowseRoute) {
      sidebarActions.setAccordionValue(["datasources"]);
    } else {
      sidebarActions.setAccordionValue(["conversations"]);
    }
  }, [isOnAnalysisRoute, isOnDatasourceBrowseRoute]);

  // Load datasources when on a datasource browse route (removed - consolidated below)

  // Handle project switch
  useEffect(() => {
    if (projectId && projectId !== sidebarSnapshot.currentProjectId) {
      // Switch sidebar state to the new project
      sidebarActions.switchToProject(projectId);
      
      // Clear old project data
      datasourcesActions.clearDatasources();
      analysisActions.clearAnalyses();
      
      // Clear conversations from chat store for clean slate
      chatStore.map = {};
      chatStore.list = [];
      chatStore.errors = {};
      chatStore.streaming = {};
      chatStore.loadingMessages = {};
    }
  }, [projectId]);

  // Always load conversations when we have a projectId (regardless of route)
  useEffect(() => {
    if (projectId) {

      // The conversations are managed by the chat hook and wsService
      // This should trigger the conversation list to load
      if (projectId !== chat.projectId) {
        chat.setProjectId(projectId);
      }

      // Force reload conversations for new project (removed the check for empty list)
      // Use REST API to load conversations
      const loadConversationsViaAPI = async () => {
        try {

          const response = await api.get(
            `/conversations?project_id=${projectId}`
          );

          // Check if response is valid
          if (!Array.isArray(response)) {
            console.error(
              "ProjectSidebar: API response is not an array:",
              response
            );

            // Fallback to WebSocket
            setTimeout(() => {
              chat.listConversations();
            }, 500);
            return;
          }

          // Update chat store directly - preserve existing conversation data with messages
          const existingConversations = { ...chatStore.map };
          
          // Sort conversations by updated_at in descending order (newest first)
          const sortedConversations = response.sort((a: any, b: any) => 
            new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
          );
          chatStore.list = sortedConversations.map((c: any) => c.id);
          
          // Clear and rebuild the map, but preserve loaded messages
          chatStore.map = {};
          sortedConversations.forEach((conversation: any) => {
            if (existingConversations[conversation.id] && existingConversations[conversation.id].messages) {
              // Keep the existing conversation with its loaded messages
              chatStore.map[conversation.id] = {
                ...conversation,
                messages: existingConversations[conversation.id].messages
              };
            } else {
              // Use the conversation from API (no messages loaded yet)
              chatStore.map[conversation.id] = conversation;
            }
          });

        } catch (error) {
          console.error(
            "ProjectSidebar: Failed to load conversations via API:",
            error
          );

          // Fallback to WebSocket on error
          setTimeout(() => {
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

  const handleBulkDeleteAnalyses = useCallback(async () => {
    if (sidebarSnapshot.selectedAnalyses.length === 0) return;

    if (!confirm(`Are you sure you want to delete ${sidebarSnapshot.selectedAnalyses.length} analysis/analyses? There is no undo.`)) {
      return;
    }

    try {
      const { analysisApi } = await import("@/lib/api/analysis");
      const { tabsStore } = await import("@/lib/store/tabs-store");

      // Delete each analysis
      for (const analysisId of sidebarSnapshot.selectedAnalyses) {
        await analysisApi.deleteAnalysis(analysisId);

        // Remove from store
        analysisActions.removeAnalysis(analysisId);

        // Close the tab if it's open
        const analysisTab = tabsStore.tabs.find(
          t => t.type === 'analysis' && t.metadata.analysisId === analysisId
        );
        if (analysisTab) {
          tabsActions.removeTab(analysisTab.id);
        }
      }

      // Exit delete mode
      sidebarActions.exitAnalysisDeleteMode();
    } catch (error) {
      console.error('Failed to delete analyses:', error);
      alert('Failed to delete some analyses');
    }
  }, [sidebarSnapshot.selectedAnalyses]);

  const openRenameDialog = (conversation: any) => {
    // Set the conversation as selected for renaming
    sidebarActions.clearSelection();
    sidebarActions.addToSelection(conversation);
  };

  const handleDeleteConversation = (conversationId: string) => {
    // Delete single conversation
    deleteConversation(conversationId);
  };

  const handleLogout = async () => {
    await logout();
    navigate("/auth/signin");
  };

  const handleProfile = () => {
    navigate("/profile");
  };

  // Load datasources when needed (either on datasource routes or when sidebar is expanded)
  useEffect(() => {
    if (projectId && (isOnDatasourceBrowseRoute || !isCollapsed)) {
      // Always load datasources when project changes or when needed
      datasourcesActions.loadDatasources(projectId);
    }
  }, [projectId]);

  // Load analyses when needed (only on project change or sidebar expand)
  useEffect(() => {
    if (projectId && !isCollapsed) {
      // Only load if we don't have analyses yet or if loading hasn't started
      if (analysisSnapshot.analyses.length === 0 && !analysisSnapshot.isLoading) {
        const loadAnalyses = async () => {
          analysisActions.setLoading(true);
          try {
            const response = await api.get(`/projects/${projectId}/analysis`);
            analysisActions.setAnalyses(response);
          } catch (error) {
            console.error('Failed to load analyses:', error);
            analysisActions.setError('Failed to load analyses');
          } finally {
            analysisActions.setLoading(false);
          }
        };
        loadAnalyses();
      }
    }
  }, [projectId, isCollapsed]);

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

  const handleQueryClick = (datasourceId: string) => {
    sidebarActions.selectDatasource(datasourceId);
    sidebarActions.setMobileMenuOpen(false);
    if (projectId) {
      // Find the datasource to get its name
      const datasource = datasourcesSnapshot.datasources.find(ds => ds.id === datasourceId);
      const tabTitle = datasource ? `Query: ${datasource.name}` : 'Query Editor';
      
      // Create or activate a query tab
      tabsActions.getOrCreateActiveTab('datasource_query', {
        datasourceId,
        projectId,
      }, tabTitle);
      
      // Navigate to the query editor
      navigate(`/p/${projectId}/datasources/${datasourceId}/query`);
    }
  };

  const handleEditClick = (datasourceId: string) => {
    sidebarActions.selectDatasource(datasourceId);
    sidebarActions.setMobileMenuOpen(false);
    if (projectId) {
      // Find the datasource to get its name
      const datasource = datasourcesSnapshot.datasources.find(ds => ds.id === datasourceId);
      const tabTitle = datasource ? `${datasource.name} - Edit` : 'Edit Datasource';
      
      // Create or activate an edit tab
      tabsActions.getOrCreateActiveTab('datasource_edit', {
        datasourceId,
        projectId,
      }, tabTitle);
      
      // Navigate to the edit datasource page
      navigate(`/p/${projectId}/datasources/${datasourceId}/edit`);
    }
  };

  const handleAnalysisClick = (analysisId: string) => {
    sidebarActions.setMobileMenuOpen(false);
    if (projectId) {
      // Find the analysis to get its name
      const analysis = analysisSnapshot.analyses?.find(a => a.id === analysisId);
      const tabTitle = analysis ? (analysis.name || 'Analysis') : 'Analysis';

      // Set active analysis
      analysisActions.setActiveAnalysis(analysisId);

      // Create or activate an analysis tab
      tabsActions.getOrCreateActiveTab('analysis', {
        analysisId,
        analysisTitle: tabTitle,
        projectId,
      }, tabTitle);

      // Navigate to the analysis page
      navigate(`/p/${projectId}/analysis/${analysisId}`);
    }
  };

  const handleRunAnalysis = async (analysisId: string) => {
    sidebarActions.setMobileMenuOpen(false);
    if (projectId) {
      // Find the analysis to get its details
      const analysis = analysisSnapshot.analyses?.find(a => a.id === analysisId);
      if (!analysis) return;
      
      // Execute the analysis
      analysisActions.setActiveAnalysis(analysisId);
      
      try {
        // Call API to execute the analysis
        const response = await api.post(`/analysis/${analysisId}/execute`, {
          project_id: projectId,
        });
        
        // Update the job status
        analysisActions.updateJob(analysisId, response.data.job);
        
        // Navigate to results view
        navigate(`/p/${projectId}/analysis/${analysisId}/results`);
      } catch (error) {
        console.error('Failed to run analysis:', error);
        analysisActions.setError('Failed to run analysis');
      }
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
          currentUserId={user?.id}
          onBulkDelete={handleBulkDelete}
          onBulkDeleteAnalyses={handleBulkDeleteAnalyses}
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
                  <div className="flex items-center justify-between w-full">
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
                    projectId={projectId}
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
                    projectId={projectId}
                    onDatasourceClick={handleDatasourceClick}
                    onTableClick={handleTableClick}
                    onQueryClick={handleQueryClick}
                    onEditClick={handleEditClick}
                    activeDatasourceId={
                      uiSnapshot.currentDatasource || undefined
                    }
                    activeTableName={uiSnapshot.currentTable || undefined}
                  />
                </AccordionContent>
              </AccordionItem>

              <AccordionItem
                value="analysis"
                className="flex flex-col data-[state=open]:flex-1"
              >
                <AccordionTrigger className="py-2 px-3 hover:no-underline hover:bg-accent/50 flex-shrink-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium">Analysis</span>
                    <Badge variant="secondary" className="text-xs px-1.5 py-0">
                      {analysisSnapshot.isLoading
                        ? "..."
                        : analysisSnapshot.analyses?.length || 0}
                    </Badge>
                  </div>
                </AccordionTrigger>
                <AccordionContent className="!p-0 flex flex-col flex-1 min-h-0 h-full">
                  <AnalysisList
                    analyses={(analysisSnapshot.analyses || []) as Analysis[]}
                    onAnalysisClick={handleAnalysisClick}
                    onRunAnalysis={handleRunAnalysis}
                    onAddNew={() => {
                      // Navigate to chat to create a new analysis
                      navigate(`/p/${projectId}/new`);
                      // TODO: Open analysis creation dialog or set focus to chat input
                    }}
                    activeAnalysisId={analysisSnapshot.activeAnalysisId || undefined}
                    projectId={projectId}
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
