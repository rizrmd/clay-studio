import { useState } from "react";
import { useSnapshot } from "valtio";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Save, RotateCcw, Code } from "lucide-react";
import { dataBrowserStore, dataBrowserActions, type DataBrowserStore } from "@/lib/store/data-browser-store";

const generateSQLQueries = (dataBrowserSnapshot: ReturnType<typeof useSnapshot<DataBrowserStore>>): string[] => {
  const queries: string[] = [];
  const tableName = dataBrowserSnapshot.selectedTable;
  
  if (!tableName) return queries;

  // Generate UPDATE queries for edited rows
  Object.entries(dataBrowserSnapshot.editingChanges).forEach(([rowId, changes]) => {
    if (changes && Object.keys(changes).length > 0) {
      const setParts = Object.entries(changes).map(([column, value]) => {
        const formattedValue = typeof value === 'string' ? `'${value.replace(/'/g, "''")}'` : value;
        return `${column} = ${formattedValue}`;
      }).join(', ');
      
      queries.push(`UPDATE ${tableName} SET ${setParts} WHERE id = '${rowId}';`);
    }
  });

  // Generate INSERT queries for new rows
  dataBrowserSnapshot.pendingNewRows.forEach((row) => {
    const { __tempId, __isNewRow, ...cleanRow } = row;
    const columns = Object.keys(cleanRow).join(', ');
    const values = Object.values(cleanRow).map(value => {
      if (value === null || value === undefined) return 'NULL';
      return typeof value === 'string' ? `'${value.replace(/'/g, "''")}'` : value;
    }).join(', ');
    
    queries.push(`INSERT INTO ${tableName} (${columns}) VALUES (${values});`);
  });

  return queries;
};

export function EditToolbar() {
  const dataBrowserSnapshot = useSnapshot(dataBrowserStore);
  const [showQueryDialog, setShowQueryDialog] = useState(false);

  const changesCount = Object.keys(dataBrowserSnapshot.editingChanges).reduce(
    (total, rowId) => total + Object.keys(dataBrowserSnapshot.editingChanges[rowId]).length,
    0
  );
  
  const newRowsCount = dataBrowserSnapshot.pendingNewRows.length;
  const totalChanges = changesCount + newRowsCount;

  const sqlQueries = generateSQLQueries(dataBrowserSnapshot);

  const handleSaveChanges = () => {
    dataBrowserActions.saveChanges();
  };

  const handleDiscardChanges = () => {
    const confirmed = window.confirm(
      "Are you sure you want to discard all changes? This action cannot be undone."
    );
    if (confirmed) {
      dataBrowserActions.discardChanges();
    }
  };

  const handleShowQuery = () => {
    setShowQueryDialog(true);
  };

  // Only show toolbar when there are changes or errors
  if (totalChanges === 0 && !dataBrowserSnapshot.error) {
    return null;
  }

  return (
    <div className="flex items-center gap-2 p-2 border-b bg-muted/30">
      {totalChanges > 0 && (
        <div className="flex items-center gap-2">
          <div className="flex gap-1">
            {newRowsCount > 0 && (
              <Badge variant="secondary" className="text-xs ">
                {newRowsCount} new row{newRowsCount !== 1 ? "s" : ""}
              </Badge>
            )}
            {changesCount > 0 && (
              <Badge variant="secondary" className="text-xs">
                {changesCount} edit{changesCount !== 1 ? "s" : ""}
              </Badge>
            )}
          </div>

          <Button
            variant="ghost"
            size="sm"
            onClick={handleShowQuery}
            disabled={totalChanges === 0}
          >
            <Code className="h-4 w-4 mr-1" />
            Show Query
          </Button>

          <Button
            variant="default"
            size="sm"
            onClick={handleSaveChanges}
            disabled={!dataBrowserSnapshot.isDirty || dataBrowserSnapshot.editingInProgress}
          >
            <Save className="h-4 w-4 mr-1" />
            {dataBrowserSnapshot.editingInProgress ? "Saving..." : "Save Changes"}
          </Button>

          <Button
            variant="outline"
            size="sm"
            onClick={handleDiscardChanges}
            disabled={!dataBrowserSnapshot.isDirty || dataBrowserSnapshot.editingInProgress}
          >
            <RotateCcw className="h-4 w-4 mr-1" />
            Discard
          </Button>
        </div>
      )}
      
      <Dialog open={showQueryDialog} onOpenChange={setShowQueryDialog}>
        <DialogContent className="max-w-4xl max-h-[80vh]">
          <DialogHeader>
            <DialogTitle>SQL Queries to be executed</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            {sqlQueries.length > 0 ? (
              <div className="space-y-2">
                <p className="text-sm text-muted-foreground">
                  The following SQL queries will be executed when you save changes:
                </p>
                <div className="bg-muted/50 rounded-lg p-4 max-h-96 overflow-auto">
                  <pre className="text-sm font-mono whitespace-pre-wrap">
                    {sqlQueries.join('\n\n')}
                  </pre>
                </div>
                <p className="text-xs text-muted-foreground">
                  Note: Actual queries may vary slightly based on database-specific syntax
                </p>
              </div>
            ) : (
              <p className="text-muted-foreground">No pending changes to show</p>
            )}
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}