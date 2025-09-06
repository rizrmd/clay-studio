import { 
  Database, 
  Edit, 
  Trash2, 
  Wifi, 
  Loader2, 
  MoreHorizontal,
  CheckCircle,
  XCircle,
  HelpCircle
} from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import type { Datasource } from "@/lib/store/datasources-store";

interface DatasourceCardProps {
  datasource: Datasource;
  onEdit: () => void;
  onDelete: () => void;
  onTestConnection: () => void;
  isTestingConnection?: boolean;
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

export function DatasourceCard({
  datasource,
  onEdit,
  onDelete,
  onTestConnection,
  isTestingConnection = false,
}: DatasourceCardProps) {
  const getConnectionStatusIcon = () => {
    if (isTestingConnection) {
      return <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />;
    }

    switch (datasource.connection_status) {
      case "connected":
        return <CheckCircle className="h-4 w-4 text-green-600" />;
      case "error":
        return <XCircle className="h-4 w-4 text-red-600" />;
      case "testing":
        return <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />;
      default:
        return <HelpCircle className="h-4 w-4 text-muted-foreground" />;
    }
  };

  const getConnectionStatusText = () => {
    if (isTestingConnection) return "Testing...";

    switch (datasource.connection_status) {
      case "connected":
        return "Connected";
      case "error":
        return "Connection Error";
      case "testing":
        return "Testing...";
      default:
        return "Unknown";
    }
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString("en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  return (
    <div className="border rounded-lg p-4 hover:shadow-md transition-shadow bg-card">
      <div className="flex items-start justify-between">
        <div className="flex items-start gap-3 flex-1">
          <div className="p-2 bg-muted rounded-lg">
            <Database className="h-5 w-5 text-muted-foreground" />
          </div>
          
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <h3 className="font-semibold truncate">{datasource.name}</h3>
              <Badge 
                variant="secondary" 
                className={`text-xs ${DATABASE_COLORS[datasource.source_type]}`}
              >
                {DATABASE_LABELS[datasource.source_type]}
              </Badge>
            </div>
            
            <div className="flex items-center gap-2 mb-2">
              {getConnectionStatusIcon()}
              <span 
                className={`text-sm ${
                  datasource.connection_status === "connected" 
                    ? "text-green-600" 
                    : datasource.connection_status === "error"
                    ? "text-red-600"
                    : "text-muted-foreground"
                }`}
              >
                {getConnectionStatusText()}
              </span>
              {datasource.connection_status === "error" && datasource.connection_error && (
                <span className="text-xs text-red-600 truncate max-w-xs">
                  ({datasource.connection_error})
                </span>
              )}
            </div>

            <div className="text-xs text-muted-foreground space-y-1">
              <div>Created: {formatDate(datasource.created_at)}</div>
              {datasource.updated_at !== datasource.created_at && (
                <div>Updated: {formatDate(datasource.updated_at)}</div>
              )}
            </div>
          </div>
        </div>

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
              <MoreHorizontal className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={onTestConnection} disabled={isTestingConnection}>
              <Wifi className="h-4 w-4 mr-2" />
              Test Connection
            </DropdownMenuItem>
            <DropdownMenuItem onClick={onEdit}>
              <Edit className="h-4 w-4 mr-2" />
              Edit
            </DropdownMenuItem>
            <DropdownMenuItem 
              onClick={onDelete}
              className="text-red-600 focus:text-red-600"
            >
              <Trash2 className="h-4 w-4 mr-2" />
              Delete
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      {/* Quick Actions */}
      <div className="flex gap-2 mt-3">
        <Button 
          variant="outline" 
          size="sm" 
          onClick={onTestConnection}
          disabled={isTestingConnection}
        >
          {isTestingConnection ? (
            <Loader2 className="h-3 w-3 mr-1 animate-spin" />
          ) : (
            <Wifi className="h-3 w-3 mr-1" />
          )}
          Test
        </Button>
        <Button variant="outline" size="sm" onClick={onEdit}>
          <Edit className="h-3 w-3 mr-1" />
          Edit
        </Button>
      </div>
    </div>
  );
}