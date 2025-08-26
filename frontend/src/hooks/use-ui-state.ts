import { useCallback } from 'react';
import { useSnapshot } from 'valtio';
import { 
  store, 
  setSidebarOpen,
  setFileSidebarOpen,
  setActiveTab
} from '../store';

/**
 * Hook for managing global UI state
 * This preserves sidebar toggles, active tabs, and other UI preferences
 */
export function useUIState() {
  const snapshot = useSnapshot(store);

  const toggleSidebar = useCallback(() => {
    setSidebarOpen(!snapshot.ui.sidebarOpen);
  }, [snapshot.ui.sidebarOpen]);

  const toggleFileSidebar = useCallback(() => {
    setFileSidebarOpen(!snapshot.ui.fileSidebarOpen);
  }, [snapshot.ui.fileSidebarOpen]);

  const setTab = useCallback((tab: string) => {
    setActiveTab(tab);
  }, []);

  return {
    sidebarOpen: snapshot.ui.sidebarOpen,
    fileSidebarOpen: snapshot.ui.fileSidebarOpen,
    activeTab: snapshot.ui.activeTab,
    setSidebarOpen,
    setFileSidebarOpen,
    setActiveTab: setTab,
    toggleSidebar,
    toggleFileSidebar,
  };
}