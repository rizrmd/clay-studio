interface AnalysisEditorProps {
  analysisId?: string;
  projectId: string;
  mode?: string;
}

export function AnalysisEditor({ analysisId, projectId, mode }: AnalysisEditorProps) {
  return (
    <div className="flex-1 flex items-center justify-center">
      <div className="text-center">
        <h2 className="text-2xl font-bold mb-4">Analysis Editor</h2>
        <p className="text-muted-foreground">
          {mode === 'create' ? 'Create a new analysis' : `Editing analysis: ${analysisId}`}
        </p>
        <p className="text-sm text-muted-foreground mt-2">Project: {projectId}</p>
      </div>
    </div>
  );
}