import { useState, forwardRef, useImperativeHandle } from "react";
import {
  Database,
  Settings,
  PlusCircle,
  Trash2,
  Loader2,
  CheckCheck,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { DataBrowserLocalStore } from "../hooks/use-local-store";
import { NewRowModal } from "./new-row-modal";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

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
    selectionVersion?: number;
    isSelectingAll?: boolean;
  };
  selectedTable?: string;
  totalRows?: number;
  onDeleteSelectedRows?: () => void;
  onSelectAllRows?: () => void;
  onDeselectAllRows?: () => void;
}

export interface TabNavigationRef {
  openAddRowModal: () => void;
}

export const TabNavigation = forwardRef<TabNavigationRef, TabNavigationProps>(
  (
    {
      localStore,
      localSnapshot,
      selectedTable,
      totalRows = 0,
      onDeleteSelectedRows,
      onSelectAllRows,
      onDeselectAllRows,
    },
    ref
  ) => {
    const [isNewRowModalOpen, setIsNewRowModalOpen] = useState(false);

    const handleAddRowClick = () => {
      if (selectedTable) {
        setIsNewRowModalOpen(true);
      }
    };

    useImperativeHandle(ref, () => ({
      openAddRowModal: handleAddRowClick,
    }));

    const selectedRowsCount = Object.keys(localSnapshot.selectedRows).length;
    const isAllSelected = totalRows > 0 && selectedRowsCount === totalRows;

    return (
      <TooltipProvider>
        <div className="border-r">
          <div className="flex flex-col w-[45px]">

            {/* Select/Deselect All Button */}
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  className={cn(
                    "p-3 transition-colors flex flex-col items-center gap-1 relative h-[45px]",
                    "border-transparent text-primary hover:text-primary hover:bg-primary/10",
                    localSnapshot.isSelectingAll &&
                      "opacity-50 cursor-not-allowed"
                  )}
                  onClick={isAllSelected ? onDeselectAllRows : onSelectAllRows}
                  disabled={localSnapshot.isSelectingAll}
                >
                  {localSnapshot.isSelectingAll ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <div className="border-[1.5px] border-gray-600 p-[2px] rounded-md">
                      <CheckCheck className="h-4 w-4" />
                    </div>
                  )}
                </button>
              </TooltipTrigger>
              <TooltipContent side="right" >
                <p>
                  {localSnapshot.isSelectingAll
                    ? "Loading all rows..."
                    : isAllSelected
                    ? "Deselect all rows"
                    : "Select all rows across all pages"}
                </p>
              </TooltipContent>
            </Tooltip>
            {selectedRowsCount > 0 && (
              <>
                {/* Delete Button */}
                <Tooltip>
                  <TooltipTrigger asChild>
                    <button
                      className={cn(
                        "p-3 transition-colors flex flex-col items-center gap-1 relative",
                        "border-transparent text-destructive hover:text-destructive hover:bg-destructive/10"
                      )}
                      onClick={onDeleteSelectedRows}
                    >
                      <div className="relative">
                        <Trash2 className="h-4 w-4" />
                        <div className="absolute -bottom-2 -right-2 bg-destructive text-destructive-foreground rounded-full min-w-[16px] h-4 text-xs flex items-center justify-center px-1">
                          {selectedRowsCount}
                        </div>
                      </div>
                    </button>
                  </TooltipTrigger>
                  <TooltipContent side="right">
                    <p>
                      Delete {selectedRowsCount} selected row
                      {selectedRowsCount !== 1 ? "s" : ""}
                    </p>
                  </TooltipContent>
                </Tooltip>
              </>
            )}
            {/* Add Row Button */}
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  className={cn(
                    "p-3 transition-colors flex flex-col items-center gap-1",
                    "border-transparent text-muted-foreground hover:text-foreground hover:bg-muted/50"
                  )}
                  disabled={!selectedTable}
                  onClick={handleAddRowClick}
                >
                  <PlusCircle className="h-4 w-4" />
                </button>
              </TooltipTrigger>
              <TooltipContent side="right">
                <p>
                  {selectedTable ? "Add new row" : "Select table to add row"}
                </p>
              </TooltipContent>
            </Tooltip>
            {tabs.map((tab) => (
              <Tooltip key={tab.value}>
                <TooltipTrigger asChild>
                  <button
                    className={cn(
                      "p-3 transition-colors flex flex-col items-center gap-1",
                      localSnapshot.currentMode === tab.value
                        ? "border-primary text-primary bg-primary/10"
                        : "border-transparent text-muted-foreground hover:text-foreground hover:bg-muted/50"
                    )}
                    disabled={!selectedTable}
                    onClick={() => (localStore.currentMode = tab.value)}
                  >
                    <tab.icon className="h-4 w-4" />
                  </button>
                </TooltipTrigger>
                <TooltipContent side="right">
                  <p>
                    {tab.label}
                    {selectedTable ? ` (${selectedTable})` : " (Select table)"}
                  </p>
                </TooltipContent>
              </Tooltip>
            ))}
          </div>

          <NewRowModal
            isOpen={isNewRowModalOpen}
            onClose={() => setIsNewRowModalOpen(false)}
          />
        </div>
      </TooltipProvider>
    );
  }
);
