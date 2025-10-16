import { proxy } from "valtio";
import { nanoid } from "nanoid";
import { subscribeKey } from "valtio/utils";

export type TabType =
  | 'chat'
  | 'datasource_table_data'
  | 'datasource_table_structure'
  | 'datasource_query'
  | 'datasource_edit'
  | 'datasource_new'
  | 'datasource_list'
  | 'analysis'
  | 'analysis_edit'
  | 'context'
  | 'members';

export interface Tab {
  id: string;
  type: TabType;
  title: string;
  isActiveForType: boolean;
  metadata: {
    conversationId?: string;
    conversationTitle?: string;
    datasourceId?: string;
    tableName?: string;
    projectId: string;
    query?: string;
    analysisId?: string;
    analysisTitle?: string;
  };
}

interface TabsState {
  tabs: Tab[];
  activeTabId: string | null;
  activeTabByType: Record<TabType, string>;
  tabHistory: string[];
  isRemovingTab: boolean; // Flag to prevent interference during removal
  currentProjectId: string | null;
  projectTabs: Record<string, {
    tabs: Tab[];
    activeTabId: string | null;
    activeTabByType: Record<TabType, string>;
    tabHistory: string[];
  }>;
}

const initialTabsState: TabsState = {
  tabs: [],
  activeTabId: null,
  activeTabByType: {} as Record<TabType, string>,
  tabHistory: [],
  isRemovingTab: false,
  currentProjectId: null,
  projectTabs: {},
};

// Load persisted tabs from localStorage
const loadPersistedTabs = (): TabsState => {
  try {
    const saved = localStorage.getItem('clay-studio-tabs');
    if (saved) {
      const parsed = JSON.parse(saved);
      // Validate the structure
      if (parsed && typeof parsed === 'object') {
        // Handle migration from old format to new project-based format
        if (Array.isArray(parsed.tabs) && !parsed.projectTabs) {
          // Old format - migrate to new format
          const migratedState = { ...initialTabsState };
          // Group tabs by projectId
          const tabsByProject: Record<string, Tab[]> = {};
          parsed.tabs.forEach((tab: Tab) => {
            const projectId = tab.metadata?.projectId;
            if (projectId) {
              if (!tabsByProject[projectId]) {
                tabsByProject[projectId] = [];
              }
              tabsByProject[projectId].push(tab);
            }
          });
          // Create project tabs structure
          Object.entries(tabsByProject).forEach(([projectId, tabs]) => {
            migratedState.projectTabs[projectId] = {
              tabs,
              activeTabId: parsed.activeTabId || null,
              activeTabByType: parsed.activeTabByType || {},
              tabHistory: parsed.tabHistory || [],
            };
          });
          return migratedState;
        }
        // New format
        return { ...initialTabsState, ...parsed };
      }
    }
  } catch (error) {
    console.warn('Failed to load persisted tabs:', error);
  }
  return initialTabsState;
};

export const tabsStore = proxy(loadPersistedTabs());

// Validate persisted tabs after loading
if (typeof window !== 'undefined') {
  // Use setTimeout to ensure this runs after the store is fully initialized
  setTimeout(() => {
    tabsActions.validatePersistedTabs();
  }, 0);
}

// Helper function to persist state
const persistState = () => {
  try {
    localStorage.setItem('clay-studio-tabs', JSON.stringify({
      tabs: tabsStore.tabs,
      activeTabId: tabsStore.activeTabId,
      activeTabByType: tabsStore.activeTabByType,
      tabHistory: tabsStore.tabHistory,
      currentProjectId: tabsStore.currentProjectId,
      projectTabs: tabsStore.projectTabs,
    }));
  } catch (error) {
    console.warn('Failed to persist tabs:', error);
  }
};

// Subscribe to changes and persist to localStorage
subscribeKey(tabsStore, 'tabs', persistState);
subscribeKey(tabsStore, 'activeTabId', persistState);
subscribeKey(tabsStore, 'projectTabs', persistState);
subscribeKey(tabsStore, 'currentProjectId', persistState);

export const tabsActions = {
  switchToProject: (projectId: string) => {
    // Save current project's tabs if there's a current project
    if (tabsStore.currentProjectId && tabsStore.currentProjectId !== projectId) {
      tabsStore.projectTabs[tabsStore.currentProjectId] = {
        tabs: [...tabsStore.tabs],
        activeTabId: tabsStore.activeTabId,
        activeTabByType: { ...tabsStore.activeTabByType },
        tabHistory: [...tabsStore.tabHistory],
      };
    }

    // Switch to new project
    tabsStore.currentProjectId = projectId;

    // Load the new project's tabs
    if (tabsStore.projectTabs[projectId]) {
      const projectData = tabsStore.projectTabs[projectId];
      tabsStore.tabs = [...projectData.tabs];
      tabsStore.activeTabId = projectData.activeTabId;
      tabsStore.activeTabByType = { ...projectData.activeTabByType };
      tabsStore.tabHistory = [...projectData.tabHistory];
    } else {
      // Initialize empty tabs for new project
      tabsStore.tabs = [];
      tabsStore.activeTabId = null;
      tabsStore.activeTabByType = {} as Record<TabType, string>;
      tabsStore.tabHistory = [];
    }
  },

  addTab: (tabData: Omit<Tab, 'id' | 'isActiveForType'>) => {
    const id = nanoid();
    const tab: Tab = {
      ...tabData,
      id,
      isActiveForType: true,
    };

    // Add the new tab
    tabsStore.tabs.push(tab);
    
    // Set as active tab overall
    tabsStore.activeTabId = id;
    
    // Add to tab history (remove if already exists, then add to end)
    const historyIndex = tabsStore.tabHistory.indexOf(id);
    if (historyIndex > -1) {
      tabsStore.tabHistory.splice(historyIndex, 1);
    }
    tabsStore.tabHistory.push(id);
    
    // Update active tab for this type
    const previousActiveTabId = tabsStore.activeTabByType[tab.type];
    if (previousActiveTabId) {
      // Mark previous active tab as not active for type
      const previousTab = tabsStore.tabs.find(t => t.id === previousActiveTabId);
      if (previousTab) {
        previousTab.isActiveForType = false;
      }
    }
    
    // Set this as the active tab for its type
    tabsStore.activeTabByType[tab.type] = id;
    
    return id;
  },

  removeTab: (tabId: string) => {
    // Set flag to prevent MainApp interference
    tabsStore.isRemovingTab = true;

    const tabIndex = tabsStore.tabs.findIndex(t => t.id === tabId);
    if (tabIndex === -1) {
      tabsStore.isRemovingTab = false;
      return;
    }

    const tab = tabsStore.tabs[tabIndex];
    const { type } = tab;

    // Remove the tab
    tabsStore.tabs.splice(tabIndex, 1);

    // If this was the currently active tab, find the next tab BEFORE removing from history
    let nextTabId: string | null = null;
    if (tabsStore.activeTabId === tabId && tabsStore.tabs.length > 0) {
      // Find the most recent tab in history that still exists and is not the tab being removed
      for (let i = tabsStore.tabHistory.length - 1; i >= 0; i--) {
        const historyTabId = tabsStore.tabHistory[i];
        if (historyTabId !== tabId && tabsStore.tabs.find(t => t.id === historyTabId)) {
          nextTabId = historyTabId;
          break;
        }
      }

      // If no tab found in history, fall back to the last tab in the array
      if (!nextTabId && tabsStore.tabs.length > 0) {
        nextTabId = tabsStore.tabs[tabsStore.tabs.length - 1].id;
      }
    }

    // Remove from tab history
    const historyIndex = tabsStore.tabHistory.indexOf(tabId);
    if (historyIndex > -1) {
      tabsStore.tabHistory.splice(historyIndex, 1);
    }

    // If this was the active tab for its type, find another tab of the same type to make active
    if (tabsStore.activeTabByType[type] === tabId) {
      const otherTabOfSameType = tabsStore.tabs.find(t => t.type === type);
      if (otherTabOfSameType) {
        tabsStore.activeTabByType[type] = otherTabOfSameType.id;
        otherTabOfSameType.isActiveForType = true;
      } else {
        delete tabsStore.activeTabByType[type];
      }
    }

    // Set the new active tab
    if (tabsStore.activeTabId === tabId) {
      if (nextTabId) {
        tabsStore.activeTabId = nextTabId;
        // Add the newly active tab to history if not already there
        if (!tabsStore.tabHistory.includes(nextTabId)) {
          tabsStore.tabHistory.push(nextTabId);
        }
      } else {
        tabsStore.activeTabId = null;
        tabsStore.tabHistory = [];
      }
    }

    // Update the projectTabs entry for the current project to reflect the removed tab
    if (tabsStore.currentProjectId) {
      // Create a new object reference to ensure reactivity
      tabsStore.projectTabs = {
        ...tabsStore.projectTabs,
        [tabsStore.currentProjectId]: {
          tabs: [...tabsStore.tabs],
          activeTabId: tabsStore.activeTabId,
          activeTabByType: { ...tabsStore.activeTabByType },
          tabHistory: [...tabsStore.tabHistory],
        },
      };
    }

    // Clear the removal flag after a brief delay to allow navigation to complete
    setTimeout(() => {
      tabsStore.isRemovingTab = false;
    }, 100);
  },

  setActiveTab: (tabId: string) => {
    const tab = tabsStore.tabs.find(t => t.id === tabId);
    if (!tab) return;
    
    // Set as overall active tab
    tabsStore.activeTabId = tabId;
    
    // Add to tab history (remove if already exists, then add to end)
    const historyIndex = tabsStore.tabHistory.indexOf(tabId);
    if (historyIndex > -1) {
      tabsStore.tabHistory.splice(historyIndex, 1);
    }
    tabsStore.tabHistory.push(tabId);
    
    // Update active tab for this type
    const previousActiveTabId = tabsStore.activeTabByType[tab.type];
    if (previousActiveTabId && previousActiveTabId !== tabId) {
      const previousTab = tabsStore.tabs.find(t => t.id === previousActiveTabId);
      if (previousTab) {
        previousTab.isActiveForType = false;
      }
    }
    
    // Set this tab as active for its type
    tabsStore.activeTabByType[tab.type] = tabId;
    tab.isActiveForType = true;
  },

  updateTab: (tabId: string, updates: Partial<Omit<Tab, 'id'>>) => {
    const tab = tabsStore.tabs.find(t => t.id === tabId);
    if (!tab) return;
    
    Object.assign(tab, updates);
  },

  getOrCreateActiveTab: (type: TabType, metadata: Tab['metadata'], title?: string) => {
    // For chat tabs, look for specific conversation match first (including 'new')
    if (type === 'chat' && metadata.conversationId) {
      const existingChatTab = tabsStore.tabs.find(t => 
        t.type === 'chat' && t.metadata.conversationId === metadata.conversationId
      );
      if (existingChatTab) {
        tabsActions.setActiveTab(existingChatTab.id);
        return existingChatTab.id;
      }
    }
    
    // For datasource tabs, look for specific datasource match first
    if (type.startsWith('datasource_') && metadata.datasourceId) {
      const existingDatasourceTab = tabsStore.tabs.find(t => 
        t.type === type && t.metadata.datasourceId === metadata.datasourceId
      );
      if (existingDatasourceTab) {
        // Update metadata and title if needed
        existingDatasourceTab.metadata = { ...existingDatasourceTab.metadata, ...metadata };
        if (title) {
          existingDatasourceTab.title = title;
        }
        tabsActions.setActiveTab(existingDatasourceTab.id);
        return existingDatasourceTab.id;
      }
    }
    
    // For analysis tabs, look for specific analysis match first
    if (type === 'analysis' && metadata.analysisId) {
      const existingAnalysisTab = tabsStore.tabs.find(t => 
        t.type === 'analysis' && t.metadata.analysisId === metadata.analysisId
      );
      if (existingAnalysisTab) {
        // Update metadata and title if needed
        existingAnalysisTab.metadata = { ...existingAnalysisTab.metadata, ...metadata };
        if (title) {
          existingAnalysisTab.title = title;
        }
        tabsActions.setActiveTab(existingAnalysisTab.id);
        return existingAnalysisTab.id;
      }
    }

    // For context tabs, only allow one per project
    if (type === 'context') {
      const existingContextTab = tabsStore.tabs.find(t =>
        t.type === 'context' && t.metadata.projectId === metadata.projectId
      );
      if (existingContextTab) {
        tabsActions.setActiveTab(existingContextTab.id);
        return existingContextTab.id;
      }
    }

    // For members tabs, only allow one per project
    if (type === 'members') {
      const existingMembersTab = tabsStore.tabs.find(t =>
        t.type === 'members' && t.metadata.projectId === metadata.projectId
      );
      if (existingMembersTab) {
        tabsActions.setActiveTab(existingMembersTab.id);
        return existingMembersTab.id;
      }
    }

    // Check if there's already an active tab of this type (fallback for general types)
    const activeTabId = tabsStore.activeTabByType[type];
    if (activeTabId) {
      const activeTab = tabsStore.tabs.find(t => t.id === activeTabId);
      if (activeTab) {
        // Update the existing active tab's metadata and title
        activeTab.metadata = { ...activeTab.metadata, ...metadata };
        if (title) {
          activeTab.title = title;
        }
        
        // Make sure it's the currently active tab
        tabsStore.activeTabId = activeTabId;
        return activeTabId;
      }
    }
    
    // No matching tab exists, create a new one
    return tabsActions.addTab({
      type,
      title: title || tabsActions.getDefaultTitle(type, metadata),
      metadata,
    });
  },

  openInNewTab: (type: TabType, metadata: Tab['metadata'], title?: string) => {
    // For context tabs, only allow one per project
    if (type === 'context') {
      const existingContextTab = tabsStore.tabs.find(t =>
        t.type === 'context' && t.metadata.projectId === metadata.projectId
      );
      if (existingContextTab) {
        tabsActions.setActiveTab(existingContextTab.id);
        return existingContextTab.id;
      }
    }

    // For members tabs, only allow one per project
    if (type === 'members') {
      const existingMembersTab = tabsStore.tabs.find(t =>
        t.type === 'members' && t.metadata.projectId === metadata.projectId
      );
      if (existingMembersTab) {
        tabsActions.setActiveTab(existingMembersTab.id);
        return existingMembersTab.id;
      }
    }

    // Always create a new tab for other types
    return tabsActions.addTab({
      type,
      title: title || tabsActions.getDefaultTitle(type, metadata),
      metadata,
    });
  },

  getDefaultTitle: (type: TabType, metadata: Tab['metadata']): string => {
    switch (type) {
      case 'chat':
        return metadata.conversationTitle || metadata.conversationId === 'new' ? 'New Chat' : 'Chat...';
      case 'datasource_table_data':
        return metadata.tableName || 'Table Data';
      case 'datasource_table_structure':
        return `${metadata.tableName || 'Table'} Structure`;
      case 'datasource_query':
        return 'Query Editor';
      case 'datasource_edit':
        return 'Edit Datasource';
      case 'datasource_new':
        return 'New Datasource';
      case 'datasource_list':
        return 'Datasources';
      case 'analysis':
        return metadata.analysisTitle || 'Analysis';
      case 'context':
        return 'Context';
      case 'members':
        return 'Members';
      default:
        return 'Tab';
    }
  },

  getTabById: (tabId: string) => {
    return tabsStore.tabs.find(t => t.id === tabId);
  },

  closeAllTabsOfType: (type: TabType) => {
    const tabsToRemove = tabsStore.tabs.filter(t => t.type === type);
    tabsToRemove.forEach(tab => tabsActions.removeTab(tab.id));
  },

  clearAll: () => {
    tabsStore.tabs = [];
    tabsStore.activeTabId = null;
    tabsStore.activeTabByType = {} as Record<TabType, string>;
    tabsStore.tabHistory = [];
  },

  // Clean up stale tabs that reference non-existent resources
  cleanupStaleTabs: (validConversationIds: string[] = [], validDatasourceIds: string[] = []) => {
    const staleTabs: string[] = [];
    
    tabsStore.tabs.forEach(tab => {
      let isStale = false;
      
      if (tab.type === 'chat' && tab.metadata.conversationId) {
        isStale = !validConversationIds.includes(tab.metadata.conversationId);
      } else if (tab.type.startsWith('datasource_') && tab.metadata.datasourceId) {
        isStale = !validDatasourceIds.includes(tab.metadata.datasourceId);
      }
      
      if (isStale) {
        staleTabs.push(tab.id);
      }
    });
    
    // Remove stale tabs
    staleTabs.forEach(tabId => tabsActions.removeTab(tabId));
  },

  // Validate and fix persisted tabs after loading
  validatePersistedTabs: () => {
    // Remove tabs with invalid structure or missing required data
    const validTabs = tabsStore.tabs.filter(tab => {
      return tab.id && tab.type && tab.metadata && tab.metadata.projectId;
    });
    
    if (validTabs.length !== tabsStore.tabs.length) {
      tabsStore.tabs = validTabs;
      
      // Fix activeTabId if it references a removed tab
      if (tabsStore.activeTabId && !validTabs.find(t => t.id === tabsStore.activeTabId)) {
        tabsStore.activeTabId = validTabs.length > 0 ? validTabs[0].id : null;
      }
      
      // Fix activeTabByType references
      Object.keys(tabsStore.activeTabByType).forEach(type => {
        const tabId = tabsStore.activeTabByType[type as TabType];
        if (!validTabs.find(t => t.id === tabId)) {
          delete tabsStore.activeTabByType[type as TabType];
        }
      });
      
      // Clean up tab history to only include valid tabs
      const validTabIds = validTabs.map(t => t.id);
      tabsStore.tabHistory = tabsStore.tabHistory.filter(tabId => validTabIds.includes(tabId));
    }
  },
};