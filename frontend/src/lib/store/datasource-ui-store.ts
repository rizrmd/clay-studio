import { proxy } from "valtio";

interface DatasourceUIState {
  // Track expanded state and tables for each datasource
  datasourceStates: Record<string, {
    isExpanded: boolean;
    tables: string[];
    loadingTables: boolean;
    tablesError: string | null;
  }>;
}

const initialState: DatasourceUIState = {
  datasourceStates: {},
};

export const datasourceUIStore = proxy(initialState);

export const datasourceUIActions = {
  // Initialize a datasource state if it doesn't exist
  ensureDatasourceState: (datasourceId: string, isActive: boolean = false, activeTableName?: string) => {
    if (!datasourceUIStore.datasourceStates[datasourceId]) {
      datasourceUIStore.datasourceStates[datasourceId] = {
        isExpanded: isActive && !!activeTableName,
        tables: [],
        loadingTables: false,
        tablesError: null,
      };
    }
  },

  // Set expanded state
  setExpanded: (datasourceId: string, expanded: boolean) => {
    datasourceUIActions.ensureDatasourceState(datasourceId);
    datasourceUIStore.datasourceStates[datasourceId].isExpanded = expanded;
  },

  // Set tables
  setTables: (datasourceId: string, tables: string[]) => {
    datasourceUIActions.ensureDatasourceState(datasourceId);
    datasourceUIStore.datasourceStates[datasourceId].tables = tables;
  },

  // Set loading state
  setLoadingTables: (datasourceId: string, loading: boolean) => {
    datasourceUIActions.ensureDatasourceState(datasourceId);
    datasourceUIStore.datasourceStates[datasourceId].loadingTables = loading;
  },

  // Set error state
  setTablesError: (datasourceId: string, error: string | null) => {
    datasourceUIActions.ensureDatasourceState(datasourceId);
    datasourceUIStore.datasourceStates[datasourceId].tablesError = error;
  },

  // Clear all state (for cleanup)
  clearAll: () => {
    datasourceUIStore.datasourceStates = {};
  },

  // Clear specific datasource state
  clearDatasource: (datasourceId: string) => {
    delete datasourceUIStore.datasourceStates[datasourceId];
  },
};