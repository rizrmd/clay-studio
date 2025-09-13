import { proxy } from "valtio";
import { datasourcesApi, type QueryResult, type TableDataRequest } from "@/lib/api/datasources";

export interface DataBrowserStore {
  // Current datasource
  selectedDatasourceId: string | null;
  
  // Tables
  tables: string[];
  selectedTable: string | null;
  tablesLoading: boolean;
  
  // Table data
  tableData: QueryResult | null;
  currentPage: number;
  pageSize: number;
  totalRows: number;
  sortColumn: string | null;
  sortDirection: "asc" | "desc";
  filters: Record<string, any>;
  dataLoading: boolean;
  
  // Query execution
  currentQuery: string;
  queryResults: QueryResult | null;
  queryHistory: string[];
  queryLoading: boolean;
  
  // UI state
  error: string | null;
  isDirty: boolean;
}

export const dataBrowserStore = proxy<DataBrowserStore>({
  selectedDatasourceId: null,
  
  tables: [],
  selectedTable: null,
  tablesLoading: false,
  
  tableData: null,
  currentPage: 1,
  pageSize: 50,
  totalRows: 0,
  sortColumn: null,
  sortDirection: "asc",
  filters: {},
  dataLoading: false,
  
  currentQuery: "",
  queryResults: null,
  queryHistory: [],
  queryLoading: false,
  
  error: null,
  isDirty: false,
});

export const dataBrowserActions = {
  // Datasource selection
  selectDatasource: (datasourceId: string | null) => {
    dataBrowserStore.selectedDatasourceId = datasourceId;
    dataBrowserStore.selectedTable = null;
    dataBrowserStore.tableData = null;
    dataBrowserStore.queryResults = null;
    dataBrowserStore.error = null;
    
    if (datasourceId) {
      dataBrowserActions.loadTables(datasourceId);
    } else {
      dataBrowserStore.tables = [];
    }
  },

  // Table operations
  async loadTables(datasourceId: string) {
    try {
      dataBrowserStore.tablesLoading = true;
      dataBrowserStore.error = null;
      
      const tables = await datasourcesApi.getTables(datasourceId);
      dataBrowserStore.tables = tables;
    } catch (error: any) {
      console.error('Failed to load tables:', error);
      dataBrowserStore.error = error?.response?.data?.error || 'Failed to load tables';
      dataBrowserStore.tables = [];
    } finally {
      dataBrowserStore.tablesLoading = false;
    }
  },

  selectTable: (tableName: string | null) => {
    dataBrowserStore.selectedTable = tableName;
    dataBrowserStore.tableData = null;
    dataBrowserStore.currentPage = 1;
    dataBrowserStore.sortColumn = null;
    dataBrowserStore.sortDirection = "asc";
    dataBrowserStore.filters = {};
    
    if (tableName && dataBrowserStore.selectedDatasourceId) {
      dataBrowserActions.loadTableData();
    }
  },

  async loadTableData() {
    if (!dataBrowserStore.selectedDatasourceId || !dataBrowserStore.selectedTable) {
      return;
    }

    try {
      dataBrowserStore.dataLoading = true;
      dataBrowserStore.error = null;

      const request: TableDataRequest = {
        page: dataBrowserStore.currentPage,
        limit: dataBrowserStore.pageSize,
        sort_column: dataBrowserStore.sortColumn || undefined,
        sort_direction: dataBrowserStore.sortDirection,
        filters: Object.keys(dataBrowserStore.filters).length > 0 ? dataBrowserStore.filters : undefined,
      };

      const result = await datasourcesApi.getTableData(
        dataBrowserStore.selectedDatasourceId,
        dataBrowserStore.selectedTable,
        request
      );

      dataBrowserStore.tableData = result;
      dataBrowserStore.totalRows = result.row_count;
    } catch (error: any) {
      console.error('Failed to load table data:', error);
      dataBrowserStore.error = error?.response?.data?.error || 'Failed to load table data';
      dataBrowserStore.tableData = null;
    } finally {
      dataBrowserStore.dataLoading = false;
    }
  },

  // Pagination
  setPage: (page: number) => {
    dataBrowserStore.currentPage = page;
    dataBrowserActions.loadTableData();
  },

  setPageSize: (size: number) => {
    dataBrowserStore.pageSize = size;
    dataBrowserStore.currentPage = 1;
    dataBrowserActions.loadTableData();
  },

  // Sorting
  setSorting: (column: string, direction: "asc" | "desc") => {
    dataBrowserStore.sortColumn = column;
    dataBrowserStore.sortDirection = direction;
    dataBrowserStore.currentPage = 1;
    dataBrowserActions.loadTableData();
  },

  // Filters
  setFilters: (filters: Record<string, any>) => {
    dataBrowserStore.filters = { ...filters };
    dataBrowserStore.currentPage = 1;
    dataBrowserActions.loadTableData();
  },

  // Query execution
  setQuery: (query: string) => {
    dataBrowserStore.currentQuery = query;
  },

  async executeQuery() {
    if (!dataBrowserStore.selectedDatasourceId || !dataBrowserStore.currentQuery.trim()) {
      return;
    }

    try {
      dataBrowserStore.queryLoading = true;
      dataBrowserStore.error = null;

      const result = await datasourcesApi.executeQuery(dataBrowserStore.selectedDatasourceId, {
        query: dataBrowserStore.currentQuery,
        limit: 1000, // Default limit for custom queries
      });

      dataBrowserStore.queryResults = result;
      
      // Add to query history if not already present
      const trimmedQuery = dataBrowserStore.currentQuery.trim();
      if (!dataBrowserStore.queryHistory.includes(trimmedQuery)) {
        dataBrowserStore.queryHistory.unshift(trimmedQuery);
        // Keep only last 20 queries
        if (dataBrowserStore.queryHistory.length > 20) {
          dataBrowserStore.queryHistory = dataBrowserStore.queryHistory.slice(0, 20);
        }
      }
    } catch (error: any) {
      console.error('Failed to execute query:', error);
      dataBrowserStore.error = error?.response?.data?.error || 'Failed to execute query';
      dataBrowserStore.queryResults = null;
    } finally {
      dataBrowserStore.queryLoading = false;
    }
  },

  // Error handling
  clearError: () => {
    dataBrowserStore.error = null;
  },

  // Reset store
  reset: () => {
    dataBrowserStore.selectedDatasourceId = null;
    dataBrowserStore.tables = [];
    dataBrowserStore.selectedTable = null;
    dataBrowserStore.tableData = null;
    dataBrowserStore.currentPage = 1;
    dataBrowserStore.pageSize = 50;
    dataBrowserStore.sortColumn = null;
    dataBrowserStore.sortDirection = "asc";
    dataBrowserStore.filters = {};
    dataBrowserStore.currentQuery = "";
    dataBrowserStore.queryResults = null;
    dataBrowserStore.error = null;
    dataBrowserStore.isDirty = false;
  },
};