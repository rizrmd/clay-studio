import { useRef, useEffect, useState } from "react";
import { Send, Square, Paperclip, X, FileText, Info } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

interface FileWithDescription extends File {
  description?: string;
  autoDescription?: string;
}

interface MultimodalInputProps {
  input: string;
  setInput: (input: string) => void;
  handleSubmit: (e: React.FormEvent, files?: FileWithDescription[]) => void;
  isLoading?: boolean;
  isStreaming?: boolean;
  canStop?: boolean;
  stop?: () => void;
  projectId?: string;
  uploadedFiles?: Array<{
    id: string;
    original_name: string;
    description?: string;
    auto_description?: string;
  }>;
  externalFiles?: File[];
  onExternalFilesChange?: (files: File[]) => void;
  shouldFocus?: boolean;
}

export function MultimodalInput({
  input,
  setInput,
  handleSubmit,
  isLoading,
  isStreaming,
  canStop,
  stop,
  projectId,
  uploadedFiles = [],
  externalFiles = [],
  onExternalFilesChange,
  shouldFocus,
}: MultimodalInputProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [selectedFiles, setSelectedFiles] = useState<FileWithDescription[]>([]);
  const [isDragging, setIsDragging] = useState(false);
  const dragCounter = useRef(0);

  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
      textareaRef.current.style.height = `${textareaRef.current.scrollHeight}px`;
    }
  }, [input]);

  // Focus the textarea when shouldFocus is true
  useEffect(() => {
    if (shouldFocus && textareaRef.current) {
      textareaRef.current.focus();
    }
  }, [shouldFocus]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleFormSubmit(e as any);
    }
  };

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files || []);
    setSelectedFiles(prev => [...prev, ...files]);
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  const removeFile = (index: number) => {
    setSelectedFiles(prev => prev.filter((_, i) => i !== index));
  };

  const handleFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const allFiles = [...selectedFiles, ...externalFiles];
    handleSubmit(e, allFiles);
    setSelectedFiles([]);
    onExternalFilesChange?.([]);
  };

  // Drag and drop handlers
  const handleDragEnter = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounter.current++;
    if (e.dataTransfer.items && e.dataTransfer.items.length > 0) {
      setIsDragging(true);
    }
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounter.current--;
    if (dragCounter.current === 0) {
      setIsDragging(false);
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
    dragCounter.current = 0;

    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      const droppedFiles = Array.from(e.dataTransfer.files);
      setSelectedFiles(prev => [...prev, ...droppedFiles]);
      e.dataTransfer.clearData();
    }
  };

  return (
    <form 
      onSubmit={handleFormSubmit} 
      className={cn(
        "relative m-4 transition-all",
        isDragging && "ring-2 ring-primary ring-offset-2 bg-primary/5 rounded-lg"
      )}
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
    >
      {/* Drop zone overlay */}
      {isDragging && (
        <div className="absolute inset-0 z-50 flex items-center justify-center pointer-events-none">
          <div className="text-center">
            <FileText className="h-12 w-12 mx-auto mb-2 text-primary animate-pulse" />
            <p className="text-lg font-semibold text-primary">Drop files here</p>
            <p className="text-sm text-muted-foreground">Release to upload files to this chat</p>
          </div>
        </div>
      )}
      {(selectedFiles.length > 0 || externalFiles.length > 0 || uploadedFiles.length > 0) && (
        <div className="mb-2 space-y-2">
          {/* Show previously uploaded files */}
          {uploadedFiles.length > 0 && (
            <div className="text-xs text-muted-foreground mb-1">Previously uploaded:</div>
          )}
          <div className="flex flex-wrap gap-2">
            {uploadedFiles.map((file) => (
              <TooltipProvider key={file.id}>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Badge variant="outline" className="flex items-center gap-1">
                      <FileText className="h-3 w-3" />
                      <span className="max-w-[150px] truncate text-xs">{file.original_name}</span>
                      {(file.description || file.auto_description) && (
                        <Info className="h-3 w-3 text-muted-foreground" />
                      )}
                    </Badge>
                  </TooltipTrigger>
                  {(file.description || file.auto_description) && (
                    <TooltipContent>
                      <p className="max-w-xs text-xs">
                        {file.description || file.auto_description}
                      </p>
                    </TooltipContent>
                  )}
                </Tooltip>
              </TooltipProvider>
            ))}
          </div>
          
          {/* Show newly selected files */}
          {(selectedFiles.length > 0 || externalFiles.length > 0) && (
            <>
              <div className="text-xs text-muted-foreground">New files:</div>
              <div className="flex flex-wrap gap-2">
                {selectedFiles.map((file, index) => (
                  <Badge key={`selected-${index}`} variant="secondary" className="flex items-center gap-1">
                    <FileText className="h-3 w-3" />
                    <span className="max-w-[150px] truncate text-xs">{file.name}</span>
                    <button
                      type="button"
                      onClick={() => removeFile(index)}
                      className="ml-1 hover:text-destructive"
                    >
                      <X className="h-3 w-3" />
                    </button>
                  </Badge>
                ))}
                {externalFiles.map((file, index) => (
                  <Badge key={`external-${index}`} variant="secondary" className="flex items-center gap-1">
                    <FileText className="h-3 w-3" />
                    <span className="max-w-[150px] truncate text-xs">{file.name}</span>
                    <button
                      type="button"
                      onClick={() => {
                        onExternalFilesChange?.(externalFiles.filter((_, i) => i !== index));
                      }}
                      className="ml-1 hover:text-destructive"
                    >
                      <X className="h-3 w-3" />
                    </button>
                  </Badge>
                ))}
              </div>
            </>
          )}
        </div>
      )}
      <Textarea
        ref={textareaRef}
        value={input}
        autoFocus
        onChange={(e) => setInput(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={isDragging ? "Drop files here..." : "Ask me anything about your data..."}
        className={cn(
          "min-h-[60px] max-h-[200px] resize-none pr-12 bg-white",
          "focus-within:outline-none focus-within:ring-2 focus-within:ring-ring focus-within:ring-offset-2",
          isDragging && "opacity-60"
        )}
        disabled={isLoading || isDragging}
        rows={1}
      />
      <input
        ref={fileInputRef}
        type="file"
        multiple
        onChange={handleFileSelect}
        className="hidden"
        accept="*"
      />
      <div className="absolute bottom-[14px] right-[14px] flex items-center gap-2">
        <Button
          type="button"
          size="icon"
          variant="ghost"
          className="h-8 w-8"
          onClick={() => fileInputRef.current?.click()}
          disabled={isLoading || isStreaming || !projectId}
        >
          <Paperclip className="h-4 w-4" />
          <span className="sr-only">Attach files</span>
        </Button>
        {(isLoading || isStreaming) && canStop ? (
          <Button
            type="button"
            size="icon"
            variant="ghost"
            className="h-8 w-8"
            onClick={stop}
          >
            <Square className="h-4 w-4" />
            <span className="sr-only">Stop generation</span>
          </Button>
        ) : (
          <Button
            type="submit"
            size="icon"
            className="h-8 w-8"
            disabled={!input.trim() || isLoading || isStreaming}
          >
            <Send className="h-4 w-4" />
            <span className="sr-only">Send message</span>
          </Button>
        )}
      </div>
    </form>
  );
}
