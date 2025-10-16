import { proxy } from "valtio";
import { api } from "@/lib/utils/api";

export interface Datasource {
  id: string;
  name: string;
  source_type: "postgresql" | "mysql" | "clickhouse" | "sqlite" | "oracle" | "sqlserver" | "csv" | "excel" | "json";
  config: string | object;
  created_at: string;
  updated_at: string;
  project_id: string;
  schema_info?: string;
  connection_status?: "connected" | "error" | "testing" | "unknown" | "uploaded";
  connection_error?: string;
}

export interface DatasourcesStore {
  datasources: Datasource[];
  isLoading: boolean;
  error: string | null;
  isModalOpen: boolean;
  editingDatasource: Datasource | null;
  testingConnection: string | null; // datasource id being tested
}

export const datasourcesStore = proxy<DatasourcesStore>({
  datasources: [],
  isLoading: false,
  error: null,
  isModalOpen: false,
  editingDatasource: null,
  testingConnection: null,
});

export const datasourcesActions = {
  setDatasources: (datasources: Datasource[]) => {
    datasourcesStore.datasources = Array.isArray(datasources) ? datasources : [];
  },

  addDatasource: (datasource: Datasource) => {
    datasourcesStore.datasources.push(datasource);
  },

  updateDatasource: (id: string, updates: Partial<Datasource>) => {
    const index = datasourcesStore.datasources.findIndex(ds => ds.id === id);
    if (index !== -1) {
      datasourcesStore.datasources[index] = { 
        ...datasourcesStore.datasources[index], 
        ...updates 
      };
    }
  },

  removeDatasource: (id: string) => {
    datasourcesStore.datasources = datasourcesStore.datasources.filter(ds => ds.id !== id);
  },

  setLoading: (loading: boolean) => {
    datasourcesStore.isLoading = loading;
  },

  setError: (error: string | null) => {
    datasourcesStore.error = error;
  },

  openModal: (datasource?: Datasource) => {
    datasourcesStore.isModalOpen = true;
    datasourcesStore.editingDatasource = datasource || null;
  },

  closeModal: () => {
    datasourcesStore.isModalOpen = false;
    datasourcesStore.editingDatasource = null;
  },

  setEditingDatasource: (datasource: Datasource | null) => {
    datasourcesStore.editingDatasource = datasource;
  },

  showForm: (datasource?: Datasource) => {
    datasourcesStore.editingDatasource = datasource || null;
  },

  hideForm: () => {
    datasourcesStore.editingDatasource = null;
  },

  setTestingConnection: (datasourceId: string | null) => {
    datasourcesStore.testingConnection = datasourceId;
  },

  updateConnectionStatus: (id: string, status: Datasource["connection_status"], error?: string) => {
    const index = datasourcesStore.datasources.findIndex(ds => ds.id === id);
    if (index !== -1) {
      datasourcesStore.datasources[index].connection_status = status;
      if (error) {
        datasourcesStore.datasources[index].connection_error = error;
      }
    }
  },

  clearDatasources: () => {
    datasourcesStore.datasources = [];
    datasourcesStore.isLoading = false;
    datasourcesStore.error = null;
    datasourcesStore.editingDatasource = null;
    datasourcesStore.testingConnection = null;
  },

  // API Actions
  async loadDatasources(projectId: string) {
    try {
      datasourcesStore.isLoading = true;
      datasourcesStore.error = null;
      
      const response = await api.get(`/projects/${projectId}/datasources`);
      
      const datasources = Array.isArray(response) ? response : [];
      
      datasourcesActions.setDatasources(datasources);
    } catch (error: any) {
      console.error('DatasourcesStore: Failed to load datasources:', error);
      console.error('DatasourcesStore: Error response:', error?.response);
      datasourcesStore.error = error?.response?.data?.error || 'Failed to load datasources';
      datasourcesActions.setDatasources([]);
    } finally {
      datasourcesStore.isLoading = false;
    }
  },

  async createDatasource(projectId: string, datasource: {name: string, source_type: Datasource["source_type"], config: string | object}) {
    try {
      datasourcesStore.isLoading = true;
      datasourcesStore.error = null;
      
      const newDatasource = await api.post(`/projects/${projectId}/datasources`, datasource);
      
      datasourcesActions.addDatasource(newDatasource);
      return newDatasource;
    } catch (error: any) {
      console.error('Failed to create datasource:', error);
      const errorMessage = error?.response?.data?.error || 'Failed to create datasource';
      datasourcesStore.error = errorMessage;
      throw error;
    } finally {
      datasourcesStore.isLoading = false;
    }
  },

  async updateDatasourceApi(id: string, updates: Partial<Datasource>) {
    try {
      datasourcesStore.isLoading = true;
      datasourcesStore.error = null;
      
      const updatedDatasource = await api.put(`/datasources/${id}`, updates);
      
      datasourcesActions.updateDatasource(id, updatedDatasource);
      return updatedDatasource;
    } catch (error: any) {
      console.error('Failed to update datasource:', error);
      const errorMessage = error?.response?.data?.error || 'Failed to update datasource';
      datasourcesStore.error = errorMessage;
      throw error;
    } finally {
      datasourcesStore.isLoading = false;
    }
  },

  async deleteDatasource(id: string) {
    try {
      datasourcesStore.isLoading = true;
      datasourcesStore.error = null;
      
      await api.delete(`/datasources/${id}`);
      
      datasourcesActions.removeDatasource(id);
    } catch (error: any) {
      console.error('Failed to delete datasource:', error);
      const errorMessage = error?.response?.data?.error || 'Failed to delete datasource';
      datasourcesStore.error = errorMessage;
      throw error;
    } finally {
      datasourcesStore.isLoading = false;
    }
  },

  async testConnection(id: string) {
    try {
      datasourcesStore.testingConnection = id;
      datasourcesActions.updateConnectionStatus(id, "testing");
      
      const result = await api.post(`/datasources/${id}/test`);
      
      if (result.success) {
        datasourcesActions.updateConnectionStatus(id, "connected");
      } else {
        datasourcesActions.updateConnectionStatus(id, "error", result.error);
      }
      
      return result;
    } catch (error: any) {
      console.error('Failed to test connection:', error);
      const errorMessage = error?.response?.data?.error || 'Failed to test connection';
      datasourcesActions.updateConnectionStatus(id, "error", errorMessage);
      throw error;
    } finally {
      datasourcesStore.testingConnection = null;
    }
  },

  async getSchema(id: string) {
    try {
      return await api.get(`/datasources/${id}/schema`);
    } catch (error: any) {
      console.error('Failed to get schema:', error);
      throw error;
    }
  },

  async testConnectionWithConfig(testData: {source_type: Datasource["source_type"]; config: any}) {
    try {
      return await api.post(`/test-connection`, testData);
    } catch (error: any) {
      console.error('Failed to test connection with config:', error);
      throw error;
    }
  },

  async uploadFileDatasource(projectId: string, formData: FormData) {
    try {
      datasourcesStore.isLoading = true;
      datasourcesStore.error = null;

      const newDatasource = await api.post(`/projects/${projectId}/datasources/upload`, formData, {
        headers: {
          'Content-Type': 'multipart/form-data',
        },
      });

      datasourcesActions.addDatasource(newDatasource);
      return newDatasource;
    } catch (error: any) {
      console.error('Failed to upload file datasource:', error);
      const errorMessage = error?.response?.data?.error || 'Failed to upload file datasource';
      datasourcesStore.error = errorMessage;
      throw error;
    } finally {
      datasourcesStore.isLoading = false;
    }
  },

  async previewFile(file: File, sourceType: Datasource["source_type"], options?: any) {
    try {
      const formData = new FormData();
      formData.append('file', file);
      formData.append('source_type', sourceType);

      // Add file-specific options
      if (options) {
        Object.entries(options).forEach(([key, value]) => {
          if (value !== null && value !== undefined && value !== '') {
            formData.append(key, String(value));
          }
        });
      }

      return await api.post(`/projects/preview/datasource`, formData, {
        headers: {
          'Content-Type': 'multipart/form-data',
        },
      });
    } catch (error: any) {
      console.error('Failed to preview file:', error);
      throw error;
    }
  },
};