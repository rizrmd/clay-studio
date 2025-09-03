import { proxy } from 'valtio';

interface UIState {
  // Main app layout
  isSidebarCollapsed: boolean;
  sidebarOpen: boolean;
  fileSidebarOpen: boolean;
  isMobile: boolean;
  
  // Chat UI states
  isDraggingOver: boolean;
  shouldFocusInput: boolean;
  isWsSubscribed: boolean;
  activeTab: string;
  
  // Modal and overlay states
  activeModals: Set<string>;
  
  // Loading states
  isLoading: boolean;
  loadingMessage?: string;
  
  // Viewport management
  viewportHeight: number;
  keyboardHeight: number;
}

export const uiStore = proxy<UIState>({
  // Main app layout
  isSidebarCollapsed: false,
  sidebarOpen: true,
  fileSidebarOpen: false,
  isMobile: window.innerWidth < 768,
  
  // Chat UI states
  isDraggingOver: false,
  shouldFocusInput: false,
  isWsSubscribed: false,
  activeTab: 'chat',
  
  // Modal and overlay states
  activeModals: new Set(),
  
  // Loading states
  isLoading: false,
  loadingMessage: undefined,
  
  // Viewport management
  viewportHeight: typeof window !== 'undefined' ? window.innerHeight : 0,
  keyboardHeight: 0,
});

// Actions
export const uiActions = {
  toggleSidebar: () => {
    uiStore.isSidebarCollapsed = !uiStore.isSidebarCollapsed;
  },
  
  setSidebarCollapsed: (collapsed: boolean) => {
    uiStore.isSidebarCollapsed = collapsed;
  },
  
  setMobile: (isMobile: boolean) => {
    uiStore.isMobile = isMobile;
  },
  
  setDragging: (isDragging: boolean) => {
    uiStore.isDraggingOver = isDragging;
  },
  
  setFocusInput: (shouldFocus: boolean) => {
    uiStore.shouldFocusInput = shouldFocus;
  },
  
  setWsSubscribed: (isSubscribed: boolean) => {
    uiStore.isWsSubscribed = isSubscribed;
  },
  
  showModal: (modalId: string) => {
    uiStore.activeModals.add(modalId);
  },
  
  hideModal: (modalId: string) => {
    uiStore.activeModals.delete(modalId);
  },
  
  setLoading: (isLoading: boolean, message?: string) => {
    uiStore.isLoading = isLoading;
    uiStore.loadingMessage = message;
  },
  
  // Sidebar management
  setSidebarOpen: (open: boolean) => {
    uiStore.sidebarOpen = open;
  },
  
  setFileSidebarOpen: (open: boolean) => {
    uiStore.fileSidebarOpen = open;
  },
  
  toggleMainSidebar: () => {
    uiStore.sidebarOpen = !uiStore.sidebarOpen;
  },
  
  toggleFileSidebar: () => {
    uiStore.fileSidebarOpen = !uiStore.fileSidebarOpen;
  },
  
  // Tab management
  setActiveTab: (tab: string) => {
    uiStore.activeTab = tab;
  },
  
  // Viewport management
  setViewportHeight: (height: number) => {
    uiStore.viewportHeight = height;
  },
  
  setKeyboardHeight: (height: number) => {
    uiStore.keyboardHeight = height;
  },
};