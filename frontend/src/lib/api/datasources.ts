import { api } from "@/lib/utils/api";
import type { Datasource } from "@/lib/store/datasources-store";

export interface CreateDatasourceRequest {
  name: string;
  source_type: Datasource["source_type"];
  config: string | object;
}

export interface UpdateDatasourceRequest {
  name?: string;
  config?: string | object;
}

export interface TestConnectionResponse {
  success: boolean;
  message: string;
  error?: string;
}

export interface QueryRequest {
  query: string;
  limit?: number;
}

export interface TableDataRequest {
  page?: number;
  limit?: number;
  sort_column?: string;
  sort_direction?: string; // "asc" | "desc"
  filters?: any;
}

export interface DistinctValuesRequest {
  column: string;
  limit?: number;
  search?: string;
}

export interface DistinctValuesResult {
  readonly values: readonly string[];
  readonly count: number;
  readonly execution_time_ms: number;
}

export interface RowIdsRequest {
  id_column?: string;
  limit?: number;
}

export interface RowIdsResult {
  readonly row_ids: readonly string[];
  readonly count: number;
  readonly id_column: string;
  readonly execution_time_ms: number;
}

export interface DeleteRowsRequest {
  row_ids: string[];
  id_column?: string;
}

export interface DeleteRowsResult {
  readonly success: boolean;
  readonly rows_affected: number;
  readonly execution_time_ms: number;
  readonly deleted_ids: readonly string[];
}

export interface UpdateRowsRequest {
  updates: Record<string, Record<string, any>>; // rowId -> columnKey -> newValue
  id_column?: string;
}

export interface UpdateRowsResult {
  readonly success: boolean;
  readonly rows_affected: number;
  readonly execution_time_ms: number;
  readonly updated_ids: readonly string[];
}

export interface InsertRowsRequest {
  rows: Record<string, any>[]; // Array of row objects
}

export interface InsertRowsResult {
  readonly success: boolean;
  readonly rows_affected: number;
  readonly execution_time_ms: number;
  readonly inserted_ids: readonly string[];
}

export interface QueryResult {
  readonly columns: readonly string[];
  readonly rows: readonly (readonly string[])[];
  readonly row_count: number;
  readonly execution_time_ms: number;
  readonly query?: string; // For execute_query responses
}

export interface TableDataResult {
  readonly columns: readonly string[];
  readonly rows: readonly (readonly string[])[];
  readonly row_count: number;
  readonly total: number;
  readonly execution_time_ms: number;
  readonly page: number;
  readonly page_size: number;
}

export interface TableColumn {
  readonly name: string;
  readonly data_type: string;
  readonly is_nullable: boolean;
  readonly column_default?: string;
  readonly is_primary_key: boolean;
  readonly is_foreign_key: boolean;
  readonly foreign_key_reference?: string;
  readonly character_maximum_length?: number;
  readonly numeric_precision?: number;
  readonly numeric_scale?: number;
}

export interface TableStructure {
  readonly table_name: string;
  readonly columns: readonly TableColumn[];
  readonly primary_keys: readonly string[];
  readonly foreign_keys: readonly {
    readonly column_name: string;
    readonly referenced_table: string;
    readonly referenced_column: string;
  }[];
  readonly indexes: readonly {
    readonly name: string;
    readonly columns: readonly string[];
    readonly is_unique: boolean;
  }[];
}

export const datasourcesApi = {
  // List all datasources for a project
  list: async (projectId: string): Promise<Datasource[]> => {
    return api.get(`/projects/${projectId}/datasources`);
  },

  // Create a new datasource
  create: async (projectId: string, data: CreateDatasourceRequest): Promise<Datasource> => {
    return api.post(`/projects/${projectId}/datasources`, data);
  },

  // Update an existing datasource
  update: async (datasourceId: string, data: UpdateDatasourceRequest): Promise<Datasource> => {
    return api.put(`/datasources/${datasourceId}`, data);
  },

  // Delete a datasource
  delete: async (datasourceId: string): Promise<void> => {
    return api.delete(`/datasources/${datasourceId}`);
  },

  // Test connection to a datasource
  testConnection: async (datasourceId: string): Promise<TestConnectionResponse> => {
    return api.post(`/datasources/${datasourceId}/test`);
  },

  // Get schema information for a datasource
  getSchema: async (datasourceId: string): Promise<any> => {
    return api.get(`/datasources/${datasourceId}/schema`);
  },

  // Data browser APIs
  // Execute a custom query
  executeQuery: async (datasourceId: string, data: QueryRequest): Promise<QueryResult> => {
    return api.post(`/datasources/${datasourceId}/query`, data);
  },

  // Get table data with pagination and sorting
  getTableData: async (datasourceId: string, tableName: string, data: TableDataRequest): Promise<TableDataResult> => {
    return api.post(`/datasources/${datasourceId}/tables/${tableName}/data`, data);
  },

  // Get list of tables
  getTables: async (datasourceId: string, forceRefresh?: boolean): Promise<string[]> => {
    const params = forceRefresh ? { force_refresh: 'true' } : {};
    return api.get(`/datasources/${datasourceId}/tables`, { params });
  },

  // Get table structure information
  getTableStructure: async (datasourceId: string, tableName: string): Promise<TableStructure> => {
    return api.get(`/datasources/${datasourceId}/tables/${tableName}/structure`);
  },

  // Get distinct values for a column
  getDistinctValues: async (datasourceId: string, tableName: string, data: DistinctValuesRequest): Promise<DistinctValuesResult> => {
    return api.post(`/datasources/${datasourceId}/tables/${tableName}/distinct`, data);
  },

  // Get all row IDs for a table (for bulk selection)
  getTableRowIds: async (datasourceId: string, tableName: string, data: RowIdsRequest): Promise<RowIdsResult> => {
    return api.post(`/datasources/${datasourceId}/tables/${tableName}/row-ids`, data);
  },

  // Delete rows from a table
  deleteRows: async (datasourceId: string, tableName: string, data: DeleteRowsRequest): Promise<DeleteRowsResult> => {
    return api.delete(`/datasources/${datasourceId}/tables/${tableName}/rows`, data);
  },

  // Update rows in a table
  updateRows: async (datasourceId: string, tableName: string, data: UpdateRowsRequest): Promise<UpdateRowsResult> => {
    return api.put(`/datasources/${datasourceId}/tables/${tableName}/rows`, data);
  },

  // Insert rows into a table
  insertRows: async (datasourceId: string, tableName: string, data: InsertRowsRequest): Promise<InsertRowsResult> => {
    return api.post(`/datasources/${datasourceId}/tables/${tableName}/rows`, data);
  },
};