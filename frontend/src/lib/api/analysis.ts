import { api } from '@/lib/utils/api';
import { 
  Analysis, 
  AnalysisConfig, 
  AnalysisJob, 
  AnalysisResult, 
  AnalysisSchedule 
} from '../store/analysis-store';

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
    return await api.get<AnalysisJob>(`/analysis/jobs/${jobId}`);
  },

  executeAnalysis: async (analysisId: string, data?: CreateJobRequest): Promise<ExecuteAnalysisResponse> => {
    return await api.post<ExecuteAnalysisResponse>(
      `/analysis/${analysisId}/execute`,
      data || {}
    );
  },

  cancelJob: async (jobId: string): Promise<void> => {
    await api.post(`/analysis/jobs/${jobId}/cancel`);
  },

  getJobResult: async (jobId: string): Promise<AnalysisResult> => {
    return await api.get<AnalysisResult>(`/analysis/jobs/${jobId}/result`);
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