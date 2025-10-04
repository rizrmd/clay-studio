import { useRef, useEffect, useState } from "react";
import { useSnapshot } from "valtio";

import { Send, Paperclip } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { API_BASE_URL } from "@/lib/utils/url";
import { FileBrowser } from "./file-browser";

import { FileUploadProgress } from "./file-upload-progress";
import { DragDropOverlay } from "./drag-drop-overlay";
import { AttachedFileItem } from "./attached-file-item";
import { useFileUpload } from "./use-file-upload";
import { useDragAndDrop } from "./use-drag-and-drop";
import {
  inputStore,
  inputActions,
  multimodalInputActions,
  type StoreFileWithDescription,
} from "@/lib/store/chat/input-store";
import { chatStore } from "@/lib/store/chat/chat-store";

interface ComponentFileWithDescription extends File {
  description?: string;
  autoDescription?: string;
  preview?: string;
}


interface ChatInputProps {
  input: string;
  setInput: (input: string) => void;
  handleSubmit: (
    e: React.FormEvent,
    message: string,
    files?: ComponentFileWithDescription[]
  ) => void;
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
  conversationId?: string;
}

export function ChatInput({
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
  conversationId,
}: ChatInputProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const inputSnapshot = useSnapshot(inputStore, { sync: true });
  const chatSnapshot = useSnapshot(chatStore, { sync: true });
  
  // State for message history navigation
  const [messageHistoryIndex, setMessageHistoryIndex] = useState(-1);
  const [originalInput, setOriginalInput] = useState("");

  const activeConversationId = chatStore.conversation_id || "new";
  const { uploadFiles, cancelUpload, generatePreview } = useFileUpload(
    activeConversationId,
    projectId
  );

  // Handle dropped files
  const handleFilesDropped = (files: File[]) => {
    files.forEach(generatePreview);
    uploadFiles(files);
  };

  const { handleDragEnter, handleDragLeave, handleDragOver, handleDrop } =
    useDragAndDrop(handleFilesDropped);

  // Sync local input with prop input when it changes externally
  useEffect(() => {
    if (activeConversationId) {
      multimodalInputActions.setLocalInput(activeConversationId, input);
    }
  }, [input, activeConversationId]);

  // Focus the textarea when navigating to a new conversation
  useEffect(() => {
    if (conversationId === 'new' && textareaRef.current) {
      textareaRef.current.focus();
      // Move cursor to end of any prefilled text
      setTimeout(() => {
        if (textareaRef.current) {
          const length = textareaRef.current.value.length;
          textareaRef.current.selectionStart = length;
          textareaRef.current.selectionEnd = length;
        }
      }, 0);
    }
  }, [conversationId]);

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
      console.debug("Chat input disabled:", {
        isDragging: inputSnapshot.isDragging,
        isUploading: inputSnapshot.isUploading,
        conversationId: activeConversationId,
      });
    }
  }, [
    inputSnapshot.isDragging,
    inputSnapshot.isUploading,
    activeConversationId,
  ]);

  // Manual reset function for development/debugging
  const resetInputState = () => {
    inputActions.setDragging(false);
    inputActions.setUploading(false);
    inputActions.clearUploadProgress();
  };

  // Expose reset function to window for debugging (only in development)
  useEffect(() => {
    if (process.env.NODE_ENV === "development") {
      (window as any).resetChatInput = resetInputState;
    }
  }, []);

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files || []);
    if (files.length === 0) return;

    files.forEach(generatePreview);
    await uploadFiles(files);

    if (fileInputRef.current) {
      fileInputRef.current.value = "";
    }
  };

  const removeFile = (index: number) => {
    const file = inputSnapshot.selectedFiles[index];
    if (file) {
      multimodalInputActions.clearFilePreview(
        activeConversationId,
        file.file.name
      );
      multimodalInputActions.clearFileDescription(
        activeConversationId,
        file.file.name
      );
      multimodalInputActions.setEditingDescription(
        activeConversationId,
        file.file.name,
        false
      );
    }
    inputActions.removeSelectedFile(file?.id || index.toString());
  };

  const toggleEditDescription = (fileName: string) => {
    const currentEditing = multimodalInputActions.getEditingDescription(
      activeConversationId,
      fileName
    );
    multimodalInputActions.setEditingDescription(
      activeConversationId,
      fileName,
      !currentEditing
    );
  };

  const updateFileDescription = (fileName: string, description: string) => {
    multimodalInputActions.setFileDescription(
      activeConversationId,
      fileName,
      description
    );
    const fileIndex = inputSnapshot.selectedFiles.findIndex(
      (f) => f.file?.name === fileName
    );
    if (fileIndex !== -1) {
      const updatedFiles = [...inputSnapshot.selectedFiles];
      updatedFiles[fileIndex] = {
        ...updatedFiles[fileIndex],
        description,
      };
      inputActions.setSelectedFiles(updatedFiles);
    }
  };

  const handleFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    if (inputSnapshot.isUploading || !input.trim()) {
      return;
    }

    const allFiles = [
      ...inputSnapshot.selectedFiles.map((f) => f.file),
      ...externalFiles,
    ];
    handleSubmit(e, input.trim(), allFiles);

    multimodalInputActions.setLocalInput(activeConversationId, "");
    inputActions.clearSelectedFiles();
    onExternalFilesChange?.([]);
    
    // Reset message history navigation
    setMessageHistoryIndex(-1);
    setOriginalInput("");
  };

  const getUserMessages = () => {
    const currentConversationId = chatSnapshot.conversation_id || conversationId;
    if (!currentConversationId || currentConversationId === "new") {
      return [];
    }

    const conversation = chatSnapshot.map[currentConversationId];
    if (!conversation || !conversation.messages) {
      return [];
    }

    return conversation.messages
      .filter(msg => msg.role === "user")
      .reverse(); // Most recent first
  };

  const handleUpArrowPressed = () => {
    const userMessages = getUserMessages();
    if (userMessages.length === 0) return;

    // Store original input when starting navigation
    if (messageHistoryIndex === -1) {
      setOriginalInput(input);
    }

    const nextIndex = Math.min(messageHistoryIndex + 1, userMessages.length - 1);
    setMessageHistoryIndex(nextIndex);
    
    const selectedMessage = userMessages[nextIndex].content;
    setInput(selectedMessage);
    multimodalInputActions.setLocalInput(activeConversationId, selectedMessage);
  };

  const handleDownArrowPressed = () => {
    if (messageHistoryIndex === -1) return;

    const userMessages = getUserMessages();
    
    if (messageHistoryIndex === 0) {
      // Go back to original input
      setMessageHistoryIndex(-1);
      setInput(originalInput);
      multimodalInputActions.setLocalInput(activeConversationId, originalInput);
    } else {
      const nextIndex = messageHistoryIndex - 1;
      setMessageHistoryIndex(nextIndex);
      
      const selectedMessage = userMessages[nextIndex].content;
      setInput(selectedMessage);
      multimodalInputActions.setLocalInput(activeConversationId, selectedMessage);
    }
  };

  // Only show connecting message for real conversations that aren't subscribed
  if (!isSubscribed && conversationId !== "new") {
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
      <DragDropOverlay isDragging={inputSnapshot.isDragging} />

      {inputSnapshot.isUploading && (
        <FileUploadProgress
          uploadProgress={inputSnapshot.uploadProgress}
          onCancel={cancelUpload}
        />
      )}

      {(inputSnapshot.selectedFiles.length > 0 ||
        externalFiles.length > 0 ||
        uploadedFiles.length > 0) && (
        <div className="mb-2 space-y-2">
          {(inputSnapshot.selectedFiles.length > 0 ||
            externalFiles.length > 0) && (
            <>
              <div className="text-xs text-muted-foreground">
                Attached files:
              </div>
              <div className="space-y-2">
                {inputSnapshot.selectedFiles.map((fileData: any, index: number) => {
                  const file = fileData.file;
                  const preview = multimodalInputActions.getFilePreview(
                    activeConversationId,
                    file.name
                  );
                  const isEditingDesc =
                    multimodalInputActions.getEditingDescription(
                      activeConversationId,
                      file.name
                    );
                  const description =
                    multimodalInputActions.getFileDescription(
                      activeConversationId,
                      file.name
                    ) ||
                    fileData.description ||
                    "";

                  return (
                    <AttachedFileItem
                      key={`selected-${index}`}
                      file={file}
                      description={description}
                      preview={preview}
                      isEditingDescription={isEditingDesc}
                      isUploading={inputSnapshot.isUploading}
                      onRemove={() => removeFile(index)}
                      onToggleEdit={() => toggleEditDescription(file.name)}
                      onUpdateDescription={(desc) =>
                        updateFileDescription(file.name, desc)
                      }
                    />
                  );
                })}
                {externalFiles.map((file: any, index: number) => {
                  const preview = multimodalInputActions.getFilePreview(
                    activeConversationId,
                    file.name
                  );
                  const isEditingDesc =
                    multimodalInputActions.getEditingDescription(
                      activeConversationId,
                      `external-${file.name}`
                    );
                  const description =
                    multimodalInputActions.getFileDescription(
                      activeConversationId,
                      `external-${file.name}`
                    ) ||
                    (file as ComponentFileWithDescription).description ||
                    "";

                  return (
                    <AttachedFileItem
                      key={`external-${index}`}
                      file={file}
                      description={description}
                      preview={preview}
                      isEditingDescription={isEditingDesc}
                      onRemove={() => {
                        onExternalFilesChange?.(
                          externalFiles.filter((_, i) => i !== index)
                        );
                        multimodalInputActions.clearFilePreview(
                          activeConversationId,
                          file.name
                        );
                        multimodalInputActions.clearFileDescription(
                          activeConversationId,
                          `external-${file.name}`
                        );
                      }}
                      onToggleEdit={() =>
                        multimodalInputActions.setEditingDescription(
                          activeConversationId,
                          `external-${file.name}`,
                          !isEditingDesc
                        )
                      }
                      onUpdateDescription={(desc) =>
                        multimodalInputActions.setFileDescription(
                          activeConversationId,
                          `external-${file.name}`,
                          desc
                        )
                      }
                    />
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
          multimodalInputActions.setLocalInput(
            activeConversationId,
            e.target.value
          );
          // Reset history navigation when user starts typing
          if (messageHistoryIndex !== -1) {
            setMessageHistoryIndex(-1);
            setOriginalInput("");
          }
        }}
        onEnterSubmit={(e) => handleFormSubmit(e as any)}
        onUpArrowPressed={handleUpArrowPressed}
        onDownArrowPressed={handleDownArrowPressed}
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
            : "Enter to send, Shift+Enter for new line, ↑↓ to navigate history"
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
                  !input.trim() ||
                  inputSnapshot.isUploading ||
                  isLoading ||
                  isStreaming
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
            const convertedFiles = files.map((f) => {
              const file = new File([], f.original_name, {
                type: f.mime_type || "application/octet-stream",
              });
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

            convertedFiles.forEach((file) => {
              const storeFile: StoreFileWithDescription = {
                file,
                description: (file as any).description || "",
                id:
                  (file as any).fileId ||
                  Date.now().toString() +
                    Math.random().toString(36).substring(2, 11),
              };
              inputActions.addSelectedFile(storeFile);
            });

            convertedFiles.forEach((file) => {
              const description =
                (file as any).description ||
                (file as any).autoDescription ||
                "";
              if (description) {
                multimodalInputActions.setFileDescription(
                  activeConversationId,
                  file.name,
                  description
                );
              }

              if (file.type.startsWith("image/")) {
                const clientId = localStorage.getItem("activeClientId");
                if (clientId && projectId) {
                  const fileName = (file as any).filePath.split("/").pop();
                  const previewUrl = `${API_BASE_URL}/uploads/${clientId}/${projectId}/${fileName}`;
                  multimodalInputActions.setFilePreview(
                    activeConversationId,
                    file.name,
                    previewUrl
                  );
                }
              }
            });
          }}
        />
      )}
    </form>
  );
}
