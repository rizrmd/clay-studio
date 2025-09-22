import { useSnapshot } from "valtio";
import {
  ChevronDown,
  ChevronRight,
  Table,
  Database,
  Edit,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  datasourcesStore,
  type Datasource,
} from "@/lib/store/datasources-store";
import { datasourcesApi } from "@/lib/api/datasources";
import { cn } from "@/lib/utils";
import { useEffect, useCallback, useState, useMemo } from "react";
import {
  datasourceUIStore,
  datasourceUIActions,
} from "@/lib/store/datasource-ui-store";
import { css } from "goober";
import { useNavigate } from "react-router-dom";
import { tabsActions } from "@/lib/store/tabs-store";

interface DatasourceListProps {
  projectId?: string;
  onDatasourceClick: (datasourceId: string) => void;
  onTableClick?: (datasourceId: string, tableName: string) => void;
  onQueryClick?: (datasourceId: string) => void;
  onEditClick?: (datasourceId: string) => void;
  activeDatasourceId?: string;
  activeTableName?: string;
}


function DatasourceItem({
  datasource,
  onTableClick,
  onQueryClick,
  onEditClick,
  isActive,
  activeTableName,
}: {
  datasource: Datasource;
  onDatasourceClick: (id: string) => void;
  onTableClick?: (datasourceId: string, tableName: string) => void;
  onQueryClick?: (datasourceId: string) => void;
  onEditClick?: (datasourceId: string) => void;
  isActive?: boolean;
  activeTableName?: string;
}) {
  const datasourceUISnapshot = useSnapshot(datasourceUIStore);
  const [searchQuery, setSearchQuery] = useState("");

  // Initialize datasource state if needed
  useEffect(() => {
    datasourceUIActions.ensureDatasourceState(
      datasource.id,
      isActive,
      activeTableName
    );
  }, [datasource.id, isActive, activeTableName]);

  const datasourceState = datasourceUISnapshot.datasourceStates[
    datasource.id
  ] || {
    isExpanded: false, // Default to collapsed to avoid the auto-expand issue
    tables: [],
    loadingTables: false,
    tablesError: null,
  };

  const { isExpanded, tables, loadingTables, tablesError } = datasourceState;

  // Filter tables based on search query
  const filteredTables = useMemo(() => {
    if (!searchQuery.trim()) return tables;
    return tables.filter((table) =>
      table.toLowerCase().includes(searchQuery.toLowerCase())
    );
  }, [tables, searchQuery]);

  const loadTables = useCallback(async () => {
    if (tables.length > 0) return; // Already loaded

    try {
      datasourceUIActions.setLoadingTables(datasource.id, true);
      datasourceUIActions.setTablesError(datasource.id, null);
      const tablesData = await datasourcesApi.getTables(datasource.id);
      datasourceUIActions.setTables(datasource.id, tablesData);
    } catch (error: any) {
      console.error("Failed to load tables:", error);
      datasourceUIActions.setTablesError(
        datasource.id,
        error?.response?.data?.error || "Failed to load tables"
      );
    } finally {
      datasourceUIActions.setLoadingTables(datasource.id, false);
    }
  }, [datasource.id, tables.length]);

  // Auto-expand and load tables if this datasource is active (only on initial load or project change)
  useEffect(() => {
    if (isActive && activeTableName) {
      // Only auto-expand if there's no existing state (first time visiting this datasource)
      if (!datasourceUISnapshot.datasourceStates[datasource.id]) {
        datasourceUIActions.setExpanded(datasource.id, true);
        loadTables();
      }
    }
  }, [datasource.id, isActive, activeTableName]);

  const handleToggleExpand = async (e: React.MouseEvent) => {
    e.stopPropagation();

    if (!isExpanded) {
      datasourceUIActions.setExpanded(datasource.id, true);
      await loadTables();
    } else {
      datasourceUIActions.setExpanded(datasource.id, false);
    }
  };

  const handleTableClick = (tableName: string, e: React.MouseEvent) => {
    e.stopPropagation();
    onTableClick?.(datasource.id, tableName);
  };


  return (
    <div className={cn("flex flex-col", isExpanded && "flex-1")}>
      {/* Row 1: Datasource info */}
      <div
        className={cn(
          "group flex items-center justify-between p-2 hover:bg-accent border-t cursor-pointer",
          isActive && "bg-accent/50 border-l-2 border-l-primary"
        )}
        onClick={handleToggleExpand}
      >
        <div className="flex items-center gap-2 min-w-0 flex-1">
          <Button
            size="sm"
            variant="ghost"
            className="h-4 w-4 p-0 hover:bg-transparent rounded-none"
            title={isExpanded ? "Collapse tables" : "Expand tables"}
          >
            {isExpanded ? (
              <ChevronDown className="h-3 w-3 text-muted-foreground" />
            ) : (
              <ChevronRight className="h-3 w-3 text-muted-foreground" />
            )}
          </Button>
          <div className="min-w-0 flex-1">
            <div className="flex flex-row">
              <div className="text-sm flex-1 font-medium truncate">
                {datasource.name}
              </div>
            </div>
            {/* <div className="flex items-center gap-1 mt-0.5">
              <Badge
                className={cn(
                  "text-[10px] rounded-sm h-[16px] px-1.5 py-0 pointer-events-none",
                  DATABASE_COLORS[datasource.source_type]
                )}
              >
                {DATABASE_LABELS[datasource.source_type]}
              </Badge>

            </div> */}

            {/* Row 2: Action buttons */}
            <div
              className={cn(
                "flex items-center gap-1 mt-1",
                css`
                  svg {
                    width: 12px !important;
                  }
                `
              )}
            >
              <Button
                variant="outline"
                onClick={(e) => {
                  e.stopPropagation();
                  onQueryClick?.(datasource.id);
                }}
                className={cn("gap-1 p-0 px-2 h-6 text-xs")}
              >
                <Database />
                Query
              </Button>
              <Button
                variant="outline"
                className={cn("gap-1 p-0 px-2 h-6 text-xs")}
                onClick={(e) => {
                  e.stopPropagation();
                  onEditClick?.(datasource.id);
                }}
              >
                <Edit />
                Edit
              </Button>

              {tables.length > 0 && (
                <div className="px-2 py-1">
                  <p className="text-xs text-muted-foreground">
                    {tables.length} table{tables.length !== 1 ? "s" : ""}
                  </p>
                </div>
              )}
            </div>
            {/* <Button
              size="sm"
              variant="ghost"
              className="h-6 text-xs flex items-center gap-1 w-full justify-start"
              onClick={handleTestConnection}
              disabled={isTestingConnection}
            >
              {getConnectionStatusIcon()}
              <span className="text-xs">
                {datasource.connection_status === "connected"
                  ? "Connected"
                  : datasource.connection_status === "error"
                  ? "Connection failed"
                  : datasource.connection_status === "testing"
                  ? "Testing..."
                  : "Test connection"}
              </span>
            </Button> */}
          </div>
        </div>
      </div>

      {/* Expandable tables list */}
      {isExpanded && (
        <>
          <input
            type="search"
            placeholder="Search tables..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="border-0 p-1 pl-2 border-t text-xs focus:outline-none focus:ring-1 focus:ring-primary/20"
          />
          <div className="space-y-0.5 border-t flex-1 relative overflow-auto">
            <div className="absolute inset-0 flex flex-col">
              {loadingTables && (
                <div className="p-2 space-y-1">
                  {[...Array(3)].map((_, i) => (
                    <div
                      key={i}
                      className="h-6 bg-muted/50 rounded animate-pulse"
                    />
                  ))}
                </div>
              )}

              {tablesError && (
                <div className="p-2">
                  <p className="text-xs text-red-500">{tablesError}</p>
                </div>
              )}

              {!loadingTables && !tablesError && tables.length === 0 && (
                <div className="p-2">
                  <p className="text-xs text-muted-foreground">
                    No tables found
                  </p>
                </div>
              )}

              {!loadingTables &&
                !tablesError &&
                searchQuery.trim() &&
                filteredTables.length === 0 &&
                tables.length > 0 && (
                  <div className="p-2">
                    <p className="text-xs text-muted-foreground">
                      No tables match "{searchQuery}"
                    </p>
                  </div>
                )}

              {!loadingTables && !tablesError && filteredTables.length > 0 && (
                <>
                  {filteredTables.map((table) => (
                    <button
                      key={table}
                      onClick={(e) => handleTableClick(table, e)}
                      className={cn(
                        "w-full pl-6 flex items-center gap-2 px-2 py-1 text-left text-sm transition-colors",
                        activeTableName === table
                          ? "bg-primary text-primary-foreground hover:bg-primary/90"
                          : " hover:bg-accent/50 hover:text-accent-foreground"
                      )}
                      title={table}
                    >
                      <Table
                        className={cn(
                          "h-3 w-3  flex-shrink-0",
                          activeTableName === table
                            ? "text-muted-background"
                            : "text-muted-foreground"
                        )}
                      />
                      <span className="truncate">{table}</span>
                    </button>
                  ))}
                </>
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );
}

export function DatasourceList({
  projectId,
  onDatasourceClick,
  onTableClick,
  onQueryClick,
  onEditClick,
  activeDatasourceId,
  activeTableName,
}: DatasourceListProps) {
  const datasourcesSnapshot = useSnapshot(datasourcesStore);
  const navigate = useNavigate();

  if (datasourcesSnapshot.isLoading) {
    return (
      <div className="p-2">
        <div className="animate-pulse space-y-2">
          <div className="h-8 bg-muted rounded"></div>
          <div className="h-8 bg-muted rounded"></div>
        </div>
      </div>
    );
  }

  if (datasourcesSnapshot.error) {
    return (
      <div className="p-2">
        <p className="text-xs text-red-500">{datasourcesSnapshot.error}</p>
      </div>
    );
  }

  if (datasourcesSnapshot.datasources.length === 0) {
    return (
      <div className="p-2">
        <p className="text-xs text-muted-foreground text-center">
          No datasources yet
        </p>
        <p className="text-xs text-muted-foreground text-center">
          Use{" "}
          <Button
            variant="link"
            className="text-xs p-0 h-auto text-primary underline"
            onClick={() => {
              if (projectId) {
                // Create a new chat tab
                tabsActions.openInNewTab('chat', {
                  projectId,
                  conversationId: 'new',
                }, 'New Chat');
                
                // Navigate to the new route
                navigate(`/p/${projectId}/new`);
              }
            }}
          >
            chat
          </Button>{" "}
          to add new datasource
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-0.5 flex flex-1 flex-col">
      {datasourcesSnapshot.datasources.map((datasource) => (
        <DatasourceItem
          key={datasource.id}
          datasource={datasource}
          onDatasourceClick={onDatasourceClick}
          onTableClick={onTableClick}
          onQueryClick={onQueryClick}
          onEditClick={onEditClick}
          isActive={datasource.id === activeDatasourceId}
          activeTableName={
            datasource.id === activeDatasourceId ? activeTableName : undefined
          }
        />
      ))}
      <div className="p-2 border-t">
        <p className="text-xs text-muted-foreground text-center">
          Use{" "}
          <Button
            variant="link"
            className="text-xs p-0 h-auto text-primary underline"
            onClick={() => {
              if (projectId) {
                // Create a new chat tab
                tabsActions.openInNewTab('chat', {
                  projectId,
                  conversationId: 'new',
                }, 'New Chat');
                
                // Navigate to the new route
                navigate(`/p/${projectId}/new`);
              }
            }}
          >
            chat
          </Button>{" "}
          to add new datasource
        </p>
      </div>
    </div>
  );
}
