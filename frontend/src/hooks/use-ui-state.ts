import { useCallback } from 'react';
import { useSnapshot } from 'valtio';
import { uiStore, uiActions } from '../store/ui-store';

/**
 * Hook for managing global UI state
 * This preserves sidebar toggles, active tabs, and other UI preferences
 */
export function useUIState() {
  const snapshot = useSnapshot(uiStore, { sync: true });

  const toggleSidebar = useCallback(() => {
    uiActions.toggleMainSidebar();
  }, []);

  const toggleFileSidebar = useCallback(() => {
    uiActions.toggleFileSidebar();
  }, []);

  const setTab = useCallback((tab: string) => {
    uiActions.setActiveTab(tab);
  }, []);

  return {
    sidebarOpen: snapshot.sidebarOpen,
    fileSidebarOpen: snapshot.fileSidebarOpen,
    activeTab: snapshot.activeTab,
    isSidebarCollapsed: snapshot.isSidebarCollapsed,
    isMobile: snapshot.isMobile,
    isDraggingOver: snapshot.isDraggingOver,
    shouldFocusInput: snapshot.shouldFocusInput,
    isWsSubscribed: snapshot.isWsSubscribed,
    viewportHeight: snapshot.viewportHeight,
    keyboardHeight: snapshot.keyboardHeight,
    setSidebarOpen: uiActions.setSidebarOpen,
    setFileSidebarOpen: uiActions.setFileSidebarOpen,
    setActiveTab: setTab,
    toggleSidebar,
    toggleFileSidebar,
    toggleSidebarCollapse: uiActions.toggleSidebar,
    setMobile: uiActions.setMobile,
    setDragging: uiActions.setDragging,
    setFocusInput: uiActions.setFocusInput,
    setWsSubscribed: uiActions.setWsSubscribed,
    setViewportHeight: uiActions.setViewportHeight,
    setKeyboardHeight: uiActions.setKeyboardHeight,
  };
}