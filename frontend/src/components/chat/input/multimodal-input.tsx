import { useRef, useEffect, useState, useCallback } from "react";
import {
  Send,
  Paperclip,
  X,
  FileText,
  Edit2,
  Check,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Input } from "@/components/ui/input";
import { API_BASE_URL } from "@/lib/url";
import { FileBrowser } from "./file-browser";

interface FileWithDescription extends File {
  description?: string;
  autoDescription?: string;
  preview?: string;
}

interface MultimodalInputProps {
  input: string;
  setInput: (input: string) => void;
  handleSubmit: (e: React.FormEvent, files?: FileWithDescription[]) => void;
  isLoading?: boolean;
  isStreaming?: boolean;
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
  className?: string;
}

export function MultimodalInput({
  input,
  setInput,
  handleSubmit,
  isLoading,
  isStreaming,
  projectId,
  uploadedFiles = [],
  externalFiles = [],
  onExternalFilesChange,
  shouldFocus,
  className,
}: MultimodalInputProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [selectedFiles, setSelectedFiles] = useState<FileWithDescription[]>([]);
  const [isDragging, setIsDragging] = useState(false);
  const [showFileBrowser, setShowFileBrowser] = useState(false);
  const [isUploading, setIsUploading] = useState(false);
  const [uploadProgress, setUploadProgress] = useState<{
    [key: string]: number;
  }>({});
  const [uploadAbortController, setUploadAbortController] =
    useState<AbortController | null>(null);
  const [editingDescription, setEditingDescription] = useState<{
    [key: string]: boolean;
  }>({});
  const [fileDescriptions, setFileDescriptions] = useState<{
    [key: string]: string;
  }>({});
  const [filePreviews, setFilePreviews] = useState<{ [key: string]: string }>(
    {}
  );
  const dragCounter = useRef(0);
  const [localInput, setLocalInput] = useState(input);
  const updateTimeout = useRef<NodeJS.Timeout>();

  // Sync local input with prop input when it changes externally
  useEffect(() => {
    setLocalInput(input);
  }, [input]);

  // Debounced update to parent
  const debouncedSetInput = useCallback((value: string) => {
    if (updateTimeout.current) {
      clearTimeout(updateTimeout.current);
    }
    updateTimeout.current = setTimeout(() => {
      setInput(value);
    }, 100);
  }, [setInput]);


  // Focus the textarea when shouldFocus is true
  useEffect(() => {
    if (shouldFocus && textareaRef.current) {
      textareaRef.current.focus();
    }
  }, [shouldFocus]);

  // Store project ID for file browser and messages
  useEffect(() => {
    if (projectId) {
      localStorage.setItem("activeProjectId", projectId);
    }
  }, [projectId]);

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files || []);
    if (files.length === 0) return;

    // Generate previews for image files
    files.forEach((file) => {
      if (file.type.startsWith("image/")) {
        const reader = new FileReader();
        reader.onload = (e) => {
          if (e.target?.result) {
            setFilePreviews((prev) => ({
              ...prev,
              [file.name]: e.target!.result as string,
            }));
          }
        };
        reader.readAsDataURL(file);
      }
    });

    // Start uploading files immediately
    await uploadFiles(files);

    if (fileInputRef.current) {
      fileInputRef.current.value = "";
    }
  };

  const uploadFiles = async (files: File[]) => {
    const clientId = localStorage.getItem("activeClientId");
    if (!clientId || !projectId) {
      // No active client or project
      return;
    }

    setIsUploading(true);
    const abortController = new AbortController();
    setUploadAbortController(abortController);

    const uploadedFilesList: FileWithDescription[] = [];

    try {
      for (const file of files) {
        // Track upload progress for this file
        setUploadProgress((prev) => ({ ...prev, [file.name]: 0 }));

        const formData = new FormData();
        formData.append("file", file);

        const xhr = new XMLHttpRequest();

        // Track upload progress
        xhr.upload.onprogress = (event) => {
          if (event.lengthComputable) {
            const percentComplete = (event.loaded / event.total) * 100;
            setUploadProgress((prev) => ({
              ...prev,
              [file.name]: percentComplete,
            }));
          }
        };

        // Use promise to handle the upload
        await new Promise((resolve, reject) => {
          xhr.onload = () => {
            if (xhr.status === 200) {
              const result = JSON.parse(xhr.responseText);
              const uploadedFile = file as FileWithDescription;
              uploadedFile.description =
                result.description || fileDescriptions[file.name];
              uploadedFile.autoDescription = result.auto_description;
              uploadedFile.preview = filePreviews[file.name];
              uploadedFilesList.push(uploadedFile);

              // Store the description
              if (result.description || fileDescriptions[file.name]) {
                setFileDescriptions((prev) => ({
                  ...prev,
                  [file.name]:
                    result.description || fileDescriptions[file.name] || "",
                }));
              }

              resolve(result);
            } else {
              reject(new Error(`Failed to upload ${file.name}`));
            }
          };

          xhr.onerror = () =>
            reject(new Error(`Failed to upload ${file.name}`));
          xhr.onabort = () =>
            reject(new Error(`Upload cancelled for ${file.name}`));

          xhr.open(
            "POST",
            `${API_BASE_URL}/upload?client_id=${clientId}&project_id=${projectId}`
          );
          xhr.withCredentials = true;
          xhr.send(formData);

          // Store the xhr object so we can abort if needed
          abortController.signal.addEventListener("abort", () => {
            xhr.abort();
          });
        });

        // Remove progress tracking for completed file
        setUploadProgress((prev) => {
          const newProgress = { ...prev };
          delete newProgress[file.name];
          return newProgress;
        });
      }

      // Add uploaded files to selected files
      setSelectedFiles((prev) => [...prev, ...uploadedFilesList]);
    } catch (error) {
      // Upload error
      // Clear all progress on error
      setUploadProgress({});
    } finally {
      setIsUploading(false);
      setUploadAbortController(null);
    }
  };

  const cancelUpload = () => {
    if (uploadAbortController) {
      uploadAbortController.abort();
      setUploadAbortController(null);
      setIsUploading(false);
      setUploadProgress({});
    }
  };

  const removeFile = (index: number) => {
    const file = selectedFiles[index];
    if (file) {
      // Clean up preview and description
      setFilePreviews((prev) => {
        const newPreviews = { ...prev };
        delete newPreviews[file.name];
        return newPreviews;
      });
      setFileDescriptions((prev) => {
        const newDescriptions = { ...prev };
        delete newDescriptions[file.name];
        return newDescriptions;
      });
      setEditingDescription((prev) => {
        const newEditing = { ...prev };
        delete newEditing[file.name];
        return newEditing;
      });
    }
    setSelectedFiles((prev) => prev.filter((_, i) => i !== index));
  };

  const toggleEditDescription = (fileName: string) => {
    setEditingDescription((prev) => ({ ...prev, [fileName]: !prev[fileName] }));
  };

  const updateFileDescription = (fileName: string, description: string) => {
    setFileDescriptions((prev) => ({ ...prev, [fileName]: description }));
    // Update the file object if it exists
    setSelectedFiles((prev) =>
      prev.map((file) => {
        if (file.name === fileName) {
          const updatedFile = file as FileWithDescription;
          updatedFile.description = description;
          return updatedFile;
        }
        return file;
      })
    );
  };

  const handleFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    // Don't submit if files are uploading or no input
    if (isUploading || !localInput.trim()) {
      return;
    }

    // Clear any pending debounced updates
    if (updateTimeout.current) {
      clearTimeout(updateTimeout.current);
    }

    // Ensure parent has the latest input value
    setInput(localInput);

    const allFiles = [...selectedFiles, ...externalFiles];
    handleSubmit(e, allFiles);
    
    // Clear local input immediately
    setLocalInput("");
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

      // Generate previews for image files
      droppedFiles.forEach((file) => {
        if (file.type.startsWith("image/")) {
          const reader = new FileReader();
          reader.onload = (e) => {
            if (e.target?.result) {
              setFilePreviews((prev) => ({
                ...prev,
                [file.name]: e.target!.result as string,
              }));
            }
          };
          reader.readAsDataURL(file);
        }
      });

      // Start uploading dropped files immediately
      uploadFiles(droppedFiles);
      e.dataTransfer.clearData();
    }
  };

  return (
    <form
      onSubmit={handleFormSubmit}
      className={cn(
        "relative m-3 transition-all",
        isDragging &&
          "ring-2 ring-primary ring-offset-2 bg-primary/5 rounded-lg",
        className
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
            <p className="text-lg font-semibold text-primary">
              Drop files here
            </p>
            <p className="text-sm text-muted-foreground">
              Release to upload files to this chat
            </p>
          </div>
        </div>
      )}
      {/* Upload progress indicators */}
      {isUploading && Object.keys(uploadProgress).length > 0 && (
        <div className="mb-2 space-y-2">
          <div className="flex items-center justify-between">
            <div className="text-xs text-muted-foreground">
              Uploading files...
            </div>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={cancelUpload}
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
      )}

      {(selectedFiles.length > 0 ||
        externalFiles.length > 0 ||
        uploadedFiles.length > 0) && (
        <div className="mb-2 space-y-2">
          {/* Show newly selected files */}
          {(selectedFiles.length > 0 || externalFiles.length > 0) && (
            <>
              <div className="text-xs text-muted-foreground">
                Attached files:
              </div>
              <div className="space-y-2">
                {selectedFiles.map((file, index) => {
                  const isImage = file.type.startsWith("image/");
                  const preview = filePreviews[file.name];
                  const isEditingDesc = editingDescription[file.name];
                  const description =
                    fileDescriptions[file.name] ||
                    (file as FileWithDescription).description ||
                    "";

                  return (
                    <div
                      key={`selected-${index}`}
                      className="border rounded-lg p-2 space-y-2"
                    >
                      <div className="flex items-start gap-2">
                        {/* Preview or icon */}
                        {isImage && preview ? (
                          <img
                            src={preview}
                            alt={file.name}
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
                              {file.name}
                            </span>
                            <button
                              type="button"
                              onClick={() => removeFile(index)}
                              className="ml-2 p-1 hover:text-destructive"
                              disabled={isUploading}
                            >
                              <X className="h-4 w-4" />
                            </button>
                          </div>

                          {/* Description */}
                          <div className="mt-1">
                            {isEditingDesc ? (
                              <div className="flex gap-1">
                                <Input
                                  type="text"
                                  placeholder="Add a description..."
                                  value={description}
                                  onChange={(e) =>
                                    updateFileDescription(
                                      file.name,
                                      e.target.value
                                    )
                                  }
                                  className="h-7 text-xs"
                                  autoFocus
                                  onKeyDown={(e) => {
                                    if (e.key === "Enter") {
                                      e.preventDefault();
                                      toggleEditDescription(file.name);
                                    }
                                  }}
                                />
                                <Button
                                  type="button"
                                  size="icon"
                                  variant="ghost"
                                  className="h-7 w-7"
                                  onClick={() =>
                                    toggleEditDescription(file.name)
                                  }
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
                                  onClick={() =>
                                    toggleEditDescription(file.name)
                                  }
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
                })}
                {externalFiles.map((file, index) => {
                  const isImage = file.type.startsWith("image/");
                  const preview = filePreviews[file.name];
                  const isEditingDesc =
                    editingDescription[`external-${file.name}`];
                  const description =
                    fileDescriptions[`external-${file.name}`] ||
                    (file as FileWithDescription).description ||
                    "";

                  return (
                    <div
                      key={`external-${index}`}
                      className="border rounded-lg p-2 space-y-2"
                    >
                      <div className="flex items-start gap-2">
                        {/* Preview or icon */}
                        {isImage && preview ? (
                          <img
                            src={preview}
                            alt={file.name}
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
                              {file.name}
                            </span>
                            <button
                              type="button"
                              onClick={() => {
                                onExternalFilesChange?.(
                                  externalFiles.filter((_, i) => i !== index)
                                );
                                // Clean up preview and description
                                setFilePreviews((prev) => {
                                  const newPreviews = { ...prev };
                                  delete newPreviews[file.name];
                                  return newPreviews;
                                });
                                setFileDescriptions((prev) => {
                                  const newDescriptions = { ...prev };
                                  delete newDescriptions[
                                    `external-${file.name}`
                                  ];
                                  return newDescriptions;
                                });
                              }}
                              className="ml-2 p-1 hover:text-destructive"
                            >
                              <X className="h-4 w-4" />
                            </button>
                          </div>

                          {/* Description */}
                          <div className="mt-1">
                            {isEditingDesc ? (
                              <div className="flex gap-1">
                                <Input
                                  type="text"
                                  placeholder="Add a description..."
                                  value={description}
                                  onChange={(e) =>
                                    setFileDescriptions((prev) => ({
                                      ...prev,
                                      [`external-${file.name}`]: e.target.value,
                                    }))
                                  }
                                  className="h-7 text-xs"
                                  autoFocus
                                  onKeyDown={(e) => {
                                    if (e.key === "Enter") {
                                      e.preventDefault();
                                      setEditingDescription((prev) => ({
                                        ...prev,
                                        [`external-${file.name}`]: false,
                                      }));
                                    }
                                  }}
                                />
                                <Button
                                  type="button"
                                  size="icon"
                                  variant="ghost"
                                  className="h-7 w-7"
                                  onClick={() =>
                                    setEditingDescription((prev) => ({
                                      ...prev,
                                      [`external-${file.name}`]: false,
                                    }))
                                  }
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
                                  onClick={() =>
                                    setEditingDescription((prev) => ({
                                      ...prev,
                                      [`external-${file.name}`]: true,
                                    }))
                                  }
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
                })}
              </div>
            </>
          )}
        </div>
      )}
      <Textarea
        ref={textareaRef}
        value={localInput}
        autoFocus
        onChange={(e) => {
          setLocalInput(e.target.value);
          debouncedSetInput(e.target.value);
        }}
        onEnterSubmit={(e) => handleFormSubmit(e as any)}
        placeholder={
          isLoading
            ? "Continue typing"
            : isDragging
            ? "Drop files here..."
            : isUploading
            ? "Uploading files..."
            : "Type to chat..."
        }
        placeholderSecondary={
          isLoading
            ? "while the ai is thinking..."
            : isDragging || isUploading
            ? undefined
            : "Enter to send, Shift+Enter for new line"
        }
        className={cn(
          "min-h-[60px] max-h-[200px] resize-none pr-12 bg-background"
        )}
        disabled={isDragging || isUploading}
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
      <div className="absolute bottom-0 top-0 right-[14px] flex items-center gap-1">
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                size="icon"
                variant="ghost"
                className={cn(
                  "h-8 w-8",
                  (isLoading || isStreaming || !projectId) && "hidden"
                )}
                onClick={() => setShowFileBrowser(true)}
                disabled={isLoading || isStreaming || !projectId}
              >
                <Paperclip className="h-4 w-4" />
                <span className="sr-only">Attach files</span>
              </Button>
            </TooltipTrigger>
            <TooltipContent>Upload files</TooltipContent>
          </Tooltip>
        </TooltipProvider>
        <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant={
                    !localInput.trim() || isUploading
                      ? "outline"
                      : isLoading || isStreaming
                      ? "secondary"
                      : "default"
                  }
                  type="submit"
                  disabled={
                    !localInput.trim() || isUploading
                  }
                >
                  <Send className="h-4 w-4" />
                  <span className="sr-only">Send message</span>
                </Button>
              </TooltipTrigger>
              {isUploading && (
                <TooltipContent>Wait for files to upload</TooltipContent>
              )}
            </Tooltip>
          </TooltipProvider>
      </div>

      {projectId && (
        <FileBrowser
          open={showFileBrowser}
          onOpenChange={setShowFileBrowser}
          projectId={projectId}
          onFilesSelected={(files) => {
            // Convert the selected existing files to File-like objects for consistency
            const convertedFiles = files.map((f) => {
              // Create a File-like object with the necessary properties
              const file = new File([], f.original_name, {
                type: f.mime_type || "application/octet-stream",
              });
              // Add our custom properties
              Object.defineProperties(file, {
                description: { value: f.description, writable: true },
                autoDescription: { value: f.auto_description, writable: true },
                isExisting: { value: true, writable: false },
                fileId: { value: f.id, writable: false },
                filePath: { value: f.file_path, writable: false },
                preview: { value: null, writable: true },
              });
              return file as FileWithDescription & {
                isExisting: boolean;
                fileId: string;
                filePath: string;
              };
            });

            setSelectedFiles((prev) => [...prev, ...convertedFiles]);

            // Store descriptions and generate previews for selected files
            convertedFiles.forEach((file) => {
              // Store the description
              const description =
                (file as any).description ||
                (file as any).autoDescription ||
                "";
              if (description) {
                setFileDescriptions((prev) => ({
                  ...prev,
                  [file.name]: description,
                }));
              }

              // Generate preview for image files
              if (file.type.startsWith("image/")) {
                const clientId = localStorage.getItem("activeClientId");
                if (clientId && projectId) {
                  const fileName = (file as any).filePath.split("/").pop();
                  const previewUrl = `${API_BASE_URL}/uploads/${clientId}/${projectId}/${fileName}`;
                  setFilePreviews((prev) => ({
                    ...prev,
                    [file.name]: previewUrl,
                  }));
                }
              }
            });
          }}
        />
      )}
    </form>
  );
}
