import React from "react";
import { useSnapshot } from "valtio";
import { cn } from "@/lib/utils";
import { EditPopup } from "./edit-popup";
import { dataBrowserStore } from "@/lib/store/data-browser-store";

interface ClickableCellProps {
  value: any;
  rowId: string;
  columnKey: string;
  dataType: string;
  nullable?: boolean;
  onCellEdit: (rowId: string, columnKey: string, newValue: any) => void;
  defaultRenderer: (value: any) => React.ReactNode;
  disabled?: boolean;
}

export function ClickableCell({
  value,
  rowId,
  columnKey,
  dataType,
  nullable = false,
  onCellEdit,
  defaultRenderer,
  disabled = false,
}: ClickableCellProps) {
  const dataBrowserSnapshot = useSnapshot(dataBrowserStore);
  
  // Check if this is a new row
  const isNewRow = String(rowId).startsWith('new_');
  
  // Check if this cell has been edited
  const hasChanges = Boolean(
    dataBrowserSnapshot.editingChanges[rowId]?.[columnKey] !== undefined
  );
  
  // Get the current value (either original or edited)
  const currentValue = hasChanges 
    ? dataBrowserSnapshot.editingChanges[rowId][columnKey]
    : value;

  const handleSave = (newValue: any) => {
    onCellEdit(rowId, columnKey, newValue);
  };

  if (disabled) {
    return (
      <div className="py-1 px-2">
        {currentValue === null || currentValue === undefined ? (
          <span className="text-muted-foreground italic text-sm">NULL</span>
        ) : (
          defaultRenderer(currentValue)
        )}
      </div>
    );
  }

  return (
    <EditPopup
      value={currentValue}
      dataType={dataType}
      nullable={nullable}
      onSave={handleSave}
      multiline={dataType === "string" && String(currentValue || "").length > 50}
    >
      <div
        className={cn(
          "min-h-[32px] flex items-center transition-colors",
          hasChanges && "bg-yellow-50 border-l-2 border-yellow-400 dark:bg-yellow-900/20",
          isNewRow && !hasChanges && "bg-green-50 border-l-2 border-green-400 dark:bg-green-900/20"
        )}
      >
        {currentValue === null || currentValue === undefined ? (
          <span className="text-muted-foreground italic text-sm">NULL</span>
        ) : (
          defaultRenderer(currentValue)
        )}
      </div>
    </EditPopup>
  );
}