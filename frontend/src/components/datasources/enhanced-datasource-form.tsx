import { useState, useEffect } from "react";
import { Loader2, Trash2, FileSpreadsheet, FileJson, FileText } from "lucide-react";
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
import { Checkbox } from "@/components/ui/checkbox";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  datasourcesActions,
  type Datasource,
} from "@/lib/store/datasources-store";

interface EnhancedDatasourceFormProps {
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

const FILE_TYPES = [
  { value: "csv", label: "CSV / TSV", icon: FileText },
  { value: "excel", label: "Excel", icon: FileSpreadsheet },
  { value: "json", label: "JSON", icon: FileJson },
] as const;


export function EnhancedDatasourceForm({
  projectId,
  datasource,
  onSuccess,
  onCancel,
}: EnhancedDatasourceFormProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<{
    success: boolean;
    message: string;
    error?: string;
  } | null>(null);
  const [isTesting, setIsTesting] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
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
    schema: "public",
    // File upload format
    file: null as File | null,
    // CSV options
    delimiter: ",",
    has_header: true,
    // Excel options
    sheet_name: "",
    header_row: "",
    // JSON options
    root_path: "",
    array_path: "",
  });

  const [dragActive, setDragActive] = useState(false);

  const isFileType = FILE_TYPES.some(type => type.value === formData.source_type);
  const isDatabaseType = DATABASE_TYPES.some(type => type.value === formData.source_type);

  // Initialize form with existing datasource data
  useEffect(() => {
    if (datasource) {
      setFormData((prev) => ({
        ...prev,
        name: datasource.name,
        source_type: datasource.source_type,
        // Try to parse config to determine if it's URL or individual fields
        ...(typeof datasource.config === "string"
          ? {
              configType: "url" as const,
              connectionUrl: datasource.config,
            }
          : {
              configType: "individual" as const,
              host: (datasource.config as any)?.host || "",
              port: (datasource.config as any)?.port?.toString() || "",
              database: (datasource.config as any)?.database || "",
              username:
                (datasource.config as any)?.user ||
                (datasource.config as any)?.username ||
                "",
              password: (datasource.config as any)?.password || "",
              schema: (datasource.config as any)?.schema || "public",
              // File options
              delimiter: (datasource.config as any)?.delimiter || ",",
              has_header: (datasource.config as any)?.has_header ?? true,
              sheet_name: (datasource.config as any)?.sheet_name || "",
              header_row: (datasource.config as any)?.header_row?.toString() || "",
              root_path: (datasource.config as any)?.root_path || "",
              array_path: (datasource.config as any)?.array_path || "",
            }),
      }));
    }
  }, [datasource]);

  const handleTestConnection = async () => {
    setIsTesting(true);
    setTestResult(null);

    try {
      // Prepare config based on selected type
      const config =
        formData.configType === "url"
          ? formData.connectionUrl
          : {
              host: formData.host,
              port: formData.port ? parseInt(formData.port) : undefined,
              database: formData.database,
              user: formData.username,
              password: formData.password,
              ...(formData.source_type === "postgresql" && formData.schema !== "public" ? { schema: formData.schema } : {}),
            };

      const testData = {
        source_type: formData.source_type,
        config,
      };

      // Test connection with current form data
      const result = await datasourcesActions.testConnectionWithConfig(testData);
      setTestResult(result);
    } catch (err) {
      console.error("Failed to test connection:", err);
      setTestResult({
        success: false,
        message: "Test failed",
        error: err instanceof Error ? err.message : "Unknown error",
      });
    } finally {
      setIsTesting(false);
    }
  };

  const handleDelete = async () => {
    if (!datasource) return;

    setIsDeleting(true);
    try {
      await datasourcesActions.deleteDatasource(datasource.id);
      onSuccess();
    } catch (err) {
      console.error("Failed to delete datasource:", err);
      setError(err instanceof Error ? err.message : "Failed to delete datasource");
    } finally {
      setIsDeleting(false);
      setShowDeleteDialog(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsLoading(true);
    setError(null);

    try {
      if (isFileType && !formData.file && !datasource) {
        setError("Please select a file to upload");
        setIsLoading(false);
        return;
      }

      if (isFileType && formData.file) {
        // Handle file upload
        const formDataToSend = new FormData();
        formDataToSend.append("name", formData.name);
        formDataToSend.append("source_type", formData.source_type);
        formDataToSend.append("file", formData.file);

        // Add file-specific options
        if (formData.source_type === "csv") {
          formDataToSend.append("delimiter", formData.delimiter);
          formDataToSend.append("has_header", formData.has_header.toString());
        } else if (formData.source_type === "excel") {
          if (formData.sheet_name) {
            formDataToSend.append("sheet_name", formData.sheet_name);
          }
          if (formData.header_row) {
            formDataToSend.append("header_row", formData.header_row);
          }
        } else if (formData.source_type === "json") {
          if (formData.root_path) {
            formDataToSend.append("root_path", formData.root_path);
          }
          if (formData.array_path) {
            formDataToSend.append("array_path", formData.array_path);
          }
        }

        await datasourcesActions.uploadFileDatasource(projectId, formDataToSend);
      } else {
        // Handle database connection (existing logic)
        const config =
          formData.configType === "url"
            ? formData.connectionUrl
            : {
                host: formData.host,
                port: formData.port ? parseInt(formData.port) : undefined,
                database: formData.database,
                user: formData.username,
                password: formData.password,
                ...(formData.source_type === "postgresql" && formData.schema !== "public" ? { schema: formData.schema } : {}),
              };

        const createData = {
          name: formData.name,
          source_type: formData.source_type,
          config,
        };

        if (datasource) {
          await datasourcesActions.updateDatasourceApi(datasource.id, createData);
        } else {
          await datasourcesActions.createDatasource(projectId, createData);
        }
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

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      setFormData(prev => ({ ...prev, file }));
    }
  };

  const handleDrag = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.type === "dragenter" || e.type === "dragover") {
      setDragActive(true);
    } else if (e.type === "dragleave") {
      setDragActive(false);
    }
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragActive(false);

    if (e.dataTransfer.files && e.dataTransfer.files[0]) {
      setFormData(prev => ({ ...prev, file: e.dataTransfer.files[0] }));
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

  const getFileTypeIcon = () => {
    const fileType = FILE_TYPES.find(type => type.value === formData.source_type);
    return fileType?.icon || FileText;
  };

  const FileTypeIcon = getFileTypeIcon();

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
            onChange={(e) =>
              setFormData((prev) => ({ ...prev, name: e.target.value }))
            }
            placeholder="My Datasource"
            required
          />
        </div>

        <div>
          <Label htmlFor="source_type">Datasource Type</Label>
          <Select
            value={formData.source_type}
            onValueChange={(value) => {
              setFormData((prev) => ({
                ...prev,
                source_type: value as Datasource["source_type"],
              }));
            }}
          >
            <SelectTrigger>
              <SelectValue placeholder="Select datasource type" />
            </SelectTrigger>
            <SelectContent>
              <div className="px-2 py-1.5 text-sm font-semibold text-muted-foreground">
                Databases
              </div>
              {DATABASE_TYPES.map((type) => (
                <SelectItem key={type.value} value={type.value}>
                  {type.label}
                </SelectItem>
              ))}
              <div className="px-2 py-1.5 text-sm font-semibold text-muted-foreground mt-2">
                Files
              </div>
              {FILE_TYPES.map((type) => (
                <SelectItem key={type.value} value={type.value}>
                  <div className="flex items-center gap-2">
                    <type.icon className="w-4 h-4" />
                    {type.label}
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* File Upload Configuration */}
      {isFileType && (
        <div className="space-y-4">
          <div>
            <Label htmlFor="file">File Upload</Label>
            <div
              className={`relative border-2 border-dashed rounded-lg p-6 text-center transition-colors ${
                dragActive
                  ? "border-primary bg-primary/5"
                  : "border-muted-foreground/25 hover:border-primary/50"
              }`}
              onDragEnter={handleDrag}
              onDragLeave={handleDrag}
              onDragOver={handleDrag}
              onDrop={handleDrop}
            >
              <input
                id="file"
                type="file"
                className="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                onChange={handleFileChange}
                accept={
                  formData.source_type === "csv"
                    ? ".csv,.tsv,.txt"
                    : formData.source_type === "excel"
                    ? ".xlsx,.xls,.xlsm"
                    : ".json,.jsonl"
                }
              />

              <div className="flex flex-col items-center gap-2">
                <FileTypeIcon className="w-8 h-8 text-muted-foreground" />
                <div className="text-sm">
                  {formData.file ? (
                    <div className="flex items-center gap-2 text-foreground">
                      <span>{formData.file.name}</span>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        onClick={() => setFormData(prev => ({ ...prev, file: null }))}
                      >
                        <Trash2 className="w-4 h-4" />
                      </Button>
                    </div>
                  ) : (
                    <div>
                      <p className="text-muted-foreground">
                        Drag and drop your file here, or click to browse
                      </p>
                      <p className="text-xs text-muted-foreground mt-1">
                        {formData.source_type === "csv" && "CSV, TSV, or TXT files"}
                        {formData.source_type === "excel" && "Excel files (.xlsx, .xls, .xlsm)"}
                        {formData.source_type === "json" && "JSON files (.json, .jsonl)"}
                      </p>
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>

          {/* File-specific options */}
          {formData.source_type === "csv" && (
            <div className="space-y-4">
              <div>
                <Label htmlFor="delimiter">Delimiter</Label>
                <Select
                  value={formData.delimiter}
                  onValueChange={(value) =>
                    setFormData((prev) => ({ ...prev, delimiter: value }))
                  }
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value=",">Comma (,)</SelectItem>
                    <SelectItem value="\t">Tab (\t)</SelectItem>
                    <SelectItem value=";">Semicolon (;)</SelectItem>
                    <SelectItem value="|">Pipe (|)</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="flex items-center space-x-2">
                <Checkbox
                  id="has_header"
                  checked={formData.has_header}
                  onCheckedChange={(checked) =>
                    setFormData((prev) => ({ ...prev, has_header: checked as boolean }))
                  }
                />
                <Label htmlFor="has_header">First row contains headers</Label>
              </div>
            </div>
          )}

          {formData.source_type === "excel" && (
            <div className="space-y-4">
              <div>
                <Label htmlFor="sheet_name">Sheet Name (optional)</Label>
                <Input
                  id="sheet_name"
                  value={formData.sheet_name}
                  onChange={(e) =>
                    setFormData((prev) => ({ ...prev, sheet_name: e.target.value }))
                  }
                  placeholder="Sheet1"
                />
                <p className="text-xs text-muted-foreground mt-1">
                  Leave blank to use the first sheet
                </p>
              </div>

              <div>
                <Label htmlFor="header_row">Header Row (optional)</Label>
                <Input
                  id="header_row"
                  type="number"
                  value={formData.header_row}
                  onChange={(e) =>
                    setFormData((prev) => ({ ...prev, header_row: e.target.value }))
                  }
                  placeholder="1"
                  min="1"
                />
                <p className="text-xs text-muted-foreground mt-1">
                  Row number that contains headers (1-based)
                </p>
              </div>
            </div>
          )}

          {formData.source_type === "json" && (
            <div className="space-y-4">
              <div>
                <Label htmlFor="root_path">Root Path (optional)</Label>
                <Input
                  id="root_path"
                  value={formData.root_path}
                  onChange={(e) =>
                    setFormData((prev) => ({ ...prev, root_path: e.target.value }))
                  }
                  placeholder="data.items"
                />
                <p className="text-xs text-muted-foreground mt-1">
                  JSON path to the root object/array (e.g., "data.items")
                </p>
              </div>

              <div>
                <Label htmlFor="array_path">Array Path (optional)</Label>
                <Input
                  id="array_path"
                  value={formData.array_path}
                  onChange={(e) =>
                    setFormData((prev) => ({ ...prev, array_path: e.target.value }))
                  }
                  placeholder="records"
                />
                <p className="text-xs text-muted-foreground mt-1">
                  Path to the array containing data records (relative to root path)
                </p>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Database Configuration */}
      {isDatabaseType && (
        <>
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
                  onChange={(e) =>
                    setFormData((prev) => ({
                      ...prev,
                      configType: e.target.value as "url" | "individual",
                    }))
                  }
                />
                <span className="text-sm">Connection URL</span>
              </label>
              <label className="flex items-center gap-2">
                <input
                  type="radio"
                  name="configType"
                  value="individual"
                  checked={formData.configType === "individual"}
                  onChange={(e) =>
                    setFormData((prev) => ({
                      ...prev,
                      configType: e.target.value as "url" | "individual",
                    }))
                  }
                />
                <span className="text-sm">Individual Fields</span>
              </label>
            </div>
          </div>

          {/* Connection Configuration */}
          {formData.configType === "url" ? (
            <div>
              <Label htmlFor="connectionUrl">Connection URL</Label>
              <Textarea
                id="connectionUrl"
                value={formData.connectionUrl}
                onChange={(e) =>
                  setFormData((prev) => ({
                    ...prev,
                    connectionUrl: e.target.value,
                  }))
                }
                placeholder={getPlaceholderUrl()}
                rows={3}
                required
              />
            </div>
          ) : (
            <div className="grid grid-cols-2 gap-4">
              <div>
                <Label htmlFor="host">Host</Label>
                <Input
                  id="host"
                  value={formData.host}
                  onChange={(e) =>
                    setFormData((prev) => ({ ...prev, host: e.target.value }))
                  }
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
                  onChange={(e) =>
                    setFormData((prev) => ({ ...prev, port: e.target.value }))
                  }
                  placeholder={
                    formData.source_type === "postgresql"
                      ? "5432"
                      : formData.source_type === "mysql"
                      ? "3306"
                      : formData.source_type === "clickhouse"
                      ? "9000"
                      : formData.source_type === "oracle"
                      ? "1521"
                      : formData.source_type === "sqlserver"
                      ? "1433"
                      : "1433"
                  }
                  required
                />
              </div>

              <div>
                <Label htmlFor="database">Database</Label>
                <Input
                  id="database"
                  value={formData.database}
                  onChange={(e) =>
                    setFormData((prev) => ({ ...prev, database: e.target.value }))
                  }
                  placeholder="mydatabase"
                  required
                />
              </div>

              <div>
                <Label htmlFor="username">Username</Label>
                <Input
                  id="username"
                  value={formData.username}
                  onChange={(e) =>
                    setFormData((prev) => ({ ...prev, username: e.target.value }))
                  }
                  placeholder="admin"
                  required
                />
              </div>

              <div className="col-span-2">
                <Label htmlFor="password">Password</Label>
                <Input
                  id="password"
                  type="password"
                  value={formData.password}
                  onChange={(e) =>
                    setFormData((prev) => ({ ...prev, password: e.target.value }))
                  }
                  placeholder="password"
                  required
                />
              </div>

              {formData.source_type === "postgresql" && (
                <div className="col-span-2">
                  <Label htmlFor="schema">Schema</Label>
                  <Input
                    id="schema"
                    value={formData.schema}
                    onChange={(e) =>
                      setFormData((prev) => ({ ...prev, schema: e.target.value }))
                    }
                    placeholder="public"
                  />
                </div>
              )}
            </div>
          )}
        </>
      )}

      {/* Test Connection Button */}
      {isDatabaseType && (
        <div className="flex items-center gap-4">
          <Button
            type="button"
            variant="outline"
            onClick={handleTestConnection}
            disabled={isTesting}
            className="flex items-center gap-2"
          >
            {isTesting ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : null}
            Test Connection
          </Button>

          {testResult && (
            <Alert
              variant={testResult.success ? "default" : "destructive"}
              className="flex-1"
            >
              <AlertDescription>
                {testResult.message}
                {testResult.error && (
                  <div className="mt-1 text-sm opacity-80">
                    Error: {testResult.error}
                  </div>
                )}
              </AlertDescription>
            </Alert>
          )}
        </div>
      )}

      {/* Actions */}
      <div className="flex justify-between">
        <div>
          {datasource && (
            <Button
              type="button"
              variant="destructive"
              onClick={() => setShowDeleteDialog(true)}
              disabled={isDeleting}
              className="flex items-center gap-2"
            >
              {isDeleting ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Trash2 className="w-4 h-4" />
              )}
              Delete Datasource
            </Button>
          )}
        </div>

        <div className="flex gap-2">
          <Button
            type="button"
            variant="outline"
            onClick={onCancel}
            disabled={isLoading}
          >
            Cancel
          </Button>
          <Button type="submit" disabled={isLoading}>
            {isLoading ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                {datasource ? "Updating..." : "Creating..."}
              </>
            ) : (
              <>{datasource ? "Update Datasource" : "Create Datasource"}</>
            )}
          </Button>
        </div>
      </div>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Datasource</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete "{datasource?.name}"? This action cannot be
              undone and will remove all associated data.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={handleDelete} className="bg-destructive text-destructive-foreground hover:bg-destructive/90">
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </form>
  );
}