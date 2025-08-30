import { FileText, Download } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { FileAttachment } from "../types";
import { formatFileSize, handleDownloadFile } from "../utils";

interface FileAttachmentsProps {
  attachments: FileAttachment[];
}

export function FileAttachments({ attachments }: FileAttachmentsProps) {
  if (!attachments || attachments.length === 0) return null;

  return (
    <div className="flex flex-wrap gap-2 mt-2">
      {attachments.map((file) => (
        <Badge
          key={file.id}
          variant="secondary"
          className="flex items-center gap-2 px-3 py-1.5 cursor-pointer hover:bg-secondary/80"
          onClick={() => handleDownloadFile(file)}
        >
          <FileText className="h-3 w-3" />
          <span className="text-xs">
            {file.original_name || file.file_name}
          </span>
          <span className="text-xs text-muted-foreground">
            ({formatFileSize(file.file_size)})
          </span>
          <Button
            variant="ghost"
            size="sm"
            className="h-4 w-4 p-0 hover:bg-transparent"
            onClick={(e) => {
              e.stopPropagation();
              handleDownloadFile(file);
            }}
          >
            <Download className="h-3 w-3" />
          </Button>
        </Badge>
      ))}
    </div>
  );
}