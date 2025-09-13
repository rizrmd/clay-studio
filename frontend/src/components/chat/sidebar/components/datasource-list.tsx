import { useSnapshot } from "valtio";
import { Database, Loader2, CheckCircle, XCircle, HelpCircle } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { datasourcesStore, datasourcesActions, type Datasource } from "@/lib/store/datasources-store";
import { cn } from "@/lib/utils";

interface DatasourceListProps {
  projectId?: string;
  onDatasourceClick: (datasourceId: string) => void;
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
  mysql: "bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-300",
  clickhouse: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-300",
  sqlite: "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300",
  oracle: "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-300",
  sqlserver: "bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-300",
} as const;

function DatasourceItem({ 
  datasource, 
  onDatasourceClick, 
  isTestingConnection 
}: { 
  datasource: Datasource; 
  onDatasourceClick: (id: string) => void;
  isTestingConnection: boolean;
}) {
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
        return <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />;
      default:
        return <HelpCircle className="h-3 w-3 text-muted-foreground" />;
    }
  };

  const handleTestConnection = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await datasourcesActions.testConnection(datasource.id);
    } catch (error) {
      console.error('Failed to test connection:', error);
    }
  };

  return (
    <div
      className="group flex items-center justify-between p-2 hover:bg-accent rounded-md cursor-pointer"
      onClick={() => onDatasourceClick(datasource.id)}
    >
      <div className="flex items-center gap-2 min-w-0 flex-1">
        <Database className="h-4 w-4 text-muted-foreground flex-shrink-0" />
        <div className="min-w-0 flex-1">
          <div className="text-sm font-medium truncate">{datasource.name}</div>
          <div className="flex items-center gap-1 mt-0.5">
            <Badge className={cn("text-xs px-1.5 py-0", DATABASE_COLORS[datasource.source_type])}>
              {DATABASE_LABELS[datasource.source_type]}
            </Badge>
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
  );
}

export function DatasourceList({ onDatasourceClick }: DatasourceListProps) {
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
    <div className="p-1 space-y-0.5">
      {datasourcesSnapshot.datasources.map((datasource) => (
        <DatasourceItem
          key={datasource.id}
          datasource={datasource}
          onDatasourceClick={onDatasourceClick}
          isTestingConnection={datasourcesSnapshot.testingConnection === datasource.id}
        />
      ))}
    </div>
  );
}