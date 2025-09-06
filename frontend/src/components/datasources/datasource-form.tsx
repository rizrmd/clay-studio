import { useState, useEffect } from "react";
import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { datasourcesApi } from "@/lib/api/datasources";
import type { Datasource } from "@/lib/store/datasources-store";

interface DatasourceFormProps {
  projectId: string;
  datasource?: Datasource | null;
  onSuccess: () => void;
  onCancel: () => void;
}

const DATABASE_TYPES = [
  { value: "postgresql", label: "PostgreSQL" },
  { value: "mysql", label: "MySQL" },
  { value: "clickhouse", label: "ClickHouse" },
  { value: "sqlite", label: "SQLite" },
  { value: "oracle", label: "Oracle" },
  { value: "sqlserver", label: "SQL Server" },
] as const;

export function DatasourceForm({ 
  projectId, 
  datasource, 
  onSuccess, 
  onCancel 
}: DatasourceFormProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [formData, setFormData] = useState({
    name: "",
    source_type: "postgresql" as Datasource["source_type"],
    configType: "url" as "url" | "individual",
    // URL format
    connectionUrl: "",
    // Individual fields format
    host: "",
    port: "",
    database: "",
    username: "",
    password: "",
  });

  // Initialize form with existing datasource data
  useEffect(() => {
    if (datasource) {
      setFormData(prev => ({
        ...prev,
        name: datasource.name,
        source_type: datasource.source_type,
        // Try to parse config to determine if it's URL or individual fields
        ...(typeof datasource.config === "string" 
          ? { 
              configType: "url" as const,
              connectionUrl: datasource.config 
            }
          : {
              configType: "individual" as const,
              host: (datasource.config as any)?.host || "",
              port: (datasource.config as any)?.port?.toString() || "",
              database: (datasource.config as any)?.database || "",
              username: (datasource.config as any)?.user || "",
              password: (datasource.config as any)?.password || "",
            }
        ),
      }));
    }
  }, [datasource]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsLoading(true);
    setError(null);

    try {
      // Prepare config based on selected type
      const config = formData.configType === "url" 
        ? formData.connectionUrl
        : {
            host: formData.host,
            port: formData.port ? parseInt(formData.port) : undefined,
            database: formData.database,
            user: formData.username,
            password: formData.password,
          };

      const requestData = {
        name: formData.name,
        source_type: formData.source_type,
        config,
      };

      if (datasource) {
        // Update existing datasource
        await datasourcesApi.update(datasource.id, requestData);
      } else {
        // Create new datasource
        await datasourcesApi.create(projectId, requestData);
      }

      onSuccess();
    } catch (err) {
      console.error("Failed to save datasource:", err);
      setError(
        err instanceof Error ? err.message : "Failed to save datasource"
      );
    } finally {
      setIsLoading(false);
    }
  };

  const getPlaceholderUrl = () => {
    switch (formData.source_type) {
      case "postgresql":
        return "postgresql://username:password@hostname:5432/database";
      case "mysql":
        return "mysql://username:password@hostname:3306/database";
      case "clickhouse":
        return "clickhouse://username:password@hostname:9000/database";
      case "sqlite":
        return "sqlite:///path/to/database.db";
      case "oracle":
        return "oracle://username:password@hostname:1521/service";
      case "sqlserver":
        return "sqlserver://username:password@hostname:1433/database";
      default:
        return "database://username:password@hostname:port/database";
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {/* Basic Information */}
      <div className="space-y-4">
        <div>
          <Label htmlFor="name">Datasource Name</Label>
          <Input
            id="name"
            value={formData.name}
            onChange={(e) => setFormData(prev => ({ ...prev, name: e.target.value }))}
            placeholder="My Database"
            required
          />
        </div>

        <div>
          <Label htmlFor="source_type">Database Type</Label>
          <Select
            value={formData.source_type}
            onValueChange={(value) => 
              setFormData(prev => ({ 
                ...prev, 
                source_type: value as Datasource["source_type"] 
              }))
            }
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {DATABASE_TYPES.map((type) => (
                <SelectItem key={type.value} value={type.value}>
                  {type.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* Configuration Type Toggle */}
        <div>
          <Label>Connection Configuration</Label>
          <div className="flex gap-4 mt-2">
            <label className="flex items-center gap-2">
              <input
                type="radio"
                name="configType"
                value="url"
                checked={formData.configType === "url"}
                onChange={(e) => setFormData(prev => ({ 
                  ...prev, 
                  configType: e.target.value as "url" | "individual" 
                }))}
              />
              <span className="text-sm">Connection URL</span>
            </label>
            <label className="flex items-center gap-2">
              <input
                type="radio"
                name="configType"
                value="individual"
                checked={formData.configType === "individual"}
                onChange={(e) => setFormData(prev => ({ 
                  ...prev, 
                  configType: e.target.value as "url" | "individual" 
                }))}
              />
              <span className="text-sm">Individual Fields</span>
            </label>
          </div>
        </div>
      </div>

      {/* Connection Configuration */}
      {formData.configType === "url" ? (
        <div>
          <Label htmlFor="connectionUrl">Connection URL</Label>
          <Textarea
            id="connectionUrl"
            value={formData.connectionUrl}
            onChange={(e) => setFormData(prev => ({ 
              ...prev, 
              connectionUrl: e.target.value 
            }))}
            placeholder={getPlaceholderUrl()}
            rows={3}
            required
          />
          <p className="text-sm text-muted-foreground mt-1">
            Provide the full connection URL for your database
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <Label htmlFor="host">Host</Label>
              <Input
                id="host"
                value={formData.host}
                onChange={(e) => setFormData(prev => ({ ...prev, host: e.target.value }))}
                placeholder="localhost"
                required
              />
            </div>
            <div>
              <Label htmlFor="port">Port</Label>
              <Input
                id="port"
                type="number"
                value={formData.port}
                onChange={(e) => setFormData(prev => ({ ...prev, port: e.target.value }))}
                placeholder={formData.source_type === "postgresql" ? "5432" : "3306"}
              />
            </div>
          </div>

          <div>
            <Label htmlFor="database">Database Name</Label>
            <Input
              id="database"
              value={formData.database}
              onChange={(e) => setFormData(prev => ({ ...prev, database: e.target.value }))}
              placeholder="my_database"
              required
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <Label htmlFor="username">Username</Label>
              <Input
                id="username"
                value={formData.username}
                onChange={(e) => setFormData(prev => ({ ...prev, username: e.target.value }))}
                placeholder="username"
                autoComplete="username"
              />
            </div>
            <div>
              <Label htmlFor="password">Password</Label>
              <Input
                id="password"
                type="password"
                value={formData.password}
                onChange={(e) => setFormData(prev => ({ ...prev, password: e.target.value }))}
                placeholder="password"
                autoComplete="current-password"
              />
            </div>
          </div>
        </div>
      )}

      {/* Actions */}
      <div className="flex gap-3 justify-end">
        <Button type="button" variant="outline" onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit" disabled={isLoading}>
          {isLoading && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
          {datasource ? "Update Datasource" : "Create Datasource"}
        </Button>
      </div>
    </form>
  );
}