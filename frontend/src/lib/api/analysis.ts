import { api } from '@/lib/utils/api';
import {
  Analysis,
  AnalysisConfig,
  AnalysisJob,
  AnalysisResult,
  AnalysisSchedule
} from '../store/analysis-store';

// MCP Analysis Tools API
export interface McpAnalysisJob {
  job_id: string;
  analysis_id: string;
  analysis_title: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
  parameters?: any;
  result?: any;
  error_message?: string;
  created_at: string;
  started_at?: string;
  completed_at?: string;
}

export interface McpAnalysis {
  analysis_id: string;
  title: string;
  description?: string;
  script_content?: string;
  is_active: boolean;
  version: number;
  created_at: string;
  updated_at: string;
  metadata?: any;
}

export interface McpAnalysisCreateRequest {
  title: string;
  script_content: string;
  description?: string;
  parameters?: any;
}

export interface McpAnalysisRunRequest {
  analysis_id: string;
  parameters?: any;
  datasources?: any;
}

export interface McpAnalysisListResponse {
  analyses: McpAnalysis[];
  count: number;
  project_id: string;
}

export interface McpJobListResponse {
  jobs: McpAnalysisJob[];
  count: number;
  project_id: string;
}

export interface CreateAnalysisRequest {
  name: string;
  description?: string;
  type: 'sql' | 'python' | 'r';
  config: AnalysisConfig;
}

export interface UpdateAnalysisRequest {
  name?: string;
  description?: string;
  config?: AnalysisConfig;
}

export interface CreateJobRequest {
  analysis_id: string;
  parameters?: Record<string, any>;
}

export interface CreateScheduleRequest {
  analysis_id: string;
  name: string;
  cron_expression: string;
  is_active?: boolean;
}

export interface ExecuteAnalysisResponse {
  job_id: string;
  status: string;
  message: string;
}

// MCP Analysis Tools API
export const mcpAnalysisApi = {
  /**
   * Create a new analysis via MCP
   */
  createAnalysis: async (data: McpAnalysisCreateRequest): Promise<{ analysis_id: string; status: string; message: string }> => {
    return await api.post('/api/analysis/create', data);
  },

  /**
   * List all analyses via MCP
   */
  listAnalyses: async (options?: {
    active_only?: boolean;
    limit?: number;
  }): Promise<McpAnalysisListResponse> => {
    const params = new URLSearchParams();
    if (options?.active_only !== undefined) {
      params.append('active_only', String(options.active_only));
    }
    if (options?.limit) {
      params.append('limit', String(options.limit));
    }

    const url = `/api/analysis/list${params.toString() ? `?${params.toString()}` : ''}`;
    return await api.get<McpAnalysisListResponse>(url);
  },

  /**
   * Get detailed information about a specific analysis via MCP
   */
  getAnalysis: async (analysisId: string): Promise<{ status: string; analysis: McpAnalysis }> => {
    return await api.get<{ status: string; analysis: McpAnalysis }>(`/api/analysis/get?analysis_id=${analysisId}`);
  },

  /**
   * Update an existing analysis via MCP
   */
  updateAnalysis: async (analysisId: string, data: {
    title?: string;
    script_content?: string;
    description?: string;
  }): Promise<{ status: string; analysis: McpAnalysis }> => {
    return await api.post('/api/analysis/update', {
      analysis_id: analysisId,
      ...data
    });
  },

  /**
   * Delete (deactivate) an analysis via MCP
   */
  deleteAnalysis: async (analysisId: string): Promise<{ status: string; message: string; analysis: { id: string; title: string } }> => {
    return await api.post('/api/analysis/delete', {
      analysis_id: analysisId
    });
  },

  /**
   * Run an analysis via MCP
   */
  runAnalysis: async (data: McpAnalysisRunRequest): Promise<{ success: boolean; message: string; analysis_id: string; status: string }> => {
    return await api.post('/api/analysis/run', data);
  },

  /**
   * Validate an analysis script via MCP
   */
  validateAnalysis: async (analysisId: string, scriptContent?: string): Promise<{
    status: string;
    valid: boolean;
    analysis_id: string;
    validation: {
      errors: string[];
      warnings: string[];
      script_length: number;
      line_count: number;
    };
    message: string;
  }> => {
    const params = new URLSearchParams({ analysis_id: analysisId });
    if (scriptContent) {
      params.append('script_content', scriptContent);
    }

    return await api.post(`/api/analysis/validate?${params.toString()}`);
  },

  /**
   * List analysis execution jobs via MCP
   */
  listJobs: async (options?: {
    analysis_id?: string;
    status?: string;
    limit?: number;
  }): Promise<McpJobListResponse> => {
    const params = new URLSearchParams();
    if (options?.analysis_id) {
      params.append('analysis_id', options.analysis_id);
    }
    if (options?.status) {
      params.append('status', options.status);
    }
    if (options?.limit) {
      params.append('limit', String(options.limit));
    }

    const url = `/api/analysis/job_list${params.toString() ? `?${params.toString()}` : ''}`;
    return await api.get<McpJobListResponse>(url);
  },

  /**
   * Get detailed information about a specific job via MCP
   */
  getJob: async (jobId: string): Promise<McpAnalysisJob> => {
    return await api.get<McpAnalysisJob>(`/api/analysis/job_get?job_id=${jobId}`);
  },

  /**
   * Cancel a running analysis job via MCP
   */
  cancelJob: async (jobId: string): Promise<{ success: boolean; job_id: string; status: string; message: string }> => {
    return await api.post('/api/analysis/job_cancel', {
      job_id: jobId
    });
  },

  /**
   * Get the result of a completed analysis job via MCP
   */
  getJobResult: async (jobId: string): Promise<{
    job_id: string;
    status: string;
    result?: any;
    error?: string;
    message?: string;
  }> => {
    return await api.get(`/api/analysis/job_result?job_id=${jobId}`);
  }
};

