import { useEffect, useCallback } from 'react';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Textarea } from '@/components/ui/textarea';
import { api } from '@/lib/utils/api';
import { proxy, useSnapshot } from 'valtio';
import { Loader2, Save, Play, RefreshCw, HelpCircle, RotateCcw } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';

interface ContextEditorProps {
  projectId: string;
}

const contextEditorState = proxy({
  sourceContent: '',
  compiledContent: '',
  isLoading: false,
  isSaving: false,
  isCompiling: false,
  error: null as string | null,
  isDirty: false,
  hasUnsavedLocalChanges: false,
  lastSaved: null as Date | null,
  lastCompiled: null as Date | null,
  originalContent: '',
});

// LocalStorage keys
const getContextLocalStorageKey = (projectId: string) => `context-editor-unsaved-${projectId}`;
const getContextTimestampKey = (projectId: string) => `context-editor-timestamp-${projectId}`;

// Save to localStorage utility
const saveToLocalStorage = (projectId: string, content: string) => {
  try {
    localStorage.setItem(getContextLocalStorageKey(projectId), content);
    localStorage.setItem(getContextTimestampKey(projectId), new Date().toISOString());
  } catch (error) {
    console.warn('Failed to save to localStorage:', error);
  }
};

// Load from localStorage utility
const loadFromLocalStorage = (projectId: string) => {
  try {
    const content = localStorage.getItem(getContextLocalStorageKey(projectId));
    const timestamp = localStorage.getItem(getContextTimestampKey(projectId));
    if (content && timestamp) {
      return {
        content,
        timestamp: new Date(timestamp)
      };
    }
  } catch (error) {
    console.warn('Failed to load from localStorage:', error);
  }
  return null;
};

// Clear localStorage utility
const clearLocalStorage = (projectId: string) => {
  try {
    localStorage.removeItem(getContextLocalStorageKey(projectId));
    localStorage.removeItem(getContextTimestampKey(projectId));
  } catch (error) {
    console.warn('Failed to clear localStorage:', error);
  }
};

export function ContextEditor({ projectId }: ContextEditorProps) {
  const state = useSnapshot(contextEditorState);

  useEffect(() => {
    loadContext();
  }, [projectId]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        if (contextEditorState.isDirty && !contextEditorState.isSaving) {
          saveContext();
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [projectId]);

  // Auto-save to localStorage when content changes and is dirty
  useEffect(() => {
    if (contextEditorState.isDirty && contextEditorState.sourceContent !== contextEditorState.originalContent) {
      const timeoutId = setTimeout(() => {
        saveToLocalStorage(projectId, contextEditorState.sourceContent);
      }, 1000); // Debounce for 1 second

      return () => clearTimeout(timeoutId);
    }
  }, [contextEditorState.sourceContent, contextEditorState.isDirty, contextEditorState.originalContent, projectId]);

  // Update hasUnsavedLocalChanges based on localStorage state
  useEffect(() => {
    const checkUnsavedLocalChanges = () => {
      const localData = loadFromLocalStorage(projectId);
      if (localData && localData.content !== contextEditorState.originalContent) {
        contextEditorState.hasUnsavedLocalChanges = true;
      } else {
        contextEditorState.hasUnsavedLocalChanges = false;
      }
    };

    checkUnsavedLocalChanges();
  }, [contextEditorState.originalContent, projectId]);

  const loadContext = async () => {
    contextEditorState.isLoading = true;
    contextEditorState.error = null;

    try {
      const response = await api.get(`/projects/${projectId}/context`);
      const serverContent = response.context || '';
      contextEditorState.originalContent = serverContent;
      contextEditorState.compiledContent = response.context_compiled || '';
      contextEditorState.lastCompiled = response.context_compiled_at
        ? new Date(response.context_compiled_at)
        : null;

      // Check if there's unsaved content in localStorage
      const localData = loadFromLocalStorage(projectId);
      if (localData && localData.content !== serverContent) {
        // Use the unsaved local content
        contextEditorState.sourceContent = localData.content;
        contextEditorState.isDirty = true;
        contextEditorState.hasUnsavedLocalChanges = true;
      } else {
        // Use the server content
        contextEditorState.sourceContent = serverContent;
        contextEditorState.isDirty = false;
        contextEditorState.hasUnsavedLocalChanges = false;
        // Clear localStorage since it matches server
        clearLocalStorage(projectId);
      }
    } catch (error: any) {
      contextEditorState.error = error.response?.data?.message || 'Failed to load context';
    } finally {
      contextEditorState.isLoading = false;
    }
  };

  const saveContext = async () => {
    contextEditorState.isSaving = true;
    contextEditorState.error = null;

    try {
      // First save the context
      await api.put(`/projects/${projectId}/context`, {
        context: contextEditorState.sourceContent,
      });

      // Update state after successful save
      contextEditorState.originalContent = contextEditorState.sourceContent;
      contextEditorState.isDirty = false;
      contextEditorState.hasUnsavedLocalChanges = false;
      contextEditorState.lastSaved = new Date();

      // Clear localStorage since content is now saved on server
      clearLocalStorage(projectId);

      // Then compile it automatically
      const response = await api.post(`/projects/${projectId}/context/compile`);
      contextEditorState.compiledContent = response.compiled || '';
      contextEditorState.lastCompiled = new Date();
    } catch (error: any) {
      contextEditorState.error = error.response?.data?.message || 'Failed to save/compile context';
    } finally {
      contextEditorState.isSaving = false;
    }
  };


  const compileContext = async () => {
    contextEditorState.isCompiling = true;
    contextEditorState.error = null;
    
    try {
      // Save first if dirty
      if (contextEditorState.isDirty) {
        await saveContext();
      }
      
      // Force recompile (bypasses cache)
      const response = await api.post(`/projects/${projectId}/context/compile`);
      contextEditorState.compiledContent = response.compiled || '';
      contextEditorState.lastCompiled = new Date();
    } catch (error: any) {
      contextEditorState.error = error.response?.data?.message || 'Failed to compile context';
    } finally {
      contextEditorState.isCompiling = false;
    }
  };

  const clearCache = async () => {
    try {
      await api.delete(`/projects/${projectId}/context/cache`);
      contextEditorState.compiledContent = '';
      contextEditorState.lastCompiled = null;
    } catch (error: any) {
      contextEditorState.error = error.response?.data?.message || 'Failed to clear cache';
    }
  };

  const handleEditorChange = (value: string | undefined) => {
    if (value !== undefined) {
      contextEditorState.sourceContent = value;
      contextEditorState.isDirty = value !== contextEditorState.originalContent;
    }
  };

  const resetToOriginal = useCallback(() => {
    if (contextEditorState.hasUnsavedLocalChanges) {
      contextEditorState.sourceContent = contextEditorState.originalContent;
      contextEditorState.isDirty = false;
      contextEditorState.hasUnsavedLocalChanges = false;
      clearLocalStorage(projectId);
    }
  }, [projectId]);

  if (state.isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 className="h-8 w-8 animate-spin" />
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="border-b px-4 py-3">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold">Programmable Context</h2>
            <p className="text-sm text-muted-foreground">
              Define context that will be compiled and included in CLAUDE.md
            </p>
          </div>
          <div className="flex items-center gap-2">
            {state.hasUnsavedLocalChanges && (
              <div className="flex items-center gap-1 text-sm text-amber-600 bg-amber-50 px-2 py-1 rounded border border-amber-200">
                <span className="w-2 h-2 bg-amber-500 rounded-full"></span>
                <span>Unsaved changes (auto-saved locally)</span>
              </div>
            )}
            {state.isDirty && !state.hasUnsavedLocalChanges && (
              <span className="text-sm text-muted-foreground">Unsaved changes</span>
            )}
            {state.hasUnsavedLocalChanges && (
              <Button
                variant="outline"
                size="sm"
                onClick={resetToOriginal}
                className="text-amber-700 border-amber-300 hover:bg-amber-100"
              >
                <RotateCcw className="h-4 w-4 mr-1" />
                Reset
              </Button>
            )}
            <Dialog>
              <DialogTrigger asChild>
                <Button variant="outline" size="sm">
                  <HelpCircle className="h-4 w-4 mr-1" />
                  Help
                </Button>
              </DialogTrigger>
              <DialogContent className="max-w-2xl">
                <DialogHeader>
                  <DialogTitle>Programmable Context Guide</DialogTitle>
                  <DialogDescription>
                    Learn how to create dynamic context for your project
                  </DialogDescription>
                </DialogHeader>
                <div className="space-y-4">
                  <div>
                    <h4 className="font-medium text-sm mb-2">Quick Reference</h4>
                    <ul className="text-sm space-y-2 text-muted-foreground">
                      <li>• Use JavaScript code blocks with <code className="px-1 py-0.5 bg-muted rounded">```javascript</code></li>
                      <li>• Access database with <code className="px-1 py-0.5 bg-muted rounded">await ctx.query(`SELECT ...`)</code></li>
                      <li>• Return strings to replace code blocks in compiled output</li>
                      <li>• Context is cached for 5 minutes after compilation</li>
                      <li>• Compiled context is automatically included in CLAUDE.md</li>
                    </ul>
                  </div>
                  <div>
                    <h4 className="font-medium text-sm mb-2">Example</h4>
                    <pre className="p-3 bg-muted rounded-lg text-xs overflow-x-auto">
{`# Project Context

Write markdown with embedded JavaScript blocks to generate dynamic context.

## Database Schema

\`\`\`javascript
const result = await ctx.query(\`
  SELECT COUNT(*) as total_tables 
  FROM information_schema.tables 
  WHERE table_schema = 'public'
\`);
return \`Total tables: \${result[0].total_tables}\`;
\`\`\`

## Recent Activity

\`\`\`javascript
const recent = await ctx.query(\`
  SELECT COUNT(*) as count
  FROM messages
  WHERE created_at > NOW() - INTERVAL '24 hours'
\`);
return \`Messages in last 24h: \${recent[0].count}\`;
\`\`\``}
                    </pre>
                  </div>
                </div>
              </DialogContent>
            </Dialog>
            <Button
              variant="outline"
              size="sm"
              onClick={clearCache}
              disabled={!state.compiledContent}
            >
              <RefreshCw className="h-4 w-4 mr-1" />
              Clear Cache
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={compileContext}
              disabled={state.isCompiling || !state.sourceContent}
            >
              {state.isCompiling ? (
                <Loader2 className="h-4 w-4 animate-spin mr-1" />
              ) : (
                <Play className="h-4 w-4 mr-1" />
              )}
              Compile
            </Button>
            <Button
              size="sm"
              onClick={saveContext}
              disabled={state.isSaving || !state.isDirty}
            >
              {state.isSaving ? (
                <Loader2 className="h-4 w-4 animate-spin mr-1" />
              ) : (
                <Save className="h-4 w-4 mr-1" />
              )}
              Save ({navigator.platform.includes('Mac') ? 'Cmd' : 'Ctrl'}+S)
            </Button>
          </div>
        </div>
      </div>
      {state.error && (
        <Alert variant="destructive" className="mx-4 mt-4">
          <AlertDescription>{state.error}</AlertDescription>
        </Alert>
      )}
      {state.hasUnsavedLocalChanges && (
        <Alert className="mx-4 mt-4 border-amber-200 bg-amber-50">
          <AlertDescription className="text-amber-800">
            <strong>Content restored from auto-save:</strong> Your latest changes have been recovered from browser storage.
            You can continue editing or click "Reset" to discard these changes and return to the last saved version.
          </AlertDescription>
        </Alert>
      )}
      
      <div className="flex-1 flex overflow-hidden">
        {/* Editor Panel */}
        <div className="flex-1 flex flex-col p-4 border-r">
          <Textarea
            value={state.sourceContent}
            onChange={(e) => handleEditorChange(e.target.value)}
            className="flex-1 font-mono text-sm resize-none border rounded-lg p-3"
            placeholder={`# Project Context

Write markdown with embedded JavaScript blocks to generate dynamic context.

Click the Help button above for examples and syntax guide.`}
          />
          
          {(state.lastSaved || state.lastCompiled) && (
            <div className="flex gap-4 mt-2 text-sm text-muted-foreground">
              {state.lastSaved && (
                <span>Last saved: {state.lastSaved.toLocaleTimeString()}</span>
              )}
              {state.lastCompiled && (
                <span>Last compiled: {state.lastCompiled.toLocaleTimeString()}</span>
              )}
            </div>
          )}
        </div>
        
        {/* Preview Panel */}
        <div className="flex-1 flex flex-col p-4">
          <div className="mb-2">
            <h3 className="text-sm font-medium text-muted-foreground">Preview</h3>
          </div>
          <div className="flex-1 border rounded-lg bg-muted/20 p-4 overflow-auto">
            {state.compiledContent ? (
              <pre className="whitespace-pre-wrap font-mono text-sm">
                {state.compiledContent}
              </pre>
            ) : (
              <div className="text-muted-foreground text-sm">
                <p>No compiled output yet.</p>
                <p className="mt-2">Click "Compile" to see the processed context.</p>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}