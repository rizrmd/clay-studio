import { useRef, useEffect } from "react";
import { useSnapshot } from "valtio";
import { multimodalInputActions } from "@/store/multimodal-input-store";
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
import { API_BASE_URL } from "@/lib/utils/url";
import { FileBrowser } from "./file-browser";
import { inputStore, inputActions } from "@/store/input-store";
import { conversationStore } from "@/store/chat/conversation-store";

interface ComponentFileWithDescription extends File {
  description?: string;
  autoDescription?: string;
  preview?: string;
}

interface StoreFileWithDescription {
  file: File;
  description?: string;
  id: string;
}

interface MultimodalInputProps {
  input: string;
  setInput: (input: string) => void;
  handleSubmit: (e: React.FormEvent, message: string, files?: ComponentFileWithDescription[]) => void;
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
  isSubscribed?: boolean;
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
  isSubscribed = true,
}: MultimodalInputProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const inputSnapshot = useSnapshot(inputStore, { sync: true });
  const dragCounter = useRef(0);

  // Get active conversation ID - fallback to 'new' if not set
  const activeConversationId = conversationStore.activeConversationId || 'new';

  // Sync local input with prop input when it changes externally
  useEffect(() => {
    if (activeConversationId) {
      multimodalInputActions.setLocalInput(activeConversationId, input);
    }
  }, [input, activeConversationId]);



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

  // Debug logging for input state
  useEffect(() => {
    const isDisabled = inputSnapshot.isDragging || inputSnapshot.isUploading;
    if (isDisabled) {
      console.debug('Chat input disabled:', {
        isDragging: inputSnapshot.isDragging,
        isUploading: inputSnapshot.isUploading,
        conversationId: activeConversationId
      });
    }
  }, [inputSnapshot.isDragging, inputSnapshot.isUploading, activeConversationId]);

  // Manual reset function for development/debugging
  const resetInputState = () => {
    inputActions.setDragging(false);
    inputActions.setUploading(false);
    inputActions.clearUploadProgress();
    dragCounter.current = 0;
    console.log('Manually reset input state');
  };

  // Expose reset function to window for debugging (only in development)
  useEffect(() => {
    if (process.env.NODE_ENV === 'development') {
      (window as any).resetChatInput = resetInputState;
    }
  }, []);

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files || []);
    if (files.length === 0) return;

    // Generate previews for image files
    files.forEach((file) => {
      if (file.type.startsWith("image/")) {
        const reader = new FileReader();
        reader.onload = (e) => {
          if (e.target?.result) {
            multimodalInputActions.setFilePreview(activeConversationId, file.name, e.target!.result as string);
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

    inputActions.setUploading(true);
    const abortController = new AbortController();
    multimodalInputActions.setUploadAbortController(activeConversationId, abortController);

    const uploadedFilesList: ComponentFileWithDescription[] = [];

    // Ensure upload state is reset even if something goes wrong
    const cleanup = () => {
      inputActions.setUploading(false);
      multimodalInputActions.setUploadAbortController(activeConversationId, null);
      inputActions.clearUploadProgress();
    };

    try {
      for (const file of files) {
        // Track upload progress for this file
        inputActions.updateUploadProgress(file?.name, 0);

        const formData = new FormData();
        formData.append("file", file);

        const xhr = new XMLHttpRequest();

        // Track upload progress
        xhr.upload.onprogress = (event) => {
          if (event.lengthComputable) {
            const percentComplete = (event.loaded / event.total) * 100;
            inputActions.updateUploadProgress(file?.name, Math.round(percentComplete));
          }
        };

        // Use promise to handle the upload
        await new Promise((resolve, reject) => {
          xhr.onload = () => {
            if (xhr.status === 200) {
              const result = JSON.parse(xhr.responseText);
              const uploadedFile: ComponentFileWithDescription = {
                ...file,
                description: result.description || multimodalInputActions.getFileDescription(activeConversationId, file.name),
                autoDescription: result.auto_description,
                preview: multimodalInputActions.getFilePreview(activeConversationId, file.name)
              };
              uploadedFilesList.push(uploadedFile);

              // Store the description
              if (result.description || multimodalInputActions.getFileDescription(activeConversationId, file.name)) {
                multimodalInputActions.setFileDescription(activeConversationId, file.name,
                  result.description || multimodalInputActions.getFileDescription(activeConversationId, file.name) || "");
              }

              resolve(result);
            } else {
              reject(new Error(`Failed to upload ${file?.name}`));
            }
          };

          xhr.onerror = () =>
            reject(new Error(`Failed to upload ${file?.name}`));
          xhr.onabort = () =>
            reject(new Error(`Upload cancelled for ${file?.name}`));

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

        // Progress is automatically updated via updateUploadProgress
      }

      // Add uploaded files to selected files
      uploadedFilesList.forEach(file => {
        const fileWithDesc: StoreFileWithDescription = {
          file,
          description: file.description || '',
          id: Date.now().toString() + Math.random().toString(36).substring(2, 11)
        };
        inputActions.addSelectedFile(fileWithDesc);
      });
    } catch (error) {
      // Upload error
      console.error('Upload failed:', error);
      // Clear all progress on error
      inputActions.clearUploadProgress();
    } finally {
      cleanup();
    }
  };

  const cancelUpload = () => {
    const abortController = multimodalInputActions.getUploadAbortController(activeConversationId);
    if (abortController) {
      abortController.abort();
    }
    // Always reset state, even if no abort controller exists
    multimodalInputActions.setUploadAbortController(activeConversationId, null);
    inputActions.setUploading(false);
    inputActions.clearUploadProgress();
  };

  const removeFile = (index: number) => {
    const file = inputSnapshot.selectedFiles[index];
    if (file) {
      // Clean up preview and description
      multimodalInputActions.clearFilePreview(activeConversationId, file.file.name);
      multimodalInputActions.clearFileDescription(activeConversationId, file.file.name);
      multimodalInputActions.setEditingDescription(activeConversationId, file.file.name, false);
    }
    inputActions.removeSelectedFile(file?.id || index.toString());
  };

  const toggleEditDescription = (fileName: string) => {
    const currentEditing = multimodalInputActions.getEditingDescription(activeConversationId, fileName);
    multimodalInputActions.setEditingDescription(activeConversationId, fileName, !currentEditing);
  };

  const updateFileDescription = (fileName: string, description: string) => {
    multimodalInputActions.setFileDescription(activeConversationId, fileName, description);
    // Update the file object if it exists - need to create new object due to valtio proxy
    const fileIndex = inputSnapshot.selectedFiles.findIndex(f => f.file?.name === fileName);
    if (fileIndex !== -1) {
      const updatedFiles = [...inputSnapshot.selectedFiles];
      updatedFiles[fileIndex] = {
        ...updatedFiles[fileIndex],
        description
      };
      inputActions.setSelectedFiles(updatedFiles);
    }
  };

  const handleFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    // Don't submit if files are uploading or no input
    if (inputSnapshot.isUploading || !input.trim()) {
      return;
    }

    const allFiles = [...inputSnapshot.selectedFiles.map(f => f.file), ...externalFiles];

    // Call handleSubmit with the message content and files
    handleSubmit(e, input.trim(), allFiles);

    // Clear local input and files after submit
    multimodalInputActions.setLocalInput(activeConversationId, "");
    inputActions.clearSelectedFiles();
    onExternalFilesChange?.([]);
  };

  // Drag and drop handlers
  const handleDragEnter = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounter.current++;
    if (e.dataTransfer.items && e.dataTransfer.items.length > 0) {
      inputActions.setDragging(true);
    }
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounter.current = Math.max(0, dragCounter.current - 1);
    if (dragCounter.current === 0) {
      inputActions.setDragging(false);
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    inputActions.setDragging(false);
    dragCounter.current = 0;

    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      const droppedFiles = Array.from(e.dataTransfer.files);

      // Generate previews for image files
      droppedFiles.forEach((file) => {
        if (file.type.startsWith("image/")) {
          const reader = new FileReader();
          reader.onload = (e) => {
            if (e.target?.result) {
              multimodalInputActions.setFilePreview(activeConversationId, file.name, e.target!.result as string);
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

  // Add cleanup effect for drag state
  useEffect(() => {
    const handleGlobalDragEnd = () => {
      inputActions.setDragging(false);
      dragCounter.current = 0;
    };

    // Reset drag state on window blur/focus to handle edge cases
    const handleWindowBlur = () => {
      inputActions.setDragging(false);
      dragCounter.current = 0;
    };

    window.addEventListener('blur', handleWindowBlur);
    window.addEventListener('dragend', handleGlobalDragEnd);
    document.addEventListener('dragend', handleGlobalDragEnd);

    return () => {
      window.removeEventListener('blur', handleWindowBlur);
      window.removeEventListener('dragend', handleGlobalDragEnd);
      document.removeEventListener('dragend', handleGlobalDragEnd);
    };
  }, []);

  // If not subscribed to WebSocket, show connecting message
  if (!isSubscribed) {
    return (
      <div className={cn("relative m-3", className)}>
        <div className="bg-muted/50 rounded-lg p-4 border border-border min-h-[60px] flex items-center justify-center">
          <div className="flex items-center gap-2">
            <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-primary"></div>
            <div className="text-sm text-muted-foreground">
              Connecting to conversation...
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <form
      onSubmit={handleFormSubmit}
      className={cn(
        "relative m-3 transition-all",
        inputSnapshot.isDragging &&
          "ring-2 ring-primary ring-offset-2 bg-primary/5 rounded-lg",
        className
      )}
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
    >
      {/* Drop zone overlay */}
      {inputSnapshot.isDragging && (
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
      {inputSnapshot.isUploading && Object.keys(inputSnapshot.uploadProgress).length > 0 && (
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
          {Object.entries(inputSnapshot.uploadProgress).map(([fileName, progress]) => (
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

      {(inputSnapshot.selectedFiles.length > 0 ||
        externalFiles.length > 0 ||
        uploadedFiles.length > 0) && (
        <div className="mb-2 space-y-2">
          {/* Show newly selected files */}
          {(inputSnapshot.selectedFiles.length > 0 || externalFiles.length > 0) && (
            <>
              <div className="text-xs text-muted-foreground">
                Attached files:
              </div>
              <div className="space-y-2">
                {inputSnapshot.selectedFiles.map((fileData, index) => {
                  const file = fileData.file;
                  const isImage = file.type.startsWith("image/");
                  const preview = multimodalInputActions.getFilePreview(activeConversationId, file.name);
                  const isEditingDesc = multimodalInputActions.getEditingDescription(activeConversationId, file.name);
                  const description = multimodalInputActions.getFileDescription(activeConversationId, file.name) || fileData.description || "";

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
                              onClick={() => removeFile(index)}
                              className="ml-2 p-1 hover:text-destructive"
                              disabled={inputSnapshot.isUploading}
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
                                      file?.name,
                                      e.target.value
                                    )
                                  }
                                  className="h-7 text-xs"
                                  autoFocus
                                  onKeyDown={(e) => {
                                    if (e.key === "Enter") {
                                      e.preventDefault();
                                      toggleEditDescription(file?.name);
                                    }
                                  }}
                                />
                                <Button
                                  type="button"
                                  size="icon"
                                  variant="ghost"
                                  className="h-7 w-7"
                                  onClick={() =>
                                    toggleEditDescription(file?.name)
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
                                    toggleEditDescription(file?.name)
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
                  const preview = multimodalInputActions.getFilePreview(activeConversationId, file.name);
                  const isEditingDesc =
                    multimodalInputActions.getEditingDescription(activeConversationId, `external-${file.name}`);
                  const description =
                    multimodalInputActions.getFileDescription(activeConversationId, `external-${file.name}`) ||
                    (file as ComponentFileWithDescription).description ||
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
                                onClick={() => {
                                  onExternalFilesChange?.(
                                    externalFiles.filter((_, i) => i !== index)
                                  );
                                  // Clean up preview and description
                                  multimodalInputActions.clearFilePreview(activeConversationId, file.name);
                                  multimodalInputActions.clearFileDescription(activeConversationId, `external-${file.name}`);
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
                                    multimodalInputActions.setFileDescription(activeConversationId, `external-${file.name}`, e.target.value)
                                  }
                                  className="h-7 text-xs"
                                  autoFocus
                                  onKeyDown={(e) => {
                                    if (e.key === "Enter") {
                                      e.preventDefault();
                                      multimodalInputActions.setEditingDescription(activeConversationId, `external-${file.name}`, false);
                                    }
                                  }}
                                />
                                <Button
                                  type="button"
                                  size="icon"
                                  variant="ghost"
                                  className="h-7 w-7"
                                  onClick={() =>
                                    multimodalInputActions.setEditingDescription(activeConversationId, `external-${file.name}`, false)
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
                                    multimodalInputActions.setEditingDescription(activeConversationId, `external-${file.name}`, true)
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
        value={input}
        autoFocus
        onChange={(e) => {
          setInput(e.target.value);
          multimodalInputActions.setLocalInput(activeConversationId, e.target.value);
        }}
        onEnterSubmit={(e) => handleFormSubmit(e as any)}
        placeholder={
          isLoading
            ? "Continue typing"
            : inputSnapshot.isDragging
            ? "Drop files here..."
            : inputSnapshot.isUploading
            ? "Uploading files..."
            : "Type to chat..."
        }
        placeholderSecondary={
          isLoading
            ? "while the ai is thinking..."
            : inputSnapshot.isDragging || inputSnapshot.isUploading
            ? undefined
            : "Enter to send, Shift+Enter for new line"
        }
        className={cn(
          "min-h-[60px] max-h-[200px] resize-none pr-12 bg-background"
        )}
        disabled={inputSnapshot.isDragging || inputSnapshot.isUploading}
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
                  (isLoading || isStreaming || !projectId) && "invisible"
                )}
                onClick={() => inputActions.setShowFileBrowser(true)}
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
                  size="icon"
                  variant={
                    !input.trim() || inputSnapshot.isUploading
                      ? "outline"
                      : isLoading || isStreaming
                      ? "secondary"
                      : "default"
                  }
                  type="submit"
                  disabled={
                    !input.trim() || inputSnapshot.isUploading || isLoading || isStreaming
                  }
                  className="h-8 w-8"
                >
                  <Send className="h-4 w-4" />
                  <span className="sr-only">Send message</span>
                </Button>
              </TooltipTrigger>
              {inputSnapshot.isUploading && (
                <TooltipContent>Wait for files to upload</TooltipContent>
              )}
            </Tooltip>
          </TooltipProvider>
      </div>

      {projectId && (
        <FileBrowser
          open={inputSnapshot.showFileBrowser}
          onOpenChange={inputActions.setShowFileBrowser}
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
              return file as ComponentFileWithDescription & {
                isExisting: boolean;
                fileId: string;
                filePath: string;
              };
            });

            convertedFiles.forEach(file => {
              const storeFile: StoreFileWithDescription = {
                file,
                description: (file as any).description || '',
                id: (file as any).fileId || Date.now().toString() + Math.random().toString(36).substring(2, 11)
              };
              inputActions.addSelectedFile(storeFile);
            });

            // Store descriptions and generate previews for selected files
            convertedFiles.forEach((file) => {
              // Store the description
              const description =
                (file as any).description ||
                (file as any).autoDescription ||
                "";
              if (description) {
                multimodalInputActions.setFileDescription(activeConversationId, file.name, description);
              }

              // Generate preview for image files
              if (file.type.startsWith("image/")) {
                const clientId = localStorage.getItem("activeClientId");
                if (clientId && projectId) {
                  const fileName = (file as any).filePath.split("/").pop();
                  const previewUrl = `${API_BASE_URL}/uploads/${clientId}/${projectId}/${fileName}`;
                  multimodalInputActions.setFilePreview(activeConversationId, file.name, previewUrl);
                }
              }
            });
          }}
        />
      )}
    </form>
  );
}
