import { Button } from "@/components/ui/button";

interface LoadingStateProps {
  onClose?: () => void;
}

interface ErrorStateProps {
  error: string;
  onClose?: () => void;
}

export const DatasourcesLoadingState = ({ onClose: _ }: LoadingStateProps) => (
  <div className="flex items-center justify-center h-full">
    <div className="text-center">
      <p className="text-muted-foreground">Loading datasources...</p>
    </div>
  </div>
);

export const DatasourcesErrorState = ({ error, onClose }: ErrorStateProps) => (
  <div className="flex items-center justify-center h-full">
    <div className="text-center">
      <p className="text-red-600 mb-2">Failed to load datasources</p>
      <p className="text-sm text-muted-foreground mb-4">{error}</p>
      {onClose && (
        <Button variant="outline" onClick={onClose} className="mt-2">
          Close
        </Button>
      )}
    </div>
  </div>
);

export const DatasourceNotFoundState = ({ onClose }: LoadingStateProps) => (
  <div className="flex items-center justify-center h-full">
    <div className="text-center">
      <p className="text-muted-foreground">Datasource not found</p>
      {onClose && (
        <Button variant="outline" onClick={onClose} className="mt-2">
          Close
        </Button>
      )}
    </div>
  </div>
);

export const StructureLoadingState = () => (
  <div className="flex items-center justify-center h-full">
    <div className="flex items-center gap-2">
      <div className="animate-spin h-4 w-4 border-2 border-primary border-t-transparent rounded-full"></div>
      <span className="text-sm text-muted-foreground">Loading...</span>
    </div>
  </div>
);

export const NoTableSelectedState = ({
  currentMode,
}: {
  currentMode: "data" | "structure";
}) => (
  <div className="flex items-center justify-center h-full">
    <div className="text-center">
      <p className="text-muted-foreground">
        Select a table to view {currentMode === "data" ? "data" : "structure"}
      </p>
      <p className="text-sm text-muted-foreground mt-1">
        Choose a table from the sidebar to view its{" "}
        {currentMode === "data" ? "data with inline editing" : "structure"}
      </p>
    </div>
  </div>
);
