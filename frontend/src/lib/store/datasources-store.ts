import { proxy } from "valtio";

export interface Datasource {
  id: string;
  name: string;
  source_type: "postgresql" | "mysql" | "clickhouse" | "sqlite" | "oracle" | "sqlserver";
  config: string | object;
  created_at: string;
  updated_at: string;
  project_id: string;
  schema_info?: string;
  connection_status?: "connected" | "error" | "testing" | "unknown";
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
};