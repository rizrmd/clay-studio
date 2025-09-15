import { useSnapshot } from "valtio";
import { useNavigate } from "react-router-dom";
import { 
  MessageSquare, 
  Database, 
  TableProperties, 
  Terminal,
  Edit, 
  Plus,
  X 
} from "lucide-react";
import { cn } from "@/lib/utils";
import { tabsStore, tabsActions, Tab, TabType } from "@/lib/store/tabs-store";

const TAB_ICONS: Record<TabType, typeof MessageSquare> = {
  'chat': MessageSquare,
  'datasource_table_data': Database,
  'datasource_table_structure': TableProperties,
  'datasource_query': Terminal,
  'datasource_edit': Edit,
  'datasource_new': Plus,
  'datasource_list': Database,
};

interface TabItemProps {
  tab: Tab;
}

export function TabItem({ tab }: TabItemProps) {
  const snapshot = useSnapshot(tabsStore);
  const navigate = useNavigate();
  const isActive = snapshot.activeTabId === tab.id;
  const Icon = TAB_ICONS[tab.type];

  const navigateToTab = (tab: Tab) => {
    const { projectId, conversationId, datasourceId, tableName } = tab.metadata;
    
    switch (tab.type) {
      case 'chat':
        if (conversationId && conversationId !== 'new') {
          navigate(`/p/${projectId}/c/${conversationId}`);
        } else {
          navigate(`/p/${projectId}/new`);
        }
        break;
        
      case 'datasource_table_data':
      case 'datasource_table_structure':
        if (datasourceId) {
          const tableParam = tableName ? `?table=${encodeURIComponent(tableName)}` : '';
          navigate(`/p/${projectId}/datasources/${datasourceId}/browse${tableParam}`);
        }
        break;
        
      case 'datasource_query':
        if (datasourceId) {
          // Check if we're already on the correct query route
          const currentPath = window.location.pathname;
          const targetPath = `/p/${projectId}/datasources/${datasourceId}/query`;
          if (currentPath !== targetPath) {
            navigate(targetPath);
          }
        }
        break;
        
      case 'datasource_edit':
        if (datasourceId) {
          // Check if we're already on the correct edit route
          const currentPath = window.location.pathname;
          const targetPath = `/p/${projectId}/datasources/${datasourceId}/edit`;
          if (currentPath !== targetPath) {
            navigate(targetPath);
          }
        } else {
          navigate(`/p/${projectId}`);
        }
        break;
        
      case 'datasource_new':
        navigate(`/p/${projectId}/datasources/new`);
        break;
        
      case 'datasource_list':
        navigate(`/p/${projectId}`);
        break;
        
      default:
        // Fallback to chat if unknown type
        navigate(`/p/${projectId}`);
        break;
    }
  };

  const handleTabClick = () => {
    // Set the tab as active
    tabsActions.setActiveTab(tab.id);
    
    // Navigate to the appropriate URL
    navigateToTab(tab);
  };

  const handleCloseClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    
    
    const wasActiveTab = isActive;
    
    // Remove the tab - this will handle active tab switching internally
    tabsActions.removeTab(tab.id);
    
    // If this was the active tab, handle navigation after closure
    if (wasActiveTab) {
      // Use setTimeout to ensure the store update has been processed and removal flag is set
      setTimeout(() => {
        // The removeTab action should have already set the correct activeTabId based on history
        const newActiveTabId = tabsStore.activeTabId;
        const remainingTabs = tabsStore.tabs;
        
        if (newActiveTabId) {
          // There's a new active tab, navigate to it
          const newActiveTab = remainingTabs.find(t => t.id === newActiveTabId);
          if (newActiveTab) {
            navigateToTab(newActiveTab);
          }
        } else {
          // No tabs left, navigate to project root
          navigate(`/p/${tab.metadata.projectId}`);
        }
      }, 50); // Increased delay to ensure removal flag is processed
    }
  };

  return (
    <div 
      className={cn(
        "flex items-center gap-2 px-3 py-1.5 rounded-t cursor-pointer min-w-0 group border-b-2 transition-all duration-200",
        isActive 
          ? "bg-background border-b-primary text-primary shadow-sm" 
          : "bg-muted hover:bg-muted/80 border-b-transparent text-muted-foreground hover:text-foreground"
      )}
      onClick={(e) => {
        // Don't trigger tab click if clicking on the close button
        if ((e.target as HTMLElement).closest('button')) {
          return;
        }
        handleTabClick();
      }}
    >
      <Icon className="h-4 w-4 flex-shrink-0" />
      <span className="text-sm truncate max-w-[120px]" title={tab.title}>
        {tab.title}
      </span>
      <button 
        onClick={handleCloseClick}
        className={cn(
          "opacity-70 group-hover:opacity-100 transition-all p-1 rounded hover:bg-muted-foreground/20 hover:bg-red-100 hover:text-red-600",
          isActive && "opacity-100"
        )}
        title="Close tab"
      >
        <X className="h-3 w-3" />
      </button>
    </div>
  );
}