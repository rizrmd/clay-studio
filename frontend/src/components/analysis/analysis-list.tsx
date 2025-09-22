interface AnalysisListProps {
  projectId: string;
}

export function AnalysisList({ projectId }: AnalysisListProps) {
  return (
    <div className="flex-1 flex items-center justify-center">
      <div className="text-center">
        <h2 className="text-2xl font-bold mb-4">Analysis List</h2>
        <p className="text-muted-foreground">Project: {projectId}</p>
      </div>
    </div>
  );
}