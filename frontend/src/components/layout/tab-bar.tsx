import { useSnapshot } from "valtio";
import { tabsStore } from "@/lib/store/tabs-store";
import { TabItem } from "./tab-item";
import { cn } from "@/lib/utils";

interface TabBarProps {
  className?: string;
}

export function TabBar({ className }: TabBarProps) {
  const snapshot = useSnapshot(tabsStore);


  // Hide when no tabs
  if (snapshot.tabs.length === 0) {
    return null;
  }
  
  // Hide when only one tab, unless it's a datasource tab (which should always show tab bar for closing)
  if (snapshot.tabs.length === 1) {
    const singleTab = snapshot.tabs[0];
    const isDatasourceTab = singleTab?.type.startsWith('datasource_');
    if (!isDatasourceTab) {
      return null;
    }
  }


  return (
    <div className={cn(
      "border-b bg-background flex items-center gap-1 pt-1 overflow-x-auto",
      "scrollbar-thin scrollbar-thumb-gray-300 scrollbar-track-transparent",
      className
    )}>
      <div className="flex items-center gap-1 min-w-0">
        {snapshot.tabs.map(tab => (
          <TabItem key={tab.id} tab={tab} />
        ))}
      </div>
    </div>
  );
}