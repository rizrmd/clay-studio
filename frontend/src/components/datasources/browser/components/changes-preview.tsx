import React from "react";
import { useSnapshot } from "valtio";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { dataBrowserStore } from "@/lib/store/data-browser-store";

export function ChangesPreview() {
  const dataBrowserSnapshot = useSnapshot(dataBrowserStore);

  if (!dataBrowserSnapshot.isDirty || Object.keys(dataBrowserSnapshot.editingChanges).length === 0) {
    return null;
  }

  const changes = dataBrowserSnapshot.editingChanges;
  const changedRows = Object.keys(changes);

  return (
    <Card className="m-4">
      <CardHeader className="pb-2">
        <CardTitle className="text-sm flex items-center gap-2">
          Pending Changes
          <Badge variant="secondary" className="text-xs">
            {changedRows.length} row{changedRows.length !== 1 ? "s" : ""}
          </Badge>
        </CardTitle>
      </CardHeader>
      <CardContent className="pt-0">
        <div className="space-y-2 max-h-32 overflow-y-auto">
          {changedRows.map((rowId) => (
            <div key={rowId} className="text-xs border rounded p-2 bg-muted/20">
              <div className="font-medium mb-1">Row {rowId}:</div>
              <div className="space-y-1">
                {Object.entries(changes[rowId]).map(([column, value]) => (
                  <div key={column} className="flex items-center gap-2">
                    <span className="text-muted-foreground">{column}:</span>
                    <span className="font-mono bg-yellow-100 px-1 rounded text-xs">
                      {String(value)}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}