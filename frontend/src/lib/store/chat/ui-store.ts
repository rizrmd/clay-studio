import { proxy } from "valtio";

interface UIState {
  sidebarOpen: boolean;
  isLoading: boolean;
  error: string | null;
  isMobile: boolean;
  isSidebarCollapsed: boolean;
  currentProject: string | null;
  currentConversation: string | null;
  isDraggingOver: boolean;
  shouldFocusInput: boolean;
  isWsSubscribed: boolean;
}

const initialUIState: UIState = {
  sidebarOpen: true,
  isLoading: false,
  error: null,
  isMobile: false,
  isSidebarCollapsed: false,
  currentProject: null,
  currentConversation: null,
  isDraggingOver: false,
  shouldFocusInput: false,
  isWsSubscribed: false,
};

export const uiStore = proxy(initialUIState);

export const uiActions = {
  setSidebarOpen: (open: boolean) => {
    uiStore.sidebarOpen = open;
  },
  toggleSidebar: () => {
    uiStore.sidebarOpen = !uiStore.sidebarOpen;
  },
  setCurrentProject: (id: string) => {
    uiStore.currentProject = id;
  },
  setCurrentConversation: (id: string) => {
    uiStore.currentConversation = id;
  },
  setMobile: (mobile: boolean) => {
    uiStore.isMobile = mobile;
  },
  setSidebarCollapsed: (collapsed: boolean) => {
    uiStore.isSidebarCollapsed = collapsed;
  },
  setLoading: (loading: boolean) => {
    uiStore.isLoading = loading;
  },
  setError: (error: string | null) => {
    uiStore.error = error;
  },
  setFocusInput: (focus: boolean) => {
    uiStore.shouldFocusInput = focus;
  },
  setDragging: (dragging: boolean) => {
    uiStore.isDraggingOver = dragging;
  },
  setWsSubscribed: (subscribed: boolean) => {
    uiStore.isWsSubscribed = subscribed;
  },
};
