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
};