import { proxy } from 'valtio';
import { subscribeKey } from 'valtio/utils';

// Types for Analysis
export interface AnalysisConfig {
  query?: string;
  code?: string;
}

export interface Analysis {
  id: string;
  name: string;
  description?: string;
  type: 'sql' | 'python' | 'r';
  function_name: string; // Name of the pre-defined JS function
  parameters: AnalysisParameter[];
  status: 'idle' | 'running' | 'completed' | 'failed';
  created_at: string;
  updated_at: string;
  project_id: string;
  last_job?: AnalysisJob;
  config: {
    query?: string;
    code?: string;
  };
}

export interface AnalysisParameter {
  name: string;
  type: 'string' | 'number' | 'boolean' | 'array' | 'object' | 'date';
  required: boolean;
  default?: any;
  description?: string;
  options?: any[]; // For select/enum parameters
}

export interface AnalysisJob {
  id: string;
  analysis_id: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
  started_at?: string;
  completed_at?: string;
  execution_time_ms?: number;
  result?: AnalysisResult;
  error?: string;
  progress?: number;
  progress_message?: string;
}

export interface AnalysisResult {
  data?: any;
  rows_affected?: number;
  output_files?: string[];
  statistics?: Record<string, any>;
  visualization_config?: any;
}

export interface AnalysisSchedule {
  id: string;
  analysis_id: string;
  name: string;
  cron_expression: string;
  is_active: boolean;
  next_run: string;
  created_at: string;
  updated_at: string;
}

interface AnalysisState {
  // Analysis data
  analyses: Analysis[];
  activeAnalysisId: string | null;
  isLoading: boolean;
  error: string | null;
  
  // Jobs and execution
  jobs: Record<string, AnalysisJob>; // analysis_id -> latest job
  activeJobs: string[]; // analysis_ids with running jobs
  
  // Schedules
  schedules: AnalysisSchedule[];
  
  // UI state
  selectedAnalysisId: string | null;
  executingAnalysisId: string | null;
  isCreating: boolean;
  isEditing: boolean;
  selectedAnalysisType: 'sql' | 'python' | 'r' | null;
  editorContent: string;
}

const initialState: AnalysisState = {
  analyses: [],
  activeAnalysisId: null,
  isLoading: false,
  error: null,
  jobs: {},
  activeJobs: [],
  schedules: [],
  selectedAnalysisId: null,
  executingAnalysisId: null,
  isCreating: false,
  isEditing: false,
  selectedAnalysisType: null,
  editorContent: '',
};

export const analysisStore = proxy<AnalysisState>(initialState);

// Persist active analysis ID to localStorage
subscribeKey(analysisStore, 'activeAnalysisId', (activeAnalysisId) => {
  if (typeof window !== 'undefined') {
    localStorage.setItem('clay-studio-active-analysis', activeAnalysisId || '');
  }
});

// Load persisted active analysis ID on startup
if (typeof window !== 'undefined') {
  const saved = localStorage.getItem('clay-studio-active-analysis');
  if (saved) {
    analysisStore.activeAnalysisId = saved || null;
  }
}

export const analysisActions = {
  // Loading states
  setLoading: (loading: boolean) => {
    analysisStore.isLoading = loading;
  },
  
  setError: (error: string | null) => {
    analysisStore.error = error;
  },
  
  // Analysis CRUD
  setAnalyses: (analyses: Analysis[]) => {
    analysisStore.analyses = analyses;
  },
  
  addAnalysis: (analysis: Analysis) => {
    analysisStore.analyses.push(analysis);
  },
  
  updateAnalysis: (id: string, updates: Partial<Analysis>) => {
    const analysis = analysisStore.analyses.find(a => a.id === id);
    if (analysis) {
      Object.assign(analysis, updates);
    }
  },
  
  removeAnalysis: (id: string) => {
    const index = analysisStore.analyses.findIndex(a => a.id === id);
    if (index > -1) {
      analysisStore.analyses.splice(index, 1);
    }
    
    // Clear active if removed
    if (analysisStore.activeAnalysisId === id) {
      analysisStore.activeAnalysisId = null;
    }
  },
  
  setActiveAnalysis: (id: string | null) => {
    analysisStore.activeAnalysisId = id;
  },
  
  // Job management
  updateJob: (analysisId: string, job: AnalysisJob) => {
    analysisStore.jobs[analysisId] = job;
    
    // Track active jobs
    if (job.status === 'running') {
      if (!analysisStore.activeJobs.includes(analysisId)) {
        analysisStore.activeJobs.push(analysisId);
      }
    } else {
      analysisStore.activeJobs = analysisStore.activeJobs.filter(id => id !== analysisId);
    }
    
    // Update analysis status based on job
    const analysis = analysisStore.analyses.find(a => a.id === analysisId);
    if (analysis) {
      analysis.last_job = job;
      if (job.status === 'completed') {
        analysis.status = 'completed';
      } else if (job.status === 'failed') {
        analysis.status = 'failed';
      } else if (job.status === 'running') {
        analysis.status = 'running';
      }
    }
  },
  
  removeJob: (analysisId: string) => {
    delete analysisStore.jobs[analysisId];
    analysisStore.activeJobs = analysisStore.activeJobs.filter(id => id !== analysisId);
  },
  
  // Schedule management
  setSchedules: (schedules: AnalysisSchedule[]) => {
    analysisStore.schedules = schedules;
  },
  
  addSchedule: (schedule: AnalysisSchedule) => {
    analysisStore.schedules.push(schedule);
  },
  
  updateSchedule: (id: string, updates: Partial<AnalysisSchedule>) => {
    const schedule = analysisStore.schedules.find(s => s.id === id);
    if (schedule) {
      Object.assign(schedule, updates);
    }
  },
  
  removeSchedule: (id: string) => {
    const index = analysisStore.schedules.findIndex(s => s.id === id);
    if (index > -1) {
      analysisStore.schedules.splice(index, 1);
    }
  },
  
  // UI actions
  startCreating: (type: 'sql' | 'python' | 'r') => {
    analysisStore.isCreating = true;
    analysisStore.isEditing = false;
    analysisStore.selectedAnalysisType = type;
    analysisStore.editorContent = type === 'sql' ? '-- Enter your SQL query here' : 
                                type === 'python' ? '# Enter your Python code here' :
                                '# Enter your R code here';
  },
  
  startEditing: (analysis: Analysis) => {
    analysisStore.isCreating = false;
    analysisStore.isEditing = true;
    analysisStore.selectedAnalysisType = analysis.type;
    analysisStore.editorContent = analysis.config.query || analysis.config.code || '';
    analysisStore.activeAnalysisId = analysis.id;
  },
  
  stopEditing: () => {
    analysisStore.isCreating = false;
    analysisStore.isEditing = false;
    analysisStore.selectedAnalysisType = null;
    analysisStore.editorContent = '';
  },
  
  updateEditorContent: (content: string) => {
    analysisStore.editorContent = content;
  },
  
  // Clear store
  clear: () => {
    Object.assign(analysisStore, initialState);
  },
  
  // Get active analysis
  getActiveAnalysis: () => {
    if (!analysisStore.activeAnalysisId) return null;
    return analysisStore.analyses.find(a => a.id === analysisStore.activeAnalysisId) || null;
  },
  
  // Get job for analysis
  getJobForAnalysis: (analysisId: string) => {
    return analysisStore.jobs[analysisId] || null;
  },
  
  // Get schedules for analysis
  getSchedulesForAnalysis: (analysisId: string) => {
    return analysisStore.schedules.filter(s => s.analysis_id === analysisId);
  },
};