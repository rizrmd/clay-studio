import { useEffect } from "react";
import { useSnapshot } from "valtio";
import { useNavigate } from "react-router-dom";
import { Plus, Database, Loader2, AlertCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { DatasourceCard } from "./datasource-card";
import { DatasourceForm } from "./datasource-form";
import { datasourcesStore, datasourcesActions } from "@/lib/store/datasources-store";

interface DatasourcesMainProps {
  projectId: string;
  mode?: 'list' | 'edit' | 'new';
  datasourceId?: string;
}

export function DatasourcesMain({ projectId, mode = 'list', datasourceId }: DatasourcesMainProps) {
  const snapshot = useSnapshot(datasourcesStore);
  const navigate = useNavigate();

  // Load datasources when component mounts
  useEffect(() => {
    loadDatasources();
  }, [projectId]);

  // Handle mode changes
  useEffect(() => {
    if (mode === 'new') {
      datasourcesActions.showForm(); // Set to create new datasource
    } else if (mode === 'edit' && datasourceId) {
      const datasource = snapshot.datasources.find(d => d.id === datasourceId);
      if (datasource) {
        datasourcesActions.showForm(datasource);
      }
    } else if (mode === 'list') {
      datasourcesActions.hideForm();
    }
  }, [mode, datasourceId, snapshot.datasources]);

  const loadDatasources = async () => {
    if (!projectId) return;
    await datasourcesActions.loadDatasources(projectId);
  };

  const handleCreateNew = () => {
    navigate(`/p/${projectId}/datasources/new`);
  };

  const handleEdit = (datasource: any) => {
    navigate(`/p/${projectId}/datasources/${datasource.id}/edit`);
  };

  const handleDelete = async (datasourceId: string) => {
    if (!confirm("Are you sure you want to delete this datasource?")) {
      return;
    }

    try {
      await datasourcesActions.deleteDatasource(datasourceId);
    } catch (error) {
      console.error("Failed to delete datasource:", error);
    }
  };

  const handleTestConnection = async (datasourceId: string) => {
    try {
      await datasourcesActions.testConnection(datasourceId);
    } catch (error) {
      console.error("Failed to test connection:", error);
    }
  };

  // Determine if we should show the form based on mode or internal state
  const isShowingForm = mode === 'edit' || mode === 'new' || snapshot.editingDatasource !== null;

  return (
    <div className="flex flex-col h-full">
      {isShowingForm ? (
        // Form View
        <div className="flex flex-col h-full">

          {/* Form Content */}
          <div className="flex-1 overflow-y-auto p-4">
            <div className="max-w-2xl mx-auto">
              <DatasourceForm 
                projectId={projectId}
                datasource={snapshot.editingDatasource}
                onSuccess={() => {
                  navigate(`/p/${projectId}`);
                  loadDatasources(); // Reload the list
                }}
                onCancel={() => navigate(`/p/${projectId}`)}
              />
            </div>
          </div>
        </div>
      ) : (
        // List View
        <div className="flex flex-col h-full">
          {/* Header */}
          <div className="border-b p-4">
            <div className="flex items-center justify-between">
              <div>
                <h1 className="text-2xl font-semibold flex items-center gap-2">
                  <Database className="h-6 w-6" />
                  Datasources
                </h1>
                <p className="text-sm text-muted-foreground mt-1">
                  Manage your project's data sources and connections
                </p>
              </div>
              <Button onClick={handleCreateNew}>
                <Plus className="h-4 w-4 mr-2" />
                Add Datasource
              </Button>
            </div>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto p-4">
            {/* Error Alert */}
            {snapshot.error && (
              <Alert variant="destructive" className="mb-4">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>{snapshot.error}</AlertDescription>
              </Alert>
            )}

            {/* Stats */}
            <div className="mb-4">
              <p className="text-sm text-muted-foreground">
                {Array.isArray(snapshot.datasources) ? snapshot.datasources.length : 0} datasource{(Array.isArray(snapshot.datasources) ? snapshot.datasources.length : 0) !== 1 ? 's' : ''}
              </p>
            </div>

            {/* Loading State */}
            {snapshot.isLoading && (
              <div className="flex items-center justify-center py-12">
                <div className="flex items-center gap-2">
                  <Loader2 className="h-6 w-6 animate-spin" />
                  <span>Loading datasources...</span>
                </div>
              </div>
            )}

            {/* Empty State */}
            {!snapshot.isLoading && Array.isArray(snapshot.datasources) && snapshot.datasources.length === 0 && (
              <div className="text-center py-12">
                <Database className="h-16 w-16 mx-auto mb-4 text-muted-foreground" />
                <h3 className="text-xl font-semibold mb-2">No datasources yet</h3>
                <p className="text-sm text-muted-foreground mb-6 max-w-md mx-auto">
                  Add your first datasource to start working with your data. Connect to PostgreSQL, MySQL, ClickHouse, and more.
                </p>
                <Button onClick={handleCreateNew} size="lg">
                  <Plus className="h-4 w-4 mr-2" />
                  Add Your First Datasource
                </Button>
              </div>
            )}

            {/* Datasources Grid */}
            {!snapshot.isLoading && Array.isArray(snapshot.datasources) && snapshot.datasources.length > 0 && (
              <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
                {snapshot.datasources.map((datasource) => (
                  <DatasourceCard
                    key={datasource.id}
                    datasource={datasource}
                    onEdit={() => handleEdit(datasource)}
                    onDelete={() => handleDelete(datasource.id)}
                    onTestConnection={() => handleTestConnection(datasource.id)}
                    isTestingConnection={snapshot.testingConnection === datasource.id}
                  />
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}