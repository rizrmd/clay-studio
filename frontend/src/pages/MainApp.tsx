import { lazy, Suspense, useEffect } from "react";
// Lazy load major components for better code splitting
const Chat = lazy(() =>
  import("@/components/chat").then((m) => ({ default: m.Chat }))
);
const ProjectSidebar = lazy(() =>
  import("@/components/chat").then((m) => ({ default: m.ProjectSidebar }))
);
const NewChat = lazy(() =>
  import("@/components/chat/main/new-chat").then((m) => ({
    default: m.NewChat,
  }))
);
const DatasourcesMain = lazy(() =>
  import("@/components/datasources/datasources-main").then((m) => ({
    default: m.DatasourcesMain,
  }))
);
const DataBrowserInlineEditing = lazy(() =>
  import("@/components/datasources/browser/data-browser").then((m) => ({
    default: m.DataBrowser,
  }))
);
const QueryEditor = lazy(() =>
  import("@/components/datasources/browser/query-editor").then((m) => ({
    default: m.QueryEditor,
  }))
);
const AnalysisEditor = lazy(() =>
  import("@/components/analysis/analysis-editor").then((m) => ({
    default: m.AnalysisEditor,
  }))
);
const AnalysisList = lazy(() =>
  import("@/components/analysis/analysis-list").then((m) => ({
    default: m.AnalysisList,
  }))
);
const ContextEditor = lazy(() =>
  import("@/components/projects/context-editor").then((m) => ({
    default: m.ContextEditor,
  }))
);
import { useParams, useNavigate, useLocation } from "react-router-dom";
import { useSnapshot } from "valtio";
import { uiStore, uiActions } from "@/lib/store/chat/ui-store";
import { tabsStore, tabsActions } from "@/lib/store/tabs-store";
import { dataBrowserStore } from "@/lib/store/data-browser-store";
import { chatStore } from "@/lib/store/chat/chat-store";
import { api } from "@/lib/utils/api";
import { useChat } from "@/lib/hooks/use-chat";
import { wsService } from "@/lib/services/ws-service";
import { TabBar } from "@/components/layout/tab-bar";

// Stub implementations
const useLoggerDebug = () => ({ isDebugMode: false });

export function MainApp() {
  const uiSnapshot = useSnapshot(uiStore);
  const tabsSnapshot = useSnapshot(tabsStore);
  const dataBrowserSnapshot = useSnapshot(dataBrowserStore);
  const chatSnapshot = useSnapshot(chatStore);
  const { projectId, conversationId, datasourceId, analysisId } = useParams<{
    projectId: string;
    conversationId?: string;
    datasourceId?: string;
    analysisId?: string;
  }>();
  const navigate = useNavigate();
  const location = useLocation();
  const chat = useChat();

  // Check if we're on the new conversation route
  const isNewRoute = location.pathname.endsWith("/new");
  // Check if we're on the data browser route
  const isDataBrowserRoute =
    location.pathname.includes("/datasources/") &&
    location.pathname.endsWith("/browse");
  // Check if we're on the query editor route
  const isQueryEditorRoute =
    location.pathname.includes("/datasources/") &&
    location.pathname.endsWith("/query");
  // Check if we're on the edit datasource route
  const isDatasourceEditRoute =
    location.pathname.includes("/datasources/") &&
    location.pathname.endsWith("/edit");
  // Check if we're on the new datasource route
  const isDatasourceNewRoute = location.pathname.endsWith("/datasources/new");
  // Check if we're on the analysis list route
  const isAnalysisListRoute = location.pathname.endsWith("/analysis");
  // Check if we're on the new analysis route
  const isAnalysisNewRoute = location.pathname.includes("/analysis/new");
  // Check if we're on the analysis view/edit route
  const isAnalysisViewRoute = location.pathname.includes("/analysis/") && !isAnalysisNewRoute;
  // Check if we're on the context route
  const isContextRoute = location.pathname.endsWith("/context");

  // Enable debug logging hooks
  useLoggerDebug();

  // Update valtio store with current route params
  useEffect(() => {
    if (projectId) {
      // Check if project has changed
      const previousProjectId = uiSnapshot.currentProject;
      if (previousProjectId !== projectId) {
        // Switch to the new project's tabs
        tabsActions.switchToProject(projectId);
      }
      uiActions.setCurrentProject(projectId);
    }
    if (conversationId) {
      uiActions.setCurrentConversation(conversationId);
      // Also update the chat store's active conversation
      chat.setConversationId(conversationId);
    }
  }, [projectId, conversationId, location.state]);


  // Handle redirection when visiting /p/:projectId without conversation ID
  // Don't redirect if we're on the /new route or data browser route or query editor route
  useEffect(() => {
    if (
      projectId &&
      !conversationId &&
      !isNewRoute &&
      !isDataBrowserRoute &&
      !isQueryEditorRoute &&
      !isDatasourceEditRoute &&
      !isDatasourceNewRoute &&
      !isAnalysisListRoute &&
      !isAnalysisNewRoute &&
      !isAnalysisViewRoute &&
      !isContextRoute
    ) {
      // Try to get the last conversation from localStorage
      const lastConversationKey = `last_conversation_${projectId}`;
      const lastConversationId = localStorage.getItem(lastConversationKey);

      if (lastConversationId) {
        // Redirect to the last conversation
        navigate(`/p/${projectId}/c/${lastConversationId}`, { replace: true });
      } else {
        // No last conversation, create a new one immediately
        createNewConversationAndRedirect();
      }
    }
  }, [
    projectId,
    conversationId,
    navigate,
    isDataBrowserRoute,
    isQueryEditorRoute,
    isDatasourceEditRoute,
    isDatasourceNewRoute,
    isAnalysisListRoute,
    isAnalysisNewRoute,
    isAnalysisViewRoute,
  ]);

  useEffect(() => {
    if (projectId) {
      if (projectId !== chat.projectId) {
        chat.setProjectId(projectId);
        // Note: Conversation loading is now handled by ProjectSidebar component
      }
    }
  }, [projectId]);

  // Auto-subscribe to conversation when project and conversation IDs are available
  useEffect(() => {
    if (projectId && conversationId && conversationId !== "new") {
      // Only subscribe if we're not already subscribed to this project/conversation
      if (!wsService.isSubscribed(projectId, conversationId)) {
        // Mark as loading messages
        chatStore.loadingMessages[conversationId] = true;
        wsService.subscribe(projectId, conversationId);
        // Request conversation messages after subscribing
        wsService.getConversationMessages(conversationId);
      }
    }
  }, [projectId, conversationId]);

  // Update tab title when selected table changes
  useEffect(() => {
    const activeTab = tabsSnapshot.tabs.find(
      (t) => t.id === tabsSnapshot.activeTabId
    );
    if (
      activeTab &&
      (activeTab.type === "datasource_table_data" ||
        activeTab.type === "datasource_table_structure") &&
      activeTab.metadata.datasourceId === datasourceId &&
      dataBrowserSnapshot.selectedTable &&
      dataBrowserSnapshot.selectedTable !== activeTab.metadata.tableName
    ) {
      tabsActions.updateTab(activeTab.id, {
        title: dataBrowserSnapshot.selectedTable,
        metadata: {
          ...activeTab.metadata,
          tableName: dataBrowserSnapshot.selectedTable,
        },
      });
    }
  }, [
    dataBrowserSnapshot.selectedTable,
    tabsSnapshot.activeTabId,
    datasourceId,
  ]);

  // Update chat tab title when conversation title changes
  useEffect(() => {
    if (conversationId && conversationId !== "new") {
      const conversation = chatSnapshot.map[conversationId];
      const activeTab = tabsSnapshot.tabs.find(
        (t) => t.id === tabsSnapshot.activeTabId
      );

      if (
        conversation &&
        activeTab &&
        activeTab.type === "chat" &&
        activeTab.metadata.conversationId === conversationId &&
        activeTab.title !== conversation.title
      ) {
        tabsActions.updateTab(activeTab.id, {
          title: conversation.title,
          metadata: {
            ...activeTab.metadata,
            conversationTitle: conversation.title,
          },
        });
      }
    }
  }, [chatSnapshot.map, conversationId, tabsSnapshot.activeTabId]);

  // Handle tab creation/updates based on URL changes
  useEffect(() => {
    if (!projectId) return;

    // Don't interfere during tab removal operations
    if (tabsSnapshot.isRemovingTab) {
      return;
    }

    // Filter persisted tabs for current project
    const projectTabs = tabsSnapshot.tabs.filter(
      (t) => t.metadata.projectId === projectId
    );

    // Check if current active tab already matches the route - if so, don't interfere
    const currentActiveTab = projectTabs.find(t => t.id === tabsSnapshot.activeTabId);
    
    
    // For edit routes, check if current tab is already correct
    if (isDatasourceEditRoute && datasourceId && currentActiveTab) {
      if (currentActiveTab.type === 'datasource_edit' && 
          currentActiveTab.metadata.datasourceId === datasourceId) {
        return;
      }
    }
    
    // For query routes, check if current tab is already correct  
    if (isQueryEditorRoute && datasourceId && currentActiveTab) {
      if (currentActiveTab.type === 'datasource_query' && 
          currentActiveTab.metadata.datasourceId === datasourceId) {
        return;
      }
    }

    // Determine the appropriate tab for the current route
    let targetTabId: string | null = null;
    let shouldCreateTab = true;

    if (conversationId && conversationId !== "new") {
      // Look for existing chat tab with this conversation
      const existingTab = projectTabs.find(
        (t) => t.type === "chat" && t.metadata.conversationId === conversationId
      );

      if (existingTab) {
        // Update existing tab with current conversation title if available
        const conversation = chatSnapshot.map[conversationId];
        if (
          conversation &&
          (!existingTab.metadata.conversationTitle ||
            existingTab.metadata.conversationTitle !== conversation.title)
        ) {
          tabsActions.updateTab(existingTab.id, {
            title: conversation.title,
            metadata: {
              ...existingTab.metadata,
              conversationTitle: conversation.title,
            },
          });
        }
        targetTabId = existingTab.id;
        shouldCreateTab = false;
      } else {
        // Create new chat tab
        const conversation = chatSnapshot.map[conversationId];
        const title = conversation?.title || "Chat";
        targetTabId = tabsActions.getOrCreateActiveTab(
          "chat",
          {
            conversationId,
            projectId,
            conversationTitle: conversation?.title,
          },
          title
        );
        shouldCreateTab = false;
      }
    } else if (isDataBrowserRoute && datasourceId) {
      // Look for existing data browser tab
      const existingTab = projectTabs.find(
        (t) =>
          (t.type === "datasource_table_data" ||
            t.type === "datasource_table_structure") &&
          t.metadata.datasourceId === datasourceId
      );

      if (existingTab) {
        // Update existing tab with current table name if available
        const tableName = dataBrowserSnapshot.selectedTable;
        if (
          tableName &&
          (!existingTab.metadata.tableName ||
            existingTab.metadata.tableName !== tableName)
        ) {
          tabsActions.updateTab(existingTab.id, {
            title: tableName,
            metadata: { ...existingTab.metadata, tableName },
          });
        }
        targetTabId = existingTab.id;
        shouldCreateTab = false;
      } else {
        // Create new data browser tab
        const tableName = dataBrowserSnapshot.selectedTable;
        const title = tableName ? tableName : "Table Data";
        targetTabId = tabsActions.getOrCreateActiveTab(
          "datasource_table_data",
          {
            datasourceId,
            projectId,
            tableName: tableName || undefined,
          },
          title
        );
        shouldCreateTab = false;
      }
    } else if (isQueryEditorRoute && datasourceId) {
      // Look for existing query editor tab
      const existingTab = projectTabs.find(
        (t) =>
          t.type === "datasource_query" &&
          t.metadata.datasourceId === datasourceId
      );

      if (existingTab) {
        targetTabId = existingTab.id;
        shouldCreateTab = false;
      } else {
        // Create new query editor tab
        targetTabId = tabsActions.getOrCreateActiveTab(
          "datasource_query",
          {
            datasourceId,
            projectId,
          },
          "Query Editor"
        );
        shouldCreateTab = false;
      }
    } else if (isDatasourceEditRoute && datasourceId) {
      // Look for existing edit datasource tab
      const existingTab = projectTabs.find(
        (t) =>
          t.type === "datasource_edit" &&
          t.metadata.datasourceId === datasourceId
      );

      if (existingTab) {
        targetTabId = existingTab.id;
        shouldCreateTab = false;
      } else {
        // Create new edit datasource tab
        targetTabId = tabsActions.getOrCreateActiveTab(
          "datasource_edit",
          {
            projectId,
            datasourceId,
          },
          "Edit Datasource"
        );
        shouldCreateTab = false;
      }
    } else if (isDatasourceNewRoute) {
      // Look for existing new datasource tab
      const existingTab = projectTabs.find((t) => t.type === "datasource_new");

      if (existingTab) {
        targetTabId = existingTab.id;
        shouldCreateTab = false;
      } else {
        // Create new datasource tab
        targetTabId = tabsActions.getOrCreateActiveTab(
          "datasource_new",
          {
            projectId,
          },
          "New Datasource"
        );
        shouldCreateTab = false;
      }
    } else if (isAnalysisViewRoute && analysisId) {
      // Look for existing analysis tab
      const existingTab = projectTabs.find(
        (t) => t.type === "analysis" && t.metadata.analysisId === analysisId
      );

      if (existingTab) {
        targetTabId = existingTab.id;
        shouldCreateTab = false;
      } else {
        // Create new analysis tab
        targetTabId = tabsActions.getOrCreateActiveTab(
          "analysis",
          {
            projectId,
            analysisId,
          },
          "Analysis"
        );
        shouldCreateTab = false;
      }
    } else if (isAnalysisNewRoute) {
      // Look for existing new analysis tab
      const existingTab = projectTabs.find((t) => t.type === "analysis");

      if (existingTab) {
        targetTabId = existingTab.id;
        shouldCreateTab = false;
      } else {
        // Create new analysis tab
        targetTabId = tabsActions.getOrCreateActiveTab(
          "analysis",
          {
            projectId,
          },
          "New Analysis"
        );
        shouldCreateTab = false;
      }
    } else if (isContextRoute) {
      // Look for existing context tab
      const existingTab = projectTabs.find(
        (t) => t.type === "context" && t.metadata.projectId === projectId
      );

      if (existingTab) {
        targetTabId = existingTab.id;
        shouldCreateTab = false;
      } else {
        // Create new context tab
        targetTabId = tabsActions.getOrCreateActiveTab(
          "context",
          {
            projectId,
          },
          "Context"
        );
        shouldCreateTab = false;
      }
    } else if (isNewRoute) {
      // Always create/reuse chat tab for new conversations
      targetTabId = tabsActions.getOrCreateActiveTab(
        "chat",
        {
          projectId,
          conversationId: "new",
        },
        "New Chat"
      );
      shouldCreateTab = false;
    }

    // If we have persisted tabs but need to create one for current route
    if (shouldCreateTab && projectTabs.length === 0) {
      // No tabs exist for this project, create default based on route
      targetTabId = tabsActions.getOrCreateActiveTab(
        "chat",
        {
          projectId,
        },
        "Chat"
      );
    }

    // Set the target tab as active if we determined one
    if (targetTabId && targetTabId !== tabsSnapshot.activeTabId) {
      tabsActions.setActiveTab(targetTabId);
    }
  }, [
    projectId,
    conversationId,
    datasourceId,
    analysisId,
    isDataBrowserRoute,
    isQueryEditorRoute,
    isDatasourceEditRoute,
    isDatasourceNewRoute,
    isAnalysisViewRoute,
    isAnalysisNewRoute,
    isNewRoute,
    isContextRoute,
    location.pathname,
    tabsSnapshot.tabs,
    tabsSnapshot.activeTabId,
    tabsSnapshot.isRemovingTab,
  ]);

  // Render content based on active tab
  const renderTabContent = () => {
    const activeTab = tabsSnapshot.tabs.find(
      (t) => t.id === tabsSnapshot.activeTabId
    );
    if (!activeTab) {
      // Fallback to original routing logic when no tabs
      if (isDataBrowserRoute && datasourceId) {
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <DataBrowserInlineEditing
              datasourceId={datasourceId}
              onClose={() => navigate(`/p/${projectId}`)}
            />
          </Suspense>
        );
      } else if (isQueryEditorRoute && datasourceId) {
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <QueryEditor datasourceId={datasourceId} />
          </Suspense>
        );
      } else if (isDatasourceEditRoute && datasourceId) {
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <DatasourcesMain
              projectId={projectId!}
              mode="edit"
              datasourceId={datasourceId}
            />
          </Suspense>
        );
      } else if (isDatasourceNewRoute) {
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <DatasourcesMain projectId={projectId!} mode="new" />
          </Suspense>
        );
      } else if (isAnalysisNewRoute) {
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <AnalysisEditor 
              projectId={projectId!}
              mode="create"
            />
          </Suspense>
        );
      } else if (isAnalysisViewRoute && analysisId) {
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <AnalysisEditor 
              analysisId={analysisId}
              projectId={projectId!}
            />
          </Suspense>
        );
      } else if (isAnalysisListRoute) {
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <AnalysisList projectId={projectId!} />
          </Suspense>
        );
      } else if (isContextRoute) {
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <ContextEditor projectId={projectId!} />
          </Suspense>
        );
      } else if (isNewRoute) {
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <NewChat />
          </Suspense>
        );
      } else {
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <Chat />
          </Suspense>
        );
      }
    }

    // Render based on active tab
    switch (activeTab.type) {
      case "chat":
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            {activeTab.metadata.conversationId === "new" ? <NewChat /> : <Chat />}
          </Suspense>
        );
      case "datasource_table_data":
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <DataBrowserInlineEditing
              datasourceId={activeTab.metadata.datasourceId!}
              onClose={() => navigate(`/p/${projectId}`)}
            />
          </Suspense>
        );
      case "datasource_table_structure":
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <DataBrowserInlineEditing
              datasourceId={activeTab.metadata.datasourceId!}
              mode="structure"
              onClose={() => navigate(`/p/${projectId}`)}
            />
          </Suspense>
        );
      case "datasource_query":
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <QueryEditor datasourceId={activeTab.metadata.datasourceId!} />
          </Suspense>
        );
      case "datasource_edit":
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <DatasourcesMain
              projectId={projectId!}
              mode="edit"
              datasourceId={activeTab.metadata.datasourceId}
            />
          </Suspense>
        );
      case "datasource_new":
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <DatasourcesMain projectId={projectId!} mode="new" />
          </Suspense>
        );
      case "analysis":
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <AnalysisEditor 
              analysisId={activeTab.metadata.analysisId}
              projectId={projectId!}
            />
          </Suspense>
        );
      case "context":
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <ContextEditor projectId={projectId!} />
          </Suspense>
        );
      default:
        return (
          <Suspense
            fallback={<div className="flex-1 animate-pulse bg-gray-50" />}
          >
            <Chat />
          </Suspense>
        );
    }
  };

  // Create new conversation immediately instead of using 'new' pseudo-state
  const createNewConversationAndRedirect = async () => {
    if (!projectId) return;

    try {
      const response = await api.fetchStream("/conversations", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          project_id: projectId,
        }),
      });

      if (!response.ok) {
        throw new Error("Failed to create new conversation");
      }

      const newConversation = await response.json();
      navigate(`/p/${projectId}/c/${newConversation.id}`, { replace: true });
    } catch (error) {
      console.error("Failed to create new conversation:", error);
      // Fallback to 'new' pseudo-conversation
      navigate(`/p/${projectId}/new`, { replace: true });
    }
  };

  // Handle 'new' conversation ID - don't save it to localStorage
  const effectiveConversationId = isNewRoute ? undefined : conversationId;

  // Handle responsive behavior
  useEffect(() => {
    const handleResize = () => {
      const mobile = window.innerWidth < 768;
      uiActions.setMobile(mobile);
      // Auto-collapse sidebar on mobile
      if (mobile && !uiSnapshot.isSidebarCollapsed) {
        uiActions.setSidebarCollapsed(true);
      }
    };

    handleResize(); // Check initial size
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [uiSnapshot.isSidebarCollapsed]);

  const toggleSidebar = () => {
    uiActions.toggleSidebar();
  };

  const handleConversationSelect = (newConversationId: string) => {
    if (projectId) {
      navigate(`/p/${projectId}/c/${newConversationId}`);
    }
  };

  // Don't render until we have a projectId
  if (!projectId) {
    return null;
  }

  return (
    <div className="flex-1 flex relative h-full w-full">
      <Suspense fallback={<div className="w-64 bg-gray-50 animate-pulse" />}>
        <ProjectSidebar
          isCollapsed={
            uiSnapshot.isMobile ? true : uiSnapshot.isSidebarCollapsed
          }
          onToggle={toggleSidebar}
          projectId={projectId}
          currentConversationId={effectiveConversationId}
          onConversationSelect={handleConversationSelect}
        />
      </Suspense>
      <div className="flex flex-1 flex-col min-w-0">
        <TabBar />
        <div className="flex-1 overflow-hidden flex flex-col">{renderTabContent()}</div>
      </div>
    </div>
  );
}
