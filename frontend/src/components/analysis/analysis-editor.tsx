import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Calendar, Edit, Loader2, MessageSquare, Play, Save, Trash2, X } from 'lucide-react';
import { analysisApi } from '@/lib/api/analysis';
import { Analysis, AnalysisParameter, analysisActions } from '@/lib/store/analysis-store';
import { AnalysisSchedulesDialog } from './analysis-schedules-dialog';
import { createChatForAnalysisError } from '@/lib/utils/chat-helpers';
import { tabsStore, tabsActions } from '@/lib/store/tabs-store';

interface AnalysisEditorProps {
  analysisId?: string;
  projectId: string;
  mode?: string;
}

type ViewMode = 'preview' | 'edit';

export function AnalysisEditor({ analysisId, projectId, mode }: AnalysisEditorProps) {
  const navigate = useNavigate();
  const [viewMode, setViewMode] = useState<ViewMode>(mode === 'create' ? 'edit' : 'preview');
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showSchedules, setShowSchedules] = useState(false);
  const [loadedAnalysisId, setLoadedAnalysisId] = useState<string | null>(null);

  // Form state for editing
  const [editForm, setEditForm] = useState({
    title: '',
    description: '',
    script_content: '',
    tags: [] as string[],
  });

  // Parameter values for execution
  const [parameterValues, setParameterValues] = useState<Record<string, any>>({});

  // Job execution state
  const [jobId, setJobId] = useState<string | null>(null);
  const [jobStatus, setJobStatus] = useState<string | null>(null);
  const [jobResult, setJobResult] = useState<any>(null);

  // Load analysis data
  useEffect(() => {
    if (analysisId && mode !== 'create' && analysisId !== loadedAnalysisId) {
      loadAnalysis();
    }
  }, [analysisId, mode]);

  const loadAnalysis = async () => {
    if (!analysisId || isLoading || analysisId === loadedAnalysisId) return; // Prevent multiple concurrent loads

    setIsLoading(true);
    setError(null);

    try {
      const data = await analysisApi.getAnalysis(analysisId);
      setAnalysis(data);
      setLoadedAnalysisId(analysisId); // Mark this analysis as loaded

      // Initialize edit form
      setEditForm({
        title: data.name || '',
        description: data.description || '',
        script_content: data.config?.code || data.config?.query || '',
        tags: [],
      });

      // Initialize parameter default values
      const defaults: Record<string, any> = {};
      data.parameters?.forEach(param => {
        if (param.default !== undefined) {
          defaults[param.name] = param.default;
        }
      });
      setParameterValues(defaults);

      // Check if there's a running job for this analysis
      if (data.last_job && (data.last_job.status === 'pending' || data.last_job.status === 'running')) {
        // Resume polling for this job
        setJobId(data.last_job.id);
        setJobStatus(data.last_job.status);
        setIsExecuting(true);
        analysisActions.updateAnalysis(analysisId, { status: 'running' });
        pollJobStatus(data.last_job.id);
      }
    } catch (err: any) {
      setError(err.message || 'Failed to load analysis');
    } finally {
      setIsLoading(false);
    }
  };

  const handleSave = async () => {
    setIsSaving(true);
    setError(null);

    try {
      if (analysisId) {
        // Update existing analysis
        await analysisApi.updateAnalysis(analysisId, {
          name: editForm.title,
          description: editForm.description,
          config: {
            query: editForm.script_content,
            code: editForm.script_content,
          },
        });
        await loadAnalysis();
        setViewMode('preview');
      } else {
        // Create new analysis
        const newAnalysis = await analysisApi.createAnalysis(projectId, {
          name: editForm.title,
          description: editForm.description,
          type: 'sql', // Default to SQL, could be configurable
          config: {
            code: editForm.script_content,
          },
        });
        navigate(`/p/${projectId}/analysis/${newAnalysis.id}`);
      }
    } catch (err: any) {
      setError(err.message || 'Failed to save analysis');
    } finally {
      setIsSaving(false);
    }
  };

  const handleExecute = async () => {
    if (!analysisId) return;

    setIsExecuting(true);
    setError(null);
    setJobId(null);
    setJobStatus(null);
    setJobResult(null);

    // Update analysis status in store to 'running'
    analysisActions.updateAnalysis(analysisId, { status: 'running' });

    try {
      const response = await analysisApi.executeAnalysis(analysisId, {
        analysis_id: analysisId,
        parameters: parameterValues,
      });

      setJobId(response.job_id);
      setJobStatus('pending');

      // Start polling for job status
      pollJobStatus(response.job_id);
    } catch (err: any) {
      setError(err.message || 'Failed to execute analysis');
      setIsExecuting(false);
      // Update status back to idle on error
      analysisActions.updateAnalysis(analysisId, { status: 'idle' });
    }
  };

  const pollJobStatus = async (currentJobId: string) => {
    if (!analysisId) return;

    try {
      const job = await analysisApi.getJob(currentJobId);
      setJobStatus(job.status);

      if (job.status === 'completed') {
        const result = await analysisApi.getJobResult(currentJobId);
        setJobResult(result);
        setIsExecuting(false);
        // Update analysis status to completed
        analysisActions.updateAnalysis(analysisId, { status: 'completed' });
      } else if (job.status === 'failed' || job.status === 'cancelled') {
        const errorMsg = job.error || 'Job failed with unknown error';
        setError(errorMsg);
        setIsExecuting(false);
        // Update analysis status to failed
        analysisActions.updateAnalysis(analysisId, { status: 'failed' });
      } else if (job.status === 'running' || job.status === 'pending') {
        // Continue polling
        setTimeout(() => pollJobStatus(currentJobId), 2000);
      }
    } catch (err: any) {
      setError(err.message || 'Failed to get job status');
      setIsExecuting(false);
      // Update status back to idle on error
      analysisActions.updateAnalysis(analysisId, { status: 'idle' });
    }
  };

  const handleDelete = async () => {
    if (!analysisId || !confirm('Are you sure you want to delete this analysis?')) return;

    try {
      await analysisApi.deleteAnalysis(analysisId);

      // Remove from store
      analysisActions.removeAnalysis(analysisId);

      // Close the current tab
      const currentTab = tabsStore.tabs.find(
        t => t.type === 'analysis' && t.metadata.analysisId === analysisId
      );
      if (currentTab) {
        tabsActions.removeTab(currentTab.id);
      }

      // Navigate to project analysis list
      navigate(`/p/${projectId}/analysis`);
    } catch (err: any) {
      setError(err.message || 'Failed to delete analysis');
    }
  };

  // Keyboard shortcut for save (Cmd+S / Ctrl+S)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        if (viewMode === 'edit' && !isSaving) {
          handleSave();
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [viewMode, isSaving, editForm]);

  const renderParameterInput = (param: AnalysisParameter) => {
    const value = parameterValues[param.name] ?? '';

    const handleChange = (newValue: any) => {
      setParameterValues(prev => ({
        ...prev,
        [param.name]: newValue,
      }));
    };

    switch (param.type) {
      case 'number':
        return (
          <Input
            type="number"
            value={value}
            onChange={(e) => handleChange(parseFloat(e.target.value))}
            required={param.required}
          />
        );

      case 'boolean':
        return (
          <Select value={String(value)} onValueChange={(v) => handleChange(v === 'true')}>
            <SelectTrigger>
              <SelectValue placeholder="Select..." />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="true">Yes</SelectItem>
              <SelectItem value="false">No</SelectItem>
            </SelectContent>
          </Select>
        );

      case 'date':
        return (
          <Input
            type="date"
            value={value}
            onChange={(e) => handleChange(e.target.value)}
            required={param.required}
          />
        );

      case 'array':
        if (param.options && param.options.length > 0) {
          return (
            <Select value={value} onValueChange={handleChange}>
              <SelectTrigger>
                <SelectValue placeholder="Select..." />
              </SelectTrigger>
              <SelectContent>
                {param.options.map((option: any) => (
                  <SelectItem key={option} value={String(option)}>
                    {String(option)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          );
        }
        return (
          <Input
            type="text"
            value={value}
            onChange={(e) => handleChange(e.target.value)}
            required={param.required}
          />
        );

      case 'object':
      case 'string':
      default:
        return (
          <Input
            type="text"
            value={value}
            onChange={(e) => handleChange(e.target.value)}
            required={param.required}
          />
        );
    }
  };

  if (isLoading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin" />
      </div>
    );
  }

  // Preview Mode
  if (viewMode === 'preview' && analysis) {
    return (
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Header */}
        <div className="border-b px-6 py-4">
          <div className="flex items-start justify-between">
            <div className="flex-1">
              <h1 className="text-2xl font-bold">{analysis.name}</h1>
              {analysis.description && (
                <p className="text-muted-foreground mt-1">{analysis.description}</p>
              )}
            </div>
            <div className="flex gap-2">
              <Button variant="outline" size="sm" onClick={() => setShowSchedules(true)}>
                <Calendar className="h-4 w-4 mr-2" />
                Schedules
              </Button>
              <Button variant="outline" size="sm" onClick={() => setViewMode('edit')}>
                <Edit className="h-4 w-4 mr-2" />
                Edit
              </Button>
            </div>
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-auto p-6">
          {error && (
            <Alert variant="destructive" className="mb-4">
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
          )}

          {/* Parameters/Filters Section */}
          {analysis.parameters && analysis.parameters.length > 0 && (
            <div className="mb-6">
              <h3 className="text-lg font-semibold mb-4">Filters</h3>
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {analysis.parameters.map((param) => (
                  <div key={param.name} className="space-y-2">
                    <Label>
                      {param.name}
                      {param.required && <span className="text-destructive ml-1">*</span>}
                    </Label>
                    {param.description && (
                      <p className="text-xs text-muted-foreground">{param.description}</p>
                    )}
                    {renderParameterInput(param)}
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Submit Button */}
          <div className="mb-6">
            <Button
              onClick={handleExecute}
              disabled={isExecuting}
              size="lg"
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

          {/* Results Section */}
          {jobId && (
            <div className="border rounded-lg p-4">
              <h3 className="text-lg font-semibold mb-4">Results</h3>

              {/* Job Status */}
              <div className="mb-4">
                <span className="text-sm text-muted-foreground">Status: </span>
                <span className={`text-sm font-medium ${
                  jobStatus === 'completed' ? 'text-green-600' :
                  jobStatus === 'failed' ? 'text-red-600' :
                  'text-yellow-600'
                }`}>
                  {jobStatus}
                </span>
              </div>

              {/* Results Display */}
              {jobResult && (
                <div className="space-y-4">
                  {jobResult.data && (
                    <div>
                      <pre className="bg-muted p-4 rounded-lg overflow-auto">
                        {JSON.stringify(jobResult.data, null, 2)}
                      </pre>
                    </div>
                  )}
                  {jobResult.statistics && (
                    <div>
                      <h4 className="font-medium mb-2">Statistics</h4>
                      <pre className="bg-muted p-4 rounded-lg overflow-auto text-sm">
                        {JSON.stringify(jobResult.statistics, null, 2)}
                      </pre>
                    </div>
                  )}
                </div>
              )}
            </div>
          )}
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

  // Edit Mode
  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {/* Header */}
      <div className="border-b px-6 py-4">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-bold">
            {analysisId ? 'Edit Analysis' : 'Create Analysis'}
          </h1>
          <div className="flex gap-2">
            {analysisId && (
              <Button variant="ghost" size="sm" onClick={() => setViewMode('preview')}>
                <X className="h-4 w-4 mr-2" />
                Cancel
              </Button>
            )}
            {analysisId && (
              <Button variant="destructive" size="sm" onClick={handleDelete}>
                <Trash2 className="h-4 w-4 mr-2" />
                Delete
              </Button>
            )}
            <Button onClick={handleSave} disabled={isSaving} size="sm">
              {isSaving ? (
                <Loader2 className="h-4 w-4 animate-spin mr-2" />
              ) : (
                <Save className="h-4 w-4 mr-2" />
              )}
              Save
            </Button>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-6">
        {error && (
          <Alert variant="destructive" className="mb-4">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        <div className="max-w-4xl space-y-6">
          {/* Title */}
          <div className="space-y-2">
            <Label htmlFor="title">Title *</Label>
            <Input
              id="title"
              value={editForm.title}
              onChange={(e) => setEditForm(prev => ({ ...prev, title: e.target.value }))}
              placeholder="Analysis title"
              required
            />
          </div>

          {/* Description */}
          <div className="space-y-2">
            <Label htmlFor="description">Description</Label>
            <Textarea
              id="description"
              value={editForm.description}
              onChange={(e) => setEditForm(prev => ({ ...prev, description: e.target.value }))}
              placeholder="Describe what this analysis does..."
              rows={3}
            />
          </div>

          {/* Script Editor */}
          <div className="space-y-2">
            <Label htmlFor="script">Script *</Label>
            <p className="text-sm text-muted-foreground">
              Enter the JavaScript code that will be executed for this analysis.
            </p>
            <Textarea
              id="script"
              value={editForm.script_content}
              onChange={(e) => setEditForm(prev => ({ ...prev, script_content: e.target.value }))}
              placeholder="// Your JavaScript code here..."
              className="font-mono text-sm min-h-[400px]"
              required
            />
          </div>

          {/* TODO: Parameters configuration UI will be added in next iteration */}
        </div>
      </div>
    </div>
  );
}