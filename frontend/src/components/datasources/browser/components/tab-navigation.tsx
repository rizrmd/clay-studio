import { useState } from "react";
import { Database, Settings, ListPlus, PlusCircle, Trash2 } from "lucide-react";
import { cn } from "@/lib/utils";
import { DataBrowserLocalStore } from "../hooks/use-local-store";
import { NewRowModal } from "./new-row-modal";

const tabs = [
  {
    value: "data" as const,
    icon: Database,
    label: "Data",
  },
  {
    value: "structure" as const,
    icon: Settings,
    label: "Structure",
  },
];

interface TabNavigationProps {
  localStore: DataBrowserLocalStore;
  localSnapshot: {
    currentMode: "data" | "structure";
    selectedRows: Record<string, boolean>;
  };
  selectedTable?: string;
  onDeleteSelectedRows?: () => void;
}

export const TabNavigation = ({
  localStore,
  localSnapshot,
  selectedTable,
  onDeleteSelectedRows,
}: TabNavigationProps) => {
  const [isNewRowModalOpen, setIsNewRowModalOpen] = useState(false);

  const handleAddRowClick = () => {
    if (selectedTable) {
      setIsNewRowModalOpen(true);
    }
  };

  const selectedRowsCount = Object.keys(localSnapshot.selectedRows).length;

  return (
    <div className="border-r">
      <div className="flex flex-col w-[45px]">
        {/* Delete Button */}
        {selectedRowsCount > 0 && (
          <button
            className={cn(
              "p-3 transition-colors flex flex-col items-center gap-1 relative",
              "border-transparent text-destructive hover:text-destructive hover:bg-destructive/10"
            )}
            onClick={onDeleteSelectedRows}
            title={`Delete ${selectedRowsCount} selected row${selectedRowsCount !== 1 ? 's' : ''}`}
          >
            <div className="relative">
              <Trash2 className="h-4 w-4" />
              <div className="absolute -bottom-2 -right-2 bg-destructive text-destructive-foreground rounded-full min-w-[16px] h-4 text-xs flex items-center justify-center px-1">
                {selectedRowsCount}
              </div>
            </div>
          </button>
        )}
        {/* Add Row Button */}
        <button
          className={cn(
            "p-3 transition-colors flex flex-col items-center gap-1",
            "border-transparent text-muted-foreground hover:text-foreground hover:bg-muted/50"
          )}
          disabled={!selectedTable}
          onClick={handleAddRowClick}
          title={selectedTable ? "Add new row" : "Select table to add row"}
        >
          <PlusCircle className="h-4 w-4" />
        </button>
        {tabs.map((tab) => (
          <button
            key={tab.value}
            className={cn(
              "p-3 transition-colors flex flex-col items-center gap-1",
              localSnapshot.currentMode === tab.value
                ? "border-primary text-primary bg-primary/10"
                : "border-transparent text-muted-foreground hover:text-foreground hover:bg-muted/50"
            )}
            disabled={!selectedTable}
            onClick={() => (localStore.currentMode = tab.value)}
            title={`${tab.label}${
              selectedTable ? ` (${selectedTable})` : " (Select table)"
            }`}
          >
            <tab.icon className="h-4 w-4" />
          </button>
        ))}
      </div>
      
      <NewRowModal
        isOpen={isNewRowModalOpen}
        onClose={() => setIsNewRowModalOpen(false)}
      />
    </div>
  );
};