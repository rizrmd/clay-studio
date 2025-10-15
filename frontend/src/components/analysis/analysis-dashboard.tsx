import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Play,
  Calendar,
  BarChart3,
  Settings,
  Code,
  Clock,
  CheckCircle,
  XCircle,
  Loader2,
  Download,
  MessageSquare,
  Edit,
  Trash2,
  Save
} from 'lucide-react';
import { analysisApi } from '@/lib/api/analysis';
import { Analysis, AnalysisParameter, analysisActions } from '@/lib/store/analysis-store';
import { DynamicFilters, type FilterConfig } from './dynamic-filters';
import { AnalysisSchedulesDialog } from './analysis-schedules-dialog';
import { createChatForAnalysisError } from '@/lib/utils/chat-helpers';
import { tabsStore, tabsActions } from '@/lib/store/tabs-store';

interface AnalysisDashboardProps {
  analysisId?: string;
  projectId: string;
  mode?: string;
}

interface JobExecution {
  id: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
  created_at: string;
  completed_at?: string;
  result?: any;
  error?: string;
  parameters?: Record<string, any>;
}

export function AnalysisDashboard({ analysisId, projectId, mode }: AnalysisDashboardProps) {
  const navigate = useNavigate();
  const [activeTab, setActiveTab] = useState('overview');
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showSchedules, setShowSchedules] = useState(false);
  const [currentJob, setCurrentJob] = useState<JobExecution | null>(null);
  const [executionHistory, setExecutionHistory] = useState<JobExecution[]>([]);
  const [filterValues, setFilterValues] = useState<Record<string, any>>({});
  const [showFilters, setShowFilters] = useState(false);

  // Load analysis data
  useEffect(() => {
    if (analysisId && mode !== 'create') {
      loadAnalysis();
    }
  }, [analysisId, mode]);

  const loadAnalysis = async () => {
    if (!analysisId || isLoading) return;

    setIsLoading(true);
    setError(null);

    try {
      const data = await analysisApi.getAnalysis(analysisId);
      setAnalysis(data);

      // Initialize filter values from analysis parameters
      const defaults: Record<string, any> = {};
      data.parameters?.forEach(param => {
        if (param.default !== undefined) {
          defaults[param.name] = param.default;
        }
      });
      setFilterValues(defaults);

      // Load execution history
      await loadExecutionHistory();

      // Check if there's a running job
      if (data.last_job && (data.last_job.status === 'pending' || data.last_job.status === 'running')) {
        setCurrentJob(data.last_job);
        setIsExecuting(true);
        pollJobStatus(data.last_job.id);
      }
    } catch (err: any) {
      setError(err.message || 'Failed to load analysis');
    } finally {
      setIsLoading(false);
    }
  };

  const loadExecutionHistory = async () => {
    if (!analysisId) return;

    try {
      const jobs = await analysisApi.listJobs({ analysis_id: analysisId, limit: 10 });
      setExecutionHistory(jobs.jobs || []);
    } catch (err) {
      console.error('Failed to load execution history:', err);
    }
  };

  const handleExecute = async () => {
    if (!analysisId) return;

    setIsExecuting(true);
    setError(null);
    setCurrentJob(null);

    // Update analysis status in store to 'running'
    analysisActions.updateAnalysis(analysisId, { status: 'running' });

    try {
      const response = await analysisApi.executeAnalysis(analysisId, {
        analysis_id: analysisId,
        parameters: filterValues,
      });

      const newJob: JobExecution = {
        id: response.job_id,
        status: 'pending',
        created_at: new Date().toISOString(),
        parameters: { ...filterValues }
      };

      setCurrentJob(newJob);
      setExecutionHistory(prev => [newJob, ...prev]);
      setActiveTab('results');

      // Start polling for job status
      pollJobStatus(response.job_id);
    } catch (err: any) {
      setError(err.message || 'Failed to execute analysis');
      setIsExecuting(false);
      analysisActions.updateAnalysis(analysisId, { status: 'idle' });
    }
  };

  const pollJobStatus = async (jobId: string) => {
    if (!analysisId) return;

    try {
      const job = await analysisApi.getJob(jobId);

      setCurrentJob(prev => prev ? { ...prev, ...job } : null);

      // Update execution history
      setExecutionHistory(prev =>
        prev.map(exec => exec.id === jobId ? { ...exec, ...job } : exec)
      );

      if (job.status === 'completed') {
        setIsExecuting(false);
        analysisActions.updateAnalysis(analysisId, { status: 'completed' });
      } else if (job.status === 'failed' || job.status === 'cancelled') {
        setIsExecuting(false);
        analysisActions.updateAnalysis(analysisId, { status: 'failed' });
      } else if (job.status === 'running' || job.status === 'pending') {
        // Continue polling
        setTimeout(() => pollJobStatus(jobId), 2000);
      }
    } catch (err: any) {
      setError(err.message || 'Failed to get job status');
      setIsExecuting(false);
      analysisActions.updateAnalysis(analysisId, { status: 'idle' });
    }
  };

  const handleFilterChange = (name: string, value: any) => {
    setFilterValues(prev => ({
      ...prev,
      [name]: value
    }));
  };

  const handleAddFilter = (filterName: string) => {
    setFilterValues(prev => ({
      ...prev,
      [filterName]: ""
    }));
  };

  const handleRemoveFilter = (filterName: string) => {
    setFilterValues(prev => {
      const newValues = { ...prev };
      delete newValues[filterName];
      return newValues;
    });
  };

  const convertParametersToFilters = (parameters: AnalysisParameter[]): FilterConfig[] => {
    return parameters.map(param => ({
      name: param.name,
      label: param.name.charAt(0).toUpperCase() + param.name.slice(1).replace(/_/g, ' '),
      type: param.type === 'number' ? 'number' :
            param.type === 'boolean' ? 'select' :
            param.type === 'date' ? 'date' :
            param.type === 'array' ? 'multiselect' : 'text',
      required: param.required || false,
      placeholder: param.description || `Enter ${param.name}`,
      options: param.options ? param.options.map(opt => ({ value: String(opt), label: String(opt) })) : undefined
    }));
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'completed': return <CheckCircle className="h-4 w-4 text-green-500" />;
      case 'failed': return <XCircle className="h-4 w-4 text-red-500" />;
      case 'running': return <Loader2 className="h-4 w-4 text-blue-500 animate-spin" />;
      case 'pending': return <Clock className="h-4 w-4 text-yellow-500" />;
      default: return <Clock className="h-4 w-4 text-gray-500" />;
    }
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleString();
  };

  if (isLoading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin" />
      </div>
    );
  }

  if (!analysis) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center">
          <h2 className="text-xl font-semibold mb-2">Analysis not found</h2>
          <p className="text-muted-foreground">The analysis you're looking for doesn't exist.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {/* Header */}
      <div className="border-b px-6 py-4 bg-white">
        <div className="flex items-start justify-between">
          <div className="flex-1">
            <div className="flex items-center gap-3">
              <h1 className="text-2xl font-bold">{analysis.name}</h1>
              {currentJob && (
                <Badge variant={currentJob.status === 'completed' ? 'default' : 'secondary'}>
                  {getStatusIcon(currentJob.status)}
                  <span className="ml-1">{currentJob.status}</span>
                </Badge>
              )}
            </div>
            {analysis.description && (
              <p className="text-muted-foreground mt-1">{analysis.description}</p>
            )}
          </div>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowSchedules(true)}
            >
              <Calendar className="h-4 w-4 mr-2" />
              Schedules
            </Button>
            <Button
              onClick={handleExecute}
              disabled={isExecuting}
              size="sm"
              className="min-w-32"
            >
              {isExecuting ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin mr-2" />
                  Running...
                </>
              ) : (
                <>
                  <Play className="h-4 w-4 mr-2" />
                  Run Analysis
                </>
              )}
            </Button>
          </div>
        </div>
      </div>

      {/* Error Display */}
      {error && (
        <div className="px-6 py-4">
          <Alert variant="destructive">
            <AlertDescription className="flex items-start justify-between gap-4">
              <span className="flex-1">{error}</span>
              <Button
                size="sm"
                onClick={() => {
                  if (analysisId) {
                    createChatForAnalysisError(projectId, analysisId, error, navigate);
                  }
                }}
                className="shrink-0"
              >
                <MessageSquare className="h-4 w-4 mr-2" />
                Fix with Chat
              </Button>
            </AlertDescription>
          </Alert>
        </div>
      )}

      {/* Tabbed Interface */}
      <div className="flex-1 overflow-hidden">
        <Tabs value={activeTab} onValueChange={setActiveTab} className="h-full flex flex-col">
          <div className="border-b bg-white px-6">
            <TabsList className="grid w-full grid-cols-4 bg-transparent">
              <TabsTrigger value="overview" className="flex items-center gap-2">
                <BarChart3 className="h-4 w-4" />
                Overview
              </TabsTrigger>
              <TabsTrigger value="results" className="flex items-center gap-2">
                <CheckCircle className="h-4 w-4" />
                Results
              </TabsTrigger>
              <TabsTrigger value="configure" className="flex items-center gap-2">
                <Settings className="h-4 w-4" />
                Configure
              </TabsTrigger>
              <TabsTrigger value="code" className="flex items-center gap-2">
                <Code className="h-4 w-4" />
                Code
              </TabsTrigger>
            </TabsList>
          </div>

          <div className="flex-1 overflow-auto">
            {/* Overview Tab */}
            <TabsContent value="overview" className="m-0">
              <div className="p-6 space-y-6">
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                  {/* Quick Stats */}
                  <Card>
                    <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                      <CardTitle className="text-sm font-medium">Total Runs</CardTitle>
                      <BarChart3 className="h-4 w-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                      <div className="text-2xl font-bold">{executionHistory.length}</div>
                      <p className="text-xs text-muted-foreground">
                        All time executions
                      </p>
                    </CardContent>
                  </Card>

                  <Card>
                    <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                      <CardTitle className="text-sm font-medium">Success Rate</CardTitle>
                      <CheckCircle className="h-4 w-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                      <div className="text-2xl font-bold">
                        {executionHistory.length > 0
                          ? Math.round((executionHistory.filter(j => j.status === 'completed').length / executionHistory.length) * 100)
                          : 0}%
                      </div>
                      <p className="text-xs text-muted-foreground">
                        Last 10 executions
                      </p>
                    </CardContent>
                  </Card>

                  <Card>
                    <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                      <CardTitle className="text-sm font-medium">Last Run</CardTitle>
                      <Clock className="h-4 w-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                      <div className="text-2xl font-bold">
                        {executionHistory.length > 0
                          ? new Date(executionHistory[0].created_at).toLocaleDateString()
                          : 'Never'}
                      </div>
                      <p className="text-xs text-muted-foreground">
                        Most recent execution
                      </p>
                    </CardContent>
                  </Card>
                </div>

                {/* Quick Actions */}
                <Card>
                  <CardHeader>
                    <CardTitle>Quick Actions</CardTitle>
                    <CardDescription>
                      Common tasks for this analysis
                    </CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-4">
                    <div className="flex flex-wrap gap-3">
                      <Button
                        onClick={handleExecute}
                        disabled={isExecuting}
                        className="min-w-32"
                      >
                        {isExecuting ? (
                          <>
                            <Loader2 className="h-4 w-4 animate-spin mr-2" />
                            Running...
                          </>
                        ) : (
                          <>
                            <Play className="h-4 w-4 mr-2" />
                            Run Now
                          </>
                        )}
                      </Button>

                      <Button
                        variant="outline"
                        onClick={() => setActiveTab('configure')}
                      >
                        <Settings className="h-4 w-4 mr-2" />
                        Configure Filters
                      </Button>

                      <Button
                        variant="outline"
                        onClick={() => setActiveTab('results')}
                      >
                        <BarChart3 className="h-4 w-4 mr-2" />
                        View Results
                      </Button>

                      <Button
                        variant="outline"
                        onClick={() => setShowSchedules(true)}
                      >
                        <Calendar className="h-4 w-4 mr-2" />
                        Schedule
                      </Button>
                    </div>
                  </CardContent>
                </Card>

                {/* Current Configuration */}
                {analysis.parameters && analysis.parameters.length > 0 && (
                  <Card>
                    <CardHeader>
                      <CardTitle>Current Configuration</CardTitle>
                      <CardDescription>
                        Active filters and parameters
                      </CardDescription>
                    </CardHeader>
                    <CardContent>
                      <div className="space-y-3">
                        {Object.entries(filterValues).map(([key, value]) => {
                          const param = analysis.parameters?.find(p => p.name === key);
                          if (!param || value === '' || value === null || value === undefined) return null;

                          return (
                            <div key={key} className="flex items-center justify-between py-2 border-b">
                              <div>
                                <span className="font-medium">{param.name}</span>
                                <span className="text-sm text-muted-foreground ml-2">
                                  {param.description}
                                </span>
                              </div>
                              <Badge variant="secondary">
                                {Array.isArray(value) ? value.join(', ') : String(value)}
                              </Badge>
                            </div>
                          );
                        })}
                        {Object.keys(filterValues).length === 0 && (
                          <p className="text-muted-foreground text-center py-4">
                            No filters configured. Click "Configure Filters" to set them up.
                          </p>
                        )}
                      </div>
                    </CardContent>
                  </Card>
                )}
              </div>
            </TabsContent>

            {/* Results Tab */}
            <TabsContent value="results" className="m-0">
              <div className="p-6 space-y-6">
                {/* Current Job Status */}
                {currentJob && (
                  <Card>
                    <CardHeader>
                      <CardTitle className="flex items-center gap-2">
                        Current Execution
                        {getStatusIcon(currentJob.status)}
                      </CardTitle>
                      <CardDescription>
                        Started: {formatDate(currentJob.created_at)}
                      </CardDescription>
                    </CardHeader>
                    <CardContent>
                      {currentJob.status === 'running' && (
                        <div className="flex items-center gap-2 text-blue-600">
                          <Loader2 className="h-4 w-4 animate-spin" />
                          <span>Analysis is running...</span>
                        </div>
                      )}

                      {currentJob.status === 'completed' && currentJob.result && (
                        <div className="space-y-4">
                          <div className="text-green-600 font-medium">
                            ✓ Analysis completed successfully
                          </div>

                          {currentJob.result.data && (
                            <div>
                              <h4 className="font-medium mb-2">Results</h4>
                              <pre className="bg-muted p-4 rounded-lg overflow-auto text-sm max-h-64">
                                {JSON.stringify(currentJob.result.data, null, 2)}
                              </pre>
                            </div>
                          )}

                          <Button variant="outline" size="sm">
                            <Download className="h-4 w-4 mr-2" />
                            Download Results
                          </Button>
                        </div>
                      )}

                      {currentJob.status === 'failed' && currentJob.error && (
                        <div className="text-red-600">
                          ✗ Analysis failed: {currentJob.error}
                        </div>
                      )}
                    </CardContent>
                  </Card>
                )}

                {/* Execution History */}
                <Card>
                  <CardHeader>
                    <CardTitle>Execution History</CardTitle>
                    <CardDescription>
                      Recent runs of this analysis
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    {executionHistory.length === 0 ? (
                      <div className="text-center py-8 text-muted-foreground">
                        <BarChart3 className="h-8 w-8 mx-auto mb-3 opacity-50" />
                        <p>No executions yet</p>
                        <p className="text-sm">Run the analysis to see results here</p>
                      </div>
                    ) : (
                      <div className="space-y-3">
                        {executionHistory.map((job) => (
                          <div key={job.id} className="flex items-center justify-between p-3 border rounded-lg">
                            <div className="flex items-center gap-3">
                              {getStatusIcon(job.status)}
                              <div>
                                <div className="font-medium">
                                  {job.status === 'completed' ? 'Successful' :
                                   job.status === 'failed' ? 'Failed' :
                                   job.status === 'running' ? 'Running' : 'Pending'}
                                </div>
                                <div className="text-sm text-muted-foreground">
                                  {formatDate(job.created_at)}
                                  {job.completed_at && ` - Completed in ${Math.round((new Date(job.completed_at).getTime() - new Date(job.created_at).getTime()) / 1000)}s`}
                                </div>
                              </div>
                            </div>

                            <div className="flex items-center gap-2">
                              {job.status === 'completed' && (
                                <Button variant="outline" size="sm">
                                  <Download className="h-4 w-4 mr-2" />
                                  Export
                                </Button>
                              )}

                              {job.parameters && Object.keys(job.parameters).length > 0 && (
                                <Button variant="ghost" size="sm">
                                  View Config
                                </Button>
                              )}
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </CardContent>
                </Card>
              </div>
            </TabsContent>

            {/* Configure Tab */}
            <TabsContent value="configure" className="m-0">
              <div className="p-6">
                {analysis.parameters && analysis.parameters.length > 0 ? (
                  <div className="space-y-6">
                    <div>
                      <h2 className="text-xl font-semibold mb-2">Configure Analysis Filters</h2>
                      <p className="text-muted-foreground">
                        Set parameters for your analysis. These will be used when you run the analysis.
                      </p>
                    </div>

                    <DynamicFilters
                      analysisId={analysisId}
                      filters={convertParametersToFilters(analysis.parameters)}
                      values={filterValues}
                      onChange={handleFilterChange}
                      onAddFilter={handleAddFilter}
                      onRemoveFilter={handleRemoveFilter}
                    />
                  </div>
                ) : (
                  <div className="text-center py-12">
                    <Settings className="h-12 w-12 mx-auto mb-4 text-muted-foreground opacity-50" />
                    <h3 className="text-lg font-medium mb-2">No Configuration Required</h3>
                    <p className="text-muted-foreground">
                      This analysis doesn't require any parameters or filters.
                    </p>
                  </div>
                )}
              </div>
            </TabsContent>

            {/* Code Tab */}
            <TabsContent value="code" className="m-0">
              <div className="p-6">
                <div className="max-w-4xl">
                  <div className="flex items-center justify-between mb-6">
                    <div>
                      <h2 className="text-xl font-semibold mb-2">Analysis Code</h2>
                      <p className="text-muted-foreground">
                        View and edit the underlying script for this analysis
                      </p>
                    </div>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => navigate(`/p/${projectId}/analysis/${analysisId}/edit`)}
                    >
                      <Edit className="h-4 w-4 mr-2" />
                      Edit Code
                    </Button>
                  </div>

                  <Card>
                    <CardContent className="p-0">
                      <pre className="bg-muted p-6 rounded-lg overflow-auto text-sm font-mono max-h-96">
                        {analysis.config?.code || analysis.config?.query || 'No code available'}
                      </pre>
                    </CardContent>
                  </Card>
                </div>
              </div>
            </TabsContent>
          </div>
        </Tabs>
      </div>

      {/* Schedules Dialog */}
      {showSchedules && analysisId && (
        <AnalysisSchedulesDialog
          analysisId={analysisId}
          open={showSchedules}
          onClose={() => setShowSchedules(false)}
        />
      )}
    </div>
  );
}