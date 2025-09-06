import { useEffect } from "react";
import { useSnapshot } from "valtio";
import { Plus, Database, Loader2, AlertCircle } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { datasourcesStore, datasourcesActions } from "@/lib/store/datasources-store";
import { datasourcesApi } from "@/lib/api/datasources";
import { DatasourceForm } from "./datasource-form";
import { DatasourceCard } from "./datasource-card";

interface DatasourcesModalProps {
  projectId: string;
}

export function DatasourcesModal({ projectId }: DatasourcesModalProps) {
  const snapshot = useSnapshot(datasourcesStore);

  // Load datasources when modal opens
  useEffect(() => {
    if (snapshot.isModalOpen && !snapshot.isLoading && snapshot.datasources.length === 0) {
      loadDatasources();
    }
  }, [snapshot.isModalOpen, projectId]);

  const loadDatasources = async () => {
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
    datasourcesActions.openModal(); // This will set editingDatasource to null for create mode
  };

  const handleEdit = (datasource: any) => {
    datasourcesActions.openModal(datasource);
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

  const isShowingForm = snapshot.editingDatasource !== undefined;

  return (
    <Dialog 
      open={snapshot.isModalOpen} 
      onOpenChange={(open) => {
        if (!open) {
          datasourcesActions.closeModal();
        }
      }}
    >
      <DialogContent className="max-w-4xl max-h-[90vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Database className="h-5 w-5" />
            {isShowingForm 
              ? (snapshot.editingDatasource ? "Edit Datasource" : "Add New Datasource")
              : "Datasources"
            }
          </DialogTitle>
          <DialogDescription>
            {isShowingForm 
              ? "Configure your database connection"
              : "Manage your project's data sources and connections"
            }
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-hidden">
          {isShowingForm ? (
            <DatasourceForm 
              projectId={projectId}
              datasource={snapshot.editingDatasource}
              onSuccess={() => {
                datasourcesActions.closeModal();
                loadDatasources(); // Reload the list
              }}
              onCancel={() => datasourcesActions.closeModal()}
            />
          ) : (
            <div className="h-full flex flex-col">
              {/* Error Alert */}
              {snapshot.error && (
                <Alert variant="destructive" className="mb-4">
                  <AlertCircle className="h-4 w-4" />
                  <AlertDescription>{snapshot.error}</AlertDescription>
                </Alert>
              )}

              {/* Header Actions */}
              <div className="flex items-center justify-between mb-4">
                <p className="text-sm text-muted-foreground">
                  {snapshot.datasources.length} datasource{snapshot.datasources.length !== 1 ? 's' : ''}
                </p>
                <Button onClick={handleCreateNew} size="sm">
                  <Plus className="h-4 w-4 mr-2" />
                  Add Datasource
                </Button>
              </div>

              {/* Loading State */}
              {snapshot.isLoading && (
                <div className="flex-1 flex items-center justify-center">
                  <div className="flex items-center gap-2">
                    <Loader2 className="h-6 w-6 animate-spin" />
                    <span>Loading datasources...</span>
                  </div>
                </div>
              )}

              {/* Empty State */}
              {!snapshot.isLoading && snapshot.datasources.length === 0 && (
                <div className="flex-1 flex items-center justify-center">
                  <div className="text-center">
                    <Database className="h-12 w-12 mx-auto mb-4 text-muted-foreground" />
                    <h3 className="text-lg font-semibold mb-2">No datasources yet</h3>
                    <p className="text-sm text-muted-foreground mb-4">
                      Add your first datasource to start working with your data
                    </p>
                    <Button onClick={handleCreateNew}>
                      <Plus className="h-4 w-4 mr-2" />
                      Add Your First Datasource
                    </Button>
                  </div>
                </div>
              )}

              {/* Datasources List */}
              {!snapshot.isLoading && snapshot.datasources.length > 0 && (
                <div className="flex-1 overflow-y-auto space-y-3">
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
      </DialogContent>
    </Dialog>
  );
}