// Analysis CRUD operations
export const analysisApi = {
  // Get all analyses for a project
  getAnalyses: async (projectId: string): Promise<Analysis[]> => {
    return await api.get<Analysis[]>(`/projects/${projectId}/analysis`);
  },

  // Get a single analysis
  getAnalysis: async (analysisId: string): Promise<Analysis> => {
    return await api.get<Analysis>(`/analysis/${analysisId}`);
  },

  // Create a new analysis
  createAnalysis: async (projectId: string, data: CreateAnalysisRequest): Promise<Analysis> => {
    return await api.post<Analysis>(`/projects/${projectId}/analysis`, data);
  },

  // Update an analysis
  updateAnalysis: async (analysisId: string, data: UpdateAnalysisRequest): Promise<Analysis> => {
    return await api.put<Analysis>(`/analysis/${analysisId}`, data);
  },

  // Delete an analysis
  deleteAnalysis: async (analysisId: string): Promise<void> => {
    await api.delete(`/analysis/${analysisId}`);
  },

  // Clone an analysis
  cloneAnalysis: async (analysisId: string, name?: string): Promise<Analysis> => {
    return await api.post<Analysis>(`/analysis/${analysisId}/clone`, { name });
  },

  // Job operations
  getJobs: async (analysisId: string, limit?: number): Promise<AnalysisJob[]> => {
    return await api.get<AnalysisJob[]>(
      `/analysis/${analysisId}/jobs${limit ? `?limit=${limit}` : ''}`
    );
  },

  getJob: async (jobId: string): Promise<AnalysisJob> => {
    return await api.get<AnalysisJob>(`/jobs/${jobId}`);
  },

  executeAnalysis: async (analysisId: string, data?: CreateJobRequest): Promise<ExecuteAnalysisResponse> => {
    return await api.post<ExecuteAnalysisResponse>(
      `/analysis/${analysisId}/execute`,
      data || {}
    );
  },

  cancelJob: async (jobId: string): Promise<void> => {
    await api.post(`/jobs/${jobId}/cancel`);
  },

  getJobResult: async (jobId: string): Promise<AnalysisResult> => {
    return await api.get<AnalysisResult>(`/jobs/${jobId}/result`);
  },

  // Schedule operations
  getSchedules: async (analysisId?: string): Promise<AnalysisSchedule[]> => {
    const url = analysisId 
      ? `/analysis/${analysisId}/schedules`
      : '/analysis/schedules';
    return await api.get<AnalysisSchedule[]>(url);
  },

  createSchedule: async (data: CreateScheduleRequest): Promise<AnalysisSchedule> => {
    return await api.post<AnalysisSchedule>('/analysis/schedules', data);
  },

  updateSchedule: async (scheduleId: string, data: Partial<CreateScheduleRequest>): Promise<AnalysisSchedule> => {
    return await api.put<AnalysisSchedule>(`/analysis/schedules/${scheduleId}`, data);
  },

  deleteSchedule: async (scheduleId: string): Promise<void> => {
    await api.delete(`/analysis/schedules/${scheduleId}`);
  },

  toggleSchedule: async (scheduleId: string, isActive: boolean): Promise<AnalysisSchedule> => {
    return await api.put<AnalysisSchedule>(
      `/analysis/schedules/${scheduleId}/toggle`,
      { is_active: isActive }
    );
  },

  // Analysis execution history
  getExecutionHistory: async (analysisId: string, limit?: number): Promise<AnalysisJob[]> => {
    return await api.get<AnalysisJob[]>(
      `/analysis/${analysisId}/history${limit ? `?limit=${limit}` : ''}`
    );
  },

  // Analysis templates
  getTemplates: async (): Promise<Analysis[]> => {
    return await api.get<Analysis[]>('/analysis/templates');
  },

  createFromTemplate: async (templateId: string, projectId: string, name: string): Promise<Analysis> => {
    return await api.post<Analysis>(`/analysis/templates/${templateId}/use`, {
      project_id: projectId,
      name,
    });
  },
};