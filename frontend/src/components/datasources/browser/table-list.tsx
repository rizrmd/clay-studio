import { useState } from "react";
import { Search, Table } from "lucide-react";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

interface TableListProps {
  tables: readonly string[];
  selectedTable: string | null;
  onTableSelect: (tableName: string) => void;
  loading: boolean;
}

export function TableList({ tables, selectedTable, onTableSelect, loading }: TableListProps) {
  const [searchQuery, setSearchQuery] = useState("");

  const filteredTables = tables.filter(table =>
    table.toLowerCase().includes(searchQuery.toLowerCase())
  );

  if (loading) {
    return (
      <div className="p-3 space-y-2">
        {[...Array(5)].map((_, i) => (
          <div key={i} className="h-8 bg-muted rounded animate-pulse" />
        ))}
      </div>
    );
  }

  return (
    <div className="p-3">
      {/* Search */}
      <div className="relative mb-3">
        <Search className="absolute left-2 top-2.5 h-3 w-3 text-muted-foreground" />
        <Input
          placeholder="Search tables..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="pl-7 h-8 text-xs"
        />
      </div>

      {/* Tables List */}
      <div className="space-y-0.5">
        {filteredTables.length === 0 ? (
          <div className="text-center py-4">
            <p className="text-xs text-muted-foreground">
              {searchQuery ? "No tables match your search" : "No tables found"}
            </p>
          </div>
        ) : (
          filteredTables.map((table) => (
            <button
              key={table}
              onClick={() => onTableSelect(table)}
              className={cn(
                "w-full flex items-center gap-2 px-2 py-2 text-left text-sm rounded-md transition-colors",
                "hover:bg-accent hover:text-accent-foreground",
                selectedTable === table && "bg-accent text-accent-foreground"
              )}
              title={table}
            >
              <Table className="h-4 w-4 text-muted-foreground flex-shrink-0" />
              <span className="truncate">{table}</span>
            </button>
          ))
        )}
      </div>

      {/* Table Count */}
      {tables.length > 0 && (
        <div className="mt-3 pt-2 border-t">
          <p className="text-xs text-muted-foreground">
            {filteredTables.length} of {tables.length} tables
            {searchQuery && ` matching "${searchQuery}"`}
          </p>
        </div>
      )}
    </div>
  );
}