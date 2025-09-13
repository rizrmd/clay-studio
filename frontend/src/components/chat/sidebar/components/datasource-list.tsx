import { useSnapshot } from "valtio";
import {
  Loader2,
  CheckCircle,
  XCircle,
  HelpCircle,
  ChevronDown,
  ChevronRight,
  Table,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  datasourcesStore,
  datasourcesActions,
  type Datasource,
} from "@/lib/store/datasources-store";
import { datasourcesApi } from "@/lib/api/datasources";
import { cn } from "@/lib/utils";
import { useEffect, useCallback } from "react";
import {
  datasourceUIStore,
  datasourceUIActions,
} from "@/lib/store/datasource-ui-store";

interface DatasourceListProps {
  projectId?: string;
  onDatasourceClick: (datasourceId: string) => void;
  onTableClick?: (datasourceId: string, tableName: string) => void;
  activeDatasourceId?: string;
  activeTableName?: string;
}

const DATABASE_LABELS = {
  postgresql: "PostgreSQL",
  mysql: "MySQL",
  clickhouse: "ClickHouse",
  sqlite: "SQLite",
  oracle: "Oracle",
  sqlserver: "SQL Server",
} as const;

const DATABASE_COLORS = {
  postgresql: "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-300",
  mysql:
    "bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-300",
  clickhouse:
    "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-300",
  sqlite: "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300",
  oracle: "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-300",
  sqlserver:
    "bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-300",
} as const;

function DatasourceItem({
  datasource,
  onDatasourceClick,
  onTableClick,
  isTestingConnection,
  isActive,
  activeTableName,
}: {
  datasource: Datasource;
  onDatasourceClick: (id: string) => void;
  onTableClick?: (datasourceId: string, tableName: string) => void;
  isTestingConnection: boolean;
  isActive?: boolean;
  activeTableName?: string;
}) {
  const datasourceUISnapshot = useSnapshot(datasourceUIStore);

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
    console.log('DatasourceList: Toggle expand clicked', {
      datasourceId: datasource.id,
      currentlyExpanded: isExpanded,
      datasourceState: datasourceState,
      storeState: datasourceUISnapshot.datasourceStates[datasource.id]
    });
    
    if (!isExpanded) {
      console.log('DatasourceList: Expanding datasource', datasource.id);
      datasourceUIActions.setExpanded(datasource.id, true);
      console.log('DatasourceList: After expanding, store state:', datasourceUISnapshot.datasourceStates[datasource.id]);
      await loadTables();
    } else {
      console.log('DatasourceList: Collapsing datasource', datasource.id);
      datasourceUIActions.setExpanded(datasource.id, false);
      console.log('DatasourceList: After collapsing, store state:', datasourceUISnapshot.datasourceStates[datasource.id]);
    }
  };

  const handleTableClick = (tableName: string, e: React.MouseEvent) => {
    e.stopPropagation();
    onTableClick?.(datasource.id, tableName);
  };

  const getConnectionStatusIcon = () => {
    if (isTestingConnection) {
      return <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />;
    }

    switch (datasource.connection_status) {
      case "connected":
        return <CheckCircle className="h-3 w-3 text-green-500" />;
      case "error":
        return <XCircle className="h-3 w-3 text-red-500" />;
      case "testing":
        return (
          <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />
        );
      default:
        return <HelpCircle className="h-3 w-3 text-muted-foreground" />;
    }
  };

  const handleTestConnection = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await datasourcesActions.testConnection(datasource.id);
    } catch (error) {
      console.error("Failed to test connection:", error);
    }
  };

  return (
    <div className={cn("flex flex-col", isExpanded && "flex-1")}>
      <div
        className={cn(
          "group flex items-center justify-between p-2 hover:bg-accent border-t cursor-pointer",
          isActive && "bg-accent/50 border-l-2 border-l-primary"
        )}
        onClick={() => onDatasourceClick(datasource.id)}
      >
        <div className="flex items-center gap-2 min-w-0 flex-1">
          <Button
            size="sm"
            variant="ghost"
            className="h-4 w-4 p-0 hover:bg-transparent rounded-none"
            onClick={handleToggleExpand}
            title={isExpanded ? "Collapse tables" : "Expand tables"}
          >
            {isExpanded ? (
              <ChevronDown className="h-3 w-3 text-muted-foreground" />
            ) : (
              <ChevronRight className="h-3 w-3 text-muted-foreground" />
            )}
          </Button>
          <div className="min-w-0 flex-1">
            <div className="text-sm font-medium truncate">
              {datasource.name}
            </div>
            <div className="flex items-center gap-1 mt-0.5">
              <Badge
                className={cn(
                  "text-[10px] rounded-sm h-[16px] px-1.5 py-0 pointer-events-none",
                  DATABASE_COLORS[datasource.source_type]
                )}
              >
                {DATABASE_LABELS[datasource.source_type]}
              </Badge>

              {tables.length > 0 && (
                <div className="px-2 py-1">
                  <p className="text-xs text-muted-foreground">
                    {tables.length} table{tables.length !== 1 ? "s" : ""}
                  </p>
                </div>
              )}
            </div>
          </div>
        </div>
        <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
          <Button
            size="sm"
            variant="ghost"
            className="h-6 w-6 p-0"
            onClick={handleTestConnection}
            disabled={isTestingConnection}
            title="Test connection"
          >
            {getConnectionStatusIcon()}
          </Button>
        </div>
      </div>

      {/* Expandable tables list */}
      {isExpanded && (
        <>
          <input
            type="search"
            placeholder="Search"
            className="border-0 p-1 pl-2 border-t"
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

              {!loadingTables && !tablesError && tables.length > 0 && (
                <>
                  {tables.map((table) => (
                    <button
                      key={table}
                      onClick={(e) => handleTableClick(table, e)}
                      className={cn(
                        "w-full pl-6 flex items-center gap-2 px-2 py-1 text-left text-xs transition-colors",
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
  onDatasourceClick,
  onTableClick,
  activeDatasourceId,
  activeTableName,
}: DatasourceListProps) {
  const datasourcesSnapshot = useSnapshot(datasourcesStore);

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
          Add one to get started
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
          isTestingConnection={
            datasourcesSnapshot.testingConnection === datasource.id
          }
          isActive={datasource.id === activeDatasourceId}
          activeTableName={
            datasource.id === activeDatasourceId ? activeTableName : undefined
          }
        />
      ))}
    </div>
  );
}
