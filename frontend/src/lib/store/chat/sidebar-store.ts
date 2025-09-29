import { proxy } from "valtio";

interface ProjectSidebarState {
  accordionValue: string[];
  selectedDatasourceId: string | null;
}

export const sidebarStore = proxy({
  isOpen: true,
  selectedConversations: [] as string[],
  isDeleteMode: false,
  isMobileMenuOpen: false,
  recentlyUpdatedConversations: new Set(),
  loading: false,
  error: null as null | string,
  renameDialogOpen: false,
  newTitle: "",
  // Accordion state
  accordionValue: ["conversations", "datasources"] as string[], // sections that are open
  selectedDatasourceId: null as string | null,
  // Project-specific states
  currentProjectId: null as string | null,
  projectStates: {} as Record<string, ProjectSidebarState>,
});

export const sidebarActions = {
  switchToProject: (projectId: string) => {
    // Save current project's state if there's a current project
    if (sidebarStore.currentProjectId && sidebarStore.currentProjectId !== projectId) {
      sidebarStore.projectStates[sidebarStore.currentProjectId] = {
        accordionValue: [...sidebarStore.accordionValue],
        selectedDatasourceId: sidebarStore.selectedDatasourceId,
      };
    }

    // Switch to new project
    sidebarStore.currentProjectId = projectId;

    // Load the new project's state
    if (sidebarStore.projectStates[projectId]) {
      const projectState = sidebarStore.projectStates[projectId];
      sidebarStore.accordionValue = [...projectState.accordionValue];
      sidebarStore.selectedDatasourceId = projectState.selectedDatasourceId;
    } else {
      // Initialize default state for new project
      sidebarStore.accordionValue = ["conversations"];
      sidebarStore.selectedDatasourceId = null;
    }
  },
  addRecentlyUpdated: (id: string) => {
    sidebarStore.recentlyUpdatedConversations.add(id);
  },
  setMobileMenuOpen: (open: boolean) => {
    sidebarStore.isMobileMenuOpen = open;
  },
  toggleMobileMenu: () => {
    sidebarStore.isMobileMenuOpen = !sidebarStore.isMobileMenuOpen;
  },
  toggleConversationSelection: (id: string) => {
    if (sidebarStore.isDeleteMode) {
      // Multi-select mode for delete
      if (sidebarStore.selectedConversations.includes(id)) {
        sidebarStore.selectedConversations = sidebarStore.selectedConversations.filter(convId => convId !== id);
      } else {
        sidebarStore.selectedConversations.push(id);
      }
    }
  },
  addToSelection: (id: string) => {
    if (!sidebarStore.selectedConversations.includes(id)) {
      sidebarStore.selectedConversations.push(id);
    }
  },
  removeFromSelection: (id: string) => {
    sidebarStore.selectedConversations = sidebarStore.selectedConversations.filter(convId => convId !== id);
  },
  clearSelection: () => {
    sidebarStore.selectedConversations = [];
  },
  selectAll: (conversationIds: string[]) => {
    sidebarStore.selectedConversations = [...conversationIds];
  },
  enterDeleteMode: (id?: string) => {
    sidebarStore.isDeleteMode = true;
    sidebarStore.selectedConversations = [];
    if (id) {
      sidebarStore.selectedConversations.push(id);
    }
  },
  exitDeleteMode: () => {
    sidebarStore.isDeleteMode = false;
    sidebarStore.selectedConversations = [];
  },
  openRenameDialog: (currentTitle?: string) => {
    sidebarStore.renameDialogOpen = true;
    sidebarStore.newTitle = currentTitle || "";
  },
  closeRenameDialog: () => {
    sidebarStore.renameDialogOpen = false;
    sidebarStore.newTitle = "";
  },
  setNewTitle: (title: string) => {
    sidebarStore.newTitle = title;
  },
  // Accordion actions
  setAccordionValue: (value: string[]) => {
    sidebarStore.accordionValue = value;
  },
  toggleAccordionSection: (section: string) => {
    if (sidebarStore.accordionValue.includes(section)) {
      sidebarStore.accordionValue = sidebarStore.accordionValue.filter(s => s !== section);
    } else {
      sidebarStore.accordionValue = [...sidebarStore.accordionValue, section];
    }
  },
  selectDatasource: (datasourceId: string | null) => {
    sidebarStore.selectedDatasourceId = datasourceId;
  },
};
