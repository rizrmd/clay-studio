import { proxy } from "valtio";
import {
  datasourcesApi,
  type QueryResult,
  type TableDataResult,
  type TableDataRequest,
  type TableStructure,
} from "@/lib/api/datasources";

export interface DataBrowserStore {
  // Current datasource
  selectedDatasourceId: string | null;

  // Tables
  tables: string[];
  selectedTable: string | null;
  tablesLoading: boolean;

  // Table data
  tableData: TableDataResult | null;
  currentPage: number;
  pageSize: number;
  totalRows: number;
  sortColumn: string | null;
  sortDirection: "asc" | "desc";
  filters: Record<string, any>;
  dataLoading: boolean;

  // Table structure
  tableStructure: TableStructure | null;
  structureLoading: boolean;

  // Query execution
  currentQuery: string;
  queryResults: QueryResult | null;
  queryHistory: string[];
  queryLoading: boolean;

  // UI state
  error: string | null;
  isDirty: boolean;

  // Cell editing
  editingChanges: Record<string, Record<string, any>>; // rowId -> columnKey -> newValue
  editingInProgress: boolean;

  // New row insertion
  pendingNewRows: any[]; // Array of new rows to be inserted
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

  tableStructure: null,
  structureLoading: false,

  currentQuery: "",
  queryResults: null,
  queryHistory: [],
  queryLoading: false,

  error: null,
  isDirty: false,

  editingChanges: {},
  editingInProgress: false,

  pendingNewRows: [],
});

export const dataBrowserActions = {
  // Datasource selection
  selectDatasource: (datasourceId: string | null) => {
    // Don't reload if it's the same datasource
    if (dataBrowserStore.selectedDatasourceId === datasourceId) {
      return;
    }
    
    dataBrowserStore.selectedDatasourceId = datasourceId;
    dataBrowserStore.selectedTable = null;
    dataBrowserStore.tableData = null;
    dataBrowserStore.tableStructure = null;
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
      console.error("Failed to load tables:", error);
      dataBrowserStore.error =
        error?.response?.data?.error || "Failed to load tables";
      dataBrowserStore.tables = [];
    } finally {
      dataBrowserStore.tablesLoading = false;
    }
  },

  selectTable: (tableName: string | null) => {
    // Don't reload if it's the same table
    if (dataBrowserStore.selectedTable === tableName) {
      return;
    }
    
    dataBrowserStore.selectedTable = tableName;
    dataBrowserStore.tableData = null;
    dataBrowserStore.tableStructure = null;
    dataBrowserStore.currentPage = 1;
    dataBrowserStore.sortColumn = null;
    dataBrowserStore.sortDirection = "asc";
    dataBrowserStore.filters = {};
    // Structure will be loaded in parallel with data by the component
  },

  async loadTableData() {
    if (
      !dataBrowserStore.selectedDatasourceId ||
      !dataBrowserStore.selectedTable
    ) {
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
        filters:
          Object.keys(dataBrowserStore.filters).length > 0
            ? dataBrowserStore.filters
            : undefined,
      };

      const result = await datasourcesApi.getTableData(
        dataBrowserStore.selectedDatasourceId,
        dataBrowserStore.selectedTable,
        request
      );

      console.log(">>", result);
      dataBrowserStore.tableData = result;
      dataBrowserStore.totalRows = result.total;
    } catch (error: any) {
      console.error("Failed to load table data:", error);
      dataBrowserStore.error =
        error?.response?.data?.error || "Failed to load table data";
      dataBrowserStore.tableData = null;
    } finally {
      dataBrowserStore.dataLoading = false;
    }
  },

  async loadTableStructure() {
    if (
      !dataBrowserStore.selectedDatasourceId ||
      !dataBrowserStore.selectedTable
    ) {
      return;
    }

    try {
      dataBrowserStore.structureLoading = true;
      dataBrowserStore.error = null;

      const structure = await datasourcesApi.getTableStructure(
        dataBrowserStore.selectedDatasourceId,
        dataBrowserStore.selectedTable
      );

      dataBrowserStore.tableStructure = structure;
    } catch (error: any) {
      console.error("Failed to load table structure:", error);
      dataBrowserStore.error =
        error?.response?.data?.error || "Failed to load table structure";
      dataBrowserStore.tableStructure = null;
    } finally {
      dataBrowserStore.structureLoading = false;
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
    if (
      !dataBrowserStore.selectedDatasourceId ||
      !dataBrowserStore.currentQuery.trim()
    ) {
      return;
    }

    try {
      dataBrowserStore.queryLoading = true;
      dataBrowserStore.error = null;

      const result = await datasourcesApi.executeQuery(
        dataBrowserStore.selectedDatasourceId,
        {
          query: dataBrowserStore.currentQuery,
          limit: 1000, // Default limit for custom queries
        }
      );

      dataBrowserStore.queryResults = result;

      // Add to query history if not already present
      const trimmedQuery = dataBrowserStore.currentQuery.trim();
      if (!dataBrowserStore.queryHistory.includes(trimmedQuery)) {
        dataBrowserStore.queryHistory.unshift(trimmedQuery);
        // Keep only last 20 queries
        if (dataBrowserStore.queryHistory.length > 20) {
          dataBrowserStore.queryHistory = dataBrowserStore.queryHistory.slice(
            0,
            20
          );
        }
      }
    } catch (error: any) {
      console.error("Failed to execute query:", error);
      dataBrowserStore.error =
        error?.response?.data?.error || "Failed to execute query";
      dataBrowserStore.queryResults = null;
    } finally {
      dataBrowserStore.queryLoading = false;
    }
  },

  // Data manipulation
  setTableData: (data: any[]) => {
    if (dataBrowserStore.tableData) {
      // Convert the flat object array back to the TableDataResult format
      const columns = Object.keys(data[0] || {});
      const rows = data.map((row) => columns.map((col) => row[col]));

      dataBrowserStore.tableData = {
        ...dataBrowserStore.tableData,
        rows: rows,
      };
      dataBrowserStore.isDirty = true;
    }
  },

  // Error handling
  clearError: () => {
    dataBrowserStore.error = null;
  },

  // Cell editing operations

  setCellValue: (rowId: string, columnKey: string, newValue: any) => {
    if (!dataBrowserStore.editingChanges[rowId]) {
      dataBrowserStore.editingChanges[rowId] = {};
    }
    dataBrowserStore.editingChanges[rowId][columnKey] = newValue;
    dataBrowserStore.isDirty = true;
  },

  async saveChanges() {
    if (
      !dataBrowserStore.selectedDatasourceId ||
      !dataBrowserStore.selectedTable
    ) {
      return;
    }

    try {
      dataBrowserStore.editingInProgress = true;
      dataBrowserStore.error = null;

      // Save cell edits if any
      if (Object.keys(dataBrowserStore.editingChanges).length > 0) {
        const primaryKeyColumn =
          dataBrowserStore.tableStructure?.primary_keys[0] || "id";

        await datasourcesApi.updateRows(
          dataBrowserStore.selectedDatasourceId,
          dataBrowserStore.selectedTable,
          {
            updates: dataBrowserStore.editingChanges,
            id_column: primaryKeyColumn,
          }
        );
      }

      // Save new rows if any
      if (dataBrowserStore.pendingNewRows.length > 0) {
        const rowsToInsert = dataBrowserStore.pendingNewRows.map((row) => {
          // Remove temporary fields before inserting
          const { __tempId, __isNewRow, ...cleanRow } = row;
          return cleanRow;
        });

        await datasourcesApi.insertRows(
          dataBrowserStore.selectedDatasourceId,
          dataBrowserStore.selectedTable,
          {
            rows: rowsToInsert,
          }
        );
      }

      // Clear changes after successful save
      dataBrowserStore.editingChanges = {};
      dataBrowserStore.pendingNewRows = [];
      dataBrowserStore.isDirty = false;

      // Reload data to reflect changes
      await dataBrowserActions.loadTableData();
    } catch (error: any) {
      console.error("Failed to save changes:", error);
      dataBrowserStore.error =
        error?.response?.data?.error || "Failed to save changes";
    } finally {
      dataBrowserStore.editingInProgress = false;
    }
  },

  discardChanges: () => {
    dataBrowserStore.editingChanges = {};
    dataBrowserStore.pendingNewRows = [];
    dataBrowserStore.isDirty = false;
  },

  addNewRow: (rowData: any) => {
    // Generate a temporary ID for the new row
    const tempId = `new_${Date.now()}_${Math.random()
      .toString(36)
      .substr(2, 9)}`;
    const newRow = { ...rowData, __tempId: tempId, __isNewRow: true };

    dataBrowserStore.pendingNewRows.push(newRow);
    dataBrowserStore.isDirty = true;
  },

  // Refresh table data (alias for loadTableData)
  refreshTableData: () => {
    return dataBrowserActions.loadTableData();
  },

  // Reset store
  reset: () => {
    dataBrowserStore.selectedDatasourceId = null;
    dataBrowserStore.tables = [];
    dataBrowserStore.selectedTable = null;
    dataBrowserStore.tableData = null;
    dataBrowserStore.tableStructure = null;
    dataBrowserStore.currentPage = 1;
    dataBrowserStore.pageSize = 50;
    dataBrowserStore.sortColumn = null;
    dataBrowserStore.sortDirection = "asc";
    dataBrowserStore.filters = {};
    dataBrowserStore.currentQuery = "";
    dataBrowserStore.queryResults = null;
    dataBrowserStore.error = null;
    dataBrowserStore.isDirty = false;
    dataBrowserStore.editingChanges = {};
    dataBrowserStore.editingInProgress = false;
    dataBrowserStore.pendingNewRows = [];
  },
};
