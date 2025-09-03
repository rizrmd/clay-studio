import { FileText } from "lucide-react";
import { Button } from "@/components/ui/button";

interface FileUploadProgressProps {
  uploadProgress: Record<string, number>;
  onCancel: () => void;
}

export function FileUploadProgress({ uploadProgress, onCancel }: FileUploadProgressProps) {
  if (Object.keys(uploadProgress).length === 0) return null;

  return (
    <div className="mb-2 space-y-2">
      <div className="flex items-center justify-between">
        <div className="text-xs text-muted-foreground">
          Uploading files...
        </div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={onCancel}
          className="h-6 px-2 text-xs"
        >
          Cancel
        </Button>
      </div>
      {Object.entries(uploadProgress).map(([fileName, progress]) => (
        <div key={fileName} className="space-y-1">
          <div className="flex items-center gap-2">
            <FileText className="h-3 w-3" />
            <span className="text-xs flex-1 truncate">{fileName}</span>
            <span className="text-xs text-muted-foreground">
              {Math.round(progress)}%
            </span>
          </div>
          <div className="h-1 bg-secondary rounded-full overflow-hidden">
            <div
              className="h-full bg-primary transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>
        </div>
      ))}
    </div>
  );
}