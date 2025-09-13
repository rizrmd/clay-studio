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

export interface QueryResult {
  readonly columns: readonly string[];
  readonly rows: readonly (readonly string[])[];
  readonly row_count: number;
  readonly execution_time_ms: number;
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
  getTableData: async (datasourceId: string, tableName: string, data: TableDataRequest): Promise<QueryResult> => {
    return api.post(`/datasources/${datasourceId}/tables/${tableName}/data`, data);
  },

  // Get list of tables
  getTables: async (datasourceId: string): Promise<string[]> => {
    return api.get(`/datasources/${datasourceId}/tables`);
  },
};