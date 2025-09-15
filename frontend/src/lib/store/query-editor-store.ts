import { proxy } from "valtio";
import { datasourcesApi, type QueryResult } from "@/lib/api/datasources";

interface QueryEditorState {
  query: string;
  datasourceId: string | null;
  
  // Query execution
  queryResults: QueryResult | null;
  queryHistory: string[];
  queryLoading: boolean;
  
  // Error handling (separate from dataBrowserStore)
  error: string | null;
}

const initialQueryEditorState: QueryEditorState = {
  query: "",
  datasourceId: null,
  
  queryResults: null,
  queryHistory: [],
  queryLoading: false,
  
  error: null,
};

export const queryEditorStore = proxy(initialQueryEditorState);

export const queryEditorActions = {
  setQuery: (query: string) => {
    queryEditorStore.query = query;
  },

  setDatasourceId: (datasourceId: string | null) => {
    queryEditorStore.datasourceId = datasourceId;
  },

  async executeQuery() {
    if (!queryEditorStore.datasourceId || !queryEditorStore.query.trim()) {
      return;
    }

    try {
      queryEditorStore.queryLoading = true;
      queryEditorStore.error = null;

      // Execute query WITHOUT limit to disable pagination
      const result = await datasourcesApi.executeQuery(queryEditorStore.datasourceId, {
        query: queryEditorStore.query,
        // No limit specified - this allows unlimited results
      });

      queryEditorStore.queryResults = result;
      
      // Add to query history if not already present
      const trimmedQuery = queryEditorStore.query.trim();
      if (!queryEditorStore.queryHistory.includes(trimmedQuery)) {
        queryEditorStore.queryHistory.unshift(trimmedQuery);
        // Keep only last 20 queries
        if (queryEditorStore.queryHistory.length > 20) {
          queryEditorStore.queryHistory = queryEditorStore.queryHistory.slice(0, 20);
        }
      }
    } catch (error: any) {
      console.error('Failed to execute query:', error);
      queryEditorStore.error = error?.response?.data?.error || 'Failed to execute query';
      queryEditorStore.queryResults = null;
    } finally {
      queryEditorStore.queryLoading = false;
    }
  },

  clearError: () => {
    queryEditorStore.error = null;
  },

  reset: () => {
    queryEditorStore.query = "";
    queryEditorStore.datasourceId = null;
    queryEditorStore.queryResults = null;
    queryEditorStore.queryHistory = [];
    queryEditorStore.error = null;
  },
};