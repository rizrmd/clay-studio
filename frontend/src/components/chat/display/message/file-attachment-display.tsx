import { useState } from "react";
import { Download, File, FileText, Image, Eye } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";

interface FileAttachment {
  id: string;
  name: string;
  contentType: string;
  url: string;
  size: number;
  description?: string;
  autoDescription?: string;
  isTextFile: boolean;
  preview?: string;
}

interface FileAttachmentDisplayProps {
  attachment: FileAttachment;
}

const formatFileSize = (bytes: number) => {
  if (bytes === 0) return "0 Bytes";
  const k = 1024;
  const sizes = ["Bytes", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
};

const getFileIcon = (mimeType: string, isText: boolean) => {
  if (isText) return FileText;
  if (mimeType.startsWith("image/")) return Image;
  return File;
};

export function FileAttachmentDisplay({ attachment }: FileAttachmentDisplayProps) {
  const [previewOpen, setPreviewOpen] = useState(false);
  const FileIcon = getFileIcon(attachment.contentType, attachment.isTextFile);

  const handleDownload = () => {
    const link = document.createElement("a");
    link.href = attachment.url;
    link.download = attachment.name;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  const canPreview = attachment.isTextFile && attachment.preview;

  return (
    <div className="flex items-start gap-2 p-3 border rounded-lg bg-muted/50 max-w-sm">
      <FileIcon className="h-4 w-4 mt-0.5 text-muted-foreground flex-shrink-0" />
      
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <span className="text-sm font-medium truncate" title={attachment.name}>
            {attachment.name}
          </span>
          {canPreview && (
            <Dialog open={previewOpen} onOpenChange={setPreviewOpen}>
              <DialogTrigger asChild>
                <Button
                  size="sm"
                  variant="ghost"
                  className="h-6 w-6 p-0"
                  title="Preview file"
                >
                  <Eye className="h-3 w-3" />
                </Button>
              </DialogTrigger>
              <DialogContent className="max-w-4xl max-h-[80vh]">
                <DialogHeader>
                  <DialogTitle className="flex items-center justify-between">
                    <span>{attachment.name}</span>
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={handleDownload}
                      className="ml-2"
                    >
                      <Download className="h-4 w-4 mr-2" />
                      Download
                    </Button>
                  </DialogTitle>
                </DialogHeader>
                <ScrollArea className="max-h-[60vh] w-full">
                  <pre className="text-xs whitespace-pre-wrap break-words p-4 bg-muted rounded">
                    {attachment.preview}
                  </pre>
                </ScrollArea>
              </DialogContent>
            </Dialog>
          )}
          <Button
            size="sm"
            variant="ghost"
            onClick={handleDownload}
            className="h-6 w-6 p-0"
            title="Download file"
          >
            <Download className="h-3 w-3" />
          </Button>
        </div>
        
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <span>{formatFileSize(attachment.size)}</span>
          <Badge variant="secondary" className="text-xs">
            {attachment.contentType.split("/")[1]?.toUpperCase() || "FILE"}
          </Badge>
        </div>
        
        {(attachment.description || attachment.autoDescription) && (
          <p className="text-xs text-muted-foreground mt-1 line-clamp-2">
            {attachment.description || attachment.autoDescription}
          </p>
        )}
      </div>
    </div>
  );
}