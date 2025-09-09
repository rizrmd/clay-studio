import { useEffect } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { useSnapshot } from "valtio";
import { Plus, ArrowLeft, Database, Loader2, AlertCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { AppLayout } from "@/components/layout/app-layout";
import { DatasourceCard } from "@/components/datasources/datasource-card";
import { DatasourceForm } from "@/components/datasources/datasource-form";
import { datasourcesStore, datasourcesActions } from "@/lib/store/datasources-store";
import { datasourcesApi } from "@/lib/api/datasources";

export function DatasourcesPage() {
  const { projectId } = useParams<{ projectId: string }>();
  const navigate = useNavigate();
  const snapshot = useSnapshot(datasourcesStore);

  // Load datasources when page loads
  useEffect(() => {
    if (projectId) {
      loadDatasources();
    }
  }, [projectId]);

  const loadDatasources = async () => {
    if (!projectId) return;

    datasourcesActions.setLoading(true);
    datasourcesActions.setError(null);
    
    try {
      const datasources = await datasourcesApi.list(projectId);
      datasourcesActions.setDatasources(datasources);
    } catch (error) {
      console.error("Failed to load datasources:", error);
      datasourcesActions.setError(
        error instanceof Error ? error.message : "Failed to load datasources"
      );
    } finally {
      datasourcesActions.setLoading(false);
    }
  };

  const handleCreateNew = () => {
    datasourcesActions.showForm(); // Show form in create mode
  };

  const handleEdit = (datasource: any) => {
    datasourcesActions.showForm(datasource);
  };

  const handleDelete = async (datasourceId: string) => {
    if (!confirm("Are you sure you want to delete this datasource?")) {
      return;
    }

    try {
      await datasourcesApi.delete(datasourceId);
      datasourcesActions.removeDatasource(datasourceId);
    } catch (error) {
      console.error("Failed to delete datasource:", error);
      datasourcesActions.setError(
        error instanceof Error ? error.message : "Failed to delete datasource"
      );
    }
  };

  const handleTestConnection = async (datasourceId: string) => {
    datasourcesActions.setTestingConnection(datasourceId);
    datasourcesActions.updateConnectionStatus(datasourceId, "testing");
    
    try {
      const result = await datasourcesApi.testConnection(datasourceId);
      datasourcesActions.updateConnectionStatus(
        datasourceId, 
        result.success ? "connected" : "error",
        result.success ? undefined : result.error
      );
    } catch (error) {
      console.error("Failed to test connection:", error);
      datasourcesActions.updateConnectionStatus(
        datasourceId, 
        "error",
        error instanceof Error ? error.message : "Connection test failed"
      );
    } finally {
      datasourcesActions.setTestingConnection(null);
    }
  };

  const handleBackToProject = () => {
    navigate(`/p/${projectId}/new`);
  };

  const isShowingForm = snapshot.editingDatasource !== undefined;

  if (!projectId) {
    return null;
  }

  return (
    <AppLayout>
      <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {isShowingForm ? (
          // Form View
          <div>
            <div className="mb-6">
              <Button 
                variant="ghost" 
                onClick={() => datasourcesActions.hideForm()}
                className="mb-4"
              >
                <ArrowLeft className="h-4 w-4 mr-2" />
                Back to Datasources
              </Button>
              <h1 className="text-3xl font-bold text-gray-900 dark:text-gray-100">
                {snapshot.editingDatasource ? "Edit Datasource" : "Add New Datasource"}
              </h1>
              <p className="mt-2 text-gray-600 dark:text-gray-400">
                Configure your database connection
              </p>
            </div>

            <div className="bg-white dark:bg-gray-800 rounded-lg border p-6">
              <DatasourceForm 
                projectId={projectId}
                datasource={snapshot.editingDatasource}
                onSuccess={() => {
                  datasourcesActions.hideForm();
                  loadDatasources(); // Reload the list
                }}
                onCancel={() => datasourcesActions.hideForm()}
              />
            </div>
          </div>
        ) : (
          // List View
          <div>
            {/* Header */}
            <div className="mb-8">
              <Button 
                variant="ghost" 
                onClick={handleBackToProject}
                className="mb-4"
              >
                <ArrowLeft className="h-4 w-4 mr-2" />
                Back to Project
              </Button>
              <div className="flex items-center justify-between">
                <div>
                  <h1 className="text-3xl font-bold text-gray-900 dark:text-gray-100 flex items-center gap-3">
                    <Database className="h-8 w-8" />
                    Datasources
                  </h1>
                  <p className="mt-2 text-gray-600 dark:text-gray-400">
                    Manage your project's data sources and connections
                  </p>
                </div>
                <Button onClick={handleCreateNew}>
                  <Plus className="h-4 w-4 mr-2" />
                  Add Datasource
                </Button>
              </div>
            </div>

            {/* Error Alert */}
            {snapshot.error && (
              <Alert variant="destructive" className="mb-6">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>{snapshot.error}</AlertDescription>
              </Alert>
            )}

            {/* Stats */}
            <div className="mb-6">
              <p className="text-sm text-muted-foreground">
                {snapshot.datasources.length} datasource{snapshot.datasources.length !== 1 ? 's' : ''}
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
            {!snapshot.isLoading && snapshot.datasources.length === 0 && (
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
            {!snapshot.isLoading && snapshot.datasources.length > 0 && (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
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
        )}
      </div>
    </AppLayout>
  );
}