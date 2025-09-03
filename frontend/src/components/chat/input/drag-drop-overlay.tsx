import { FileText } from "lucide-react";

interface DragDropOverlayProps {
  isDragging: boolean;
}

export function DragDropOverlay({ isDragging }: DragDropOverlayProps) {
  if (!isDragging) return null;

  return (
    <div className="absolute inset-0 z-50 flex items-center justify-center pointer-events-none">
      <div className="text-center">
        <FileText className="h-12 w-12 mx-auto mb-2 text-primary animate-pulse" />
        <p className="text-lg font-semibold text-primary">
          Drop files here
        </p>
        <p className="text-sm text-muted-foreground">
          Release to upload files to this chat
        </p>
      </div>
    </div>
  );
}