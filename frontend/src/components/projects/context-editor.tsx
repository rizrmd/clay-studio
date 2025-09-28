import { useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Textarea } from '@/components/ui/textarea';
import { api } from '@/lib/utils/api';
import { proxy, useSnapshot } from 'valtio';
import { Loader2, Save, Play, RefreshCw, HelpCircle } from 'lucide-react';
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
  lastSaved: null as Date | null,
  lastCompiled: null as Date | null,
});

export function ContextEditor({ projectId }: ContextEditorProps) {
  const state = useSnapshot(contextEditorState);

  useEffect(() => {
    loadContext();
  }, [projectId]);

  const loadContext = async () => {
    contextEditorState.isLoading = true;
    contextEditorState.error = null;
    
    try {
      const response = await api.get(`/projects/${projectId}/context`);
      contextEditorState.sourceContent = response.context || '';
      contextEditorState.compiledContent = response.context_compiled || '';
      contextEditorState.lastCompiled = response.context_compiled_at 
        ? new Date(response.context_compiled_at) 
        : null;
      contextEditorState.isDirty = false;
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
      await api.put(`/projects/${projectId}/context`, {
        context: contextEditorState.sourceContent,
      });
      contextEditorState.isDirty = false;
      contextEditorState.lastSaved = new Date();
      
      // Clear compiled cache since source changed
      contextEditorState.compiledContent = '';
      contextEditorState.lastCompiled = null;
    } catch (error: any) {
      contextEditorState.error = error.response?.data?.message || 'Failed to save context';
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
      contextEditorState.isDirty = true;
    }
  };

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
            {state.isDirty && (
              <span className="text-sm text-muted-foreground">Unsaved changes</span>
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
              Save
            </Button>
          </div>
        </div>
      </div>
      {state.error && (
        <Alert variant="destructive" className="mx-4 mt-4">
          <AlertDescription>{state.error}</AlertDescription>
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