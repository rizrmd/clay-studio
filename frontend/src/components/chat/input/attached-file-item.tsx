import { FileText, X, Edit2, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

interface AttachedFileItemProps {
  file: File;
  description: string;
  preview?: string;
  isEditingDescription: boolean;
  isUploading?: boolean;
  onRemove: () => void;
  onToggleEdit: () => void;
  onUpdateDescription: (description: string) => void;
}

export function AttachedFileItem({
  file,
  description,
  preview,
  isEditingDescription,
  isUploading = false,
  onRemove,
  onToggleEdit,
  onUpdateDescription,
}: AttachedFileItemProps) {
  const isImage = file.type.startsWith("image/");

  return (
    <div className="border rounded-lg p-2 space-y-2">
      <div className="flex items-start gap-2">
        {/* Preview or icon */}
        {isImage && preview ? (
          <img
            src={preview}
            alt={file?.name}
            className="w-16 h-16 object-cover rounded border"
          />
        ) : (
          <div className="w-16 h-16 flex items-center justify-center border rounded bg-secondary">
            <FileText className="h-6 w-6 text-muted-foreground" />
          </div>
        )}

        {/* File info and controls */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium truncate">
              {file?.name}
            </span>
            <button
              type="button"
              onClick={onRemove}
              className="ml-2 p-1 hover:text-destructive"
              disabled={isUploading}
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Description */}
          <div className="mt-1">
            {isEditingDescription ? (
              <div className="flex gap-1">
                <Input
                  type="text"
                  placeholder="Add a description..."
                  value={description}
                  onChange={(e) => onUpdateDescription(e.target.value)}
                  className="h-7 text-xs"
                  autoFocus
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      onToggleEdit();
                    }
                  }}
                />
                <Button
                  type="button"
                  size="icon"
                  variant="ghost"
                  className="h-7 w-7"
                  onClick={onToggleEdit}
                >
                  <Check className="h-3 w-3" />
                </Button>
              </div>
            ) : (
              <div className="flex items-center gap-1 group">
                <div className="text-xs text-muted-foreground">
                  <div>{description || "No description"}</div>
                  <div className="opacity-70 mt-1">
                    {Math.round(file.size / 1024)} KB •{" "}
                    {file.type || "Unknown type"}
                  </div>
                </div>
                <Button
                  type="button"
                  size="icon"
                  variant="ghost"
                  className="h-5 w-5 opacity-0 group-hover:opacity-100 transition-opacity"
                  onClick={onToggleEdit}
                >
                  <Edit2 className="h-3 w-3" />
                </Button>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}