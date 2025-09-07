import { useEffect } from "react";
import { useSnapshot } from "valtio";
import { Plus, Database, Loader2, AlertCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { DatasourceCard } from "./datasource-card";
import { DatasourceForm } from "./datasource-form";
import { datasourcesStore, datasourcesActions } from "@/lib/store/datasources-store";

interface DatasourcesMainProps {
  projectId: string;
}

export function DatasourcesMain({ projectId }: DatasourcesMainProps) {
  console.log('DatasourcesMain: Component rendered with projectId:', projectId);
  const snapshot = useSnapshot(datasourcesStore);

  // Load datasources when component mounts
  useEffect(() => {
    console.log('DatasourcesMain: Loading datasources for project:', projectId);
    loadDatasources();
  }, [projectId]);

  const loadDatasources = async () => {
    if (!projectId) return;
    await datasourcesActions.loadDatasources(projectId);
  };

  const handleCreateNew = () => {
    datasourcesActions.showForm(); // This will set editingDatasource to null for create mode
  };

  const handleEdit = (datasource: any) => {
    datasourcesActions.showForm(datasource);
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

  const isShowingForm = snapshot.editingDatasource !== null;

  return (
    <div className="flex flex-col h-full">
      {isShowingForm ? (
        // Form View
        <div className="flex flex-col h-full">
          {/* Header */}
          <div className="border-b p-4">
            <div className="flex items-center justify-between">
              <div>
                <h1 className="text-2xl font-semibold flex items-center gap-2">
                  <Database className="h-6 w-6" />
                  {snapshot.editingDatasource ? "Edit Datasource" : "Add New Datasource"}
                </h1>
                <p className="text-sm text-muted-foreground mt-1">
                  Configure your database connection
                </p>
              </div>
              <Button 
                variant="outline" 
                onClick={() => datasourcesActions.hideForm()}
              >
                Back to List
              </Button>
            </div>
          </div>

          {/* Form Content */}
          <div className="flex-1 overflow-y-auto p-4">
            <div className="max-w-2xl mx-auto">
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