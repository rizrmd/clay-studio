import { useState, useEffect, useRef } from "react";
import { logger } from "@/lib/logger";
import {
  X,
  Check,
  Search,
  FileImage,
  FileCode,
  File,
  Upload,
  Edit2,
  Trash2,
} from "lucide-react";
import {
  Dialog,
  DialogContent,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { API_BASE_URL } from "@/lib/url";
import { cn } from "@/lib/utils";

interface FileUpload {
  id: string;
  file_name: string;
  original_name: string;
  file_path: string;
  file_size: number;
  mime_type?: string;
  description?: string;
  auto_description?: string;
  created_at: string;
}

interface FileBrowserProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projectId: string;
  onFilesSelected: (files: FileUpload[]) => void;
}

const formatFileSize = (bytes: number | undefined | null) => {
  // Handle undefined, null, or NaN
  if (bytes === undefined || bytes === null || isNaN(bytes)) {
    return "Size unknown";
  }
  // Handle 0 bytes
  if (bytes === 0) {
    return "0 Bytes";
  }
  // Calculate size
  const k = 1024;
  const sizes = ["Bytes", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const size = Math.round((bytes / Math.pow(k, i)) * 100) / 100;
  return size + " " + sizes[i];
};

const getFileIcon = (mimeType?: string, fileName?: string) => {
  if (!mimeType && fileName) {
    const ext = fileName.split(".").pop()?.toLowerCase();
    if (["jpg", "jpeg", "png", "gif", "webp", "svg"].includes(ext || "")) {
      return <FileImage className="h-4 w-4" />;
    }
    if (
      [
        "js",
        "ts",
        "jsx",
        "tsx",
        "py",
        "java",
        "cpp",
        "c",
        "h",
        "css",
        "html",
        "xml",
        "json",
      ].includes(ext || "")
    ) {
      return <FileCode className="h-4 w-4" />;
    }
  }

  if (mimeType?.startsWith("image/")) {
    return <FileImage className="h-4 w-4" />;
  }
  if (
    mimeType?.startsWith("text/") ||
    mimeType?.includes("json") ||
    mimeType?.includes("xml")
  ) {
    return <FileCode className="h-4 w-4" />;
  }

  return <File className="h-4 w-4" />;
};

export function FileBrowser({
  open,
  onOpenChange,
  projectId,
  onFilesSelected,
}: FileBrowserProps) {
  const [files, setFiles] = useState<FileUpload[]>([]);
  const [selectedFiles, setSelectedFiles] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [isUploading, setIsUploading] = useState(false);
  const [uploadProgress, setUploadProgress] = useState<{
    [key: string]: number;
  }>({});
  const [editingFile, setEditingFile] = useState<string | null>(null);
  const [editedDescription, setEditedDescription] = useState("");
  const fileInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) {
      loadFiles();
      // Clear any previous selections when opening
      setSelectedFiles(new Set());
    }
  }, [open, projectId]);

  const loadFiles = async () => {
    setIsLoading(true);
    try {
      const clientId = localStorage.getItem("activeClientId");
      if (!clientId) return;

      const response = await fetch(
        `${API_BASE_URL}/uploads?client_id=${clientId}&project_id=${projectId}`,
        { credentials: "include" }
      );

      if (response.ok) {
        const data = await response.json();
        setFiles(data);
      }
    } catch (error) {
      // Failed to load files
    } finally {
      setIsLoading(false);
    }
  };

  const filteredFiles = files.filter(
    (file) =>
      file.original_name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      file.description?.toLowerCase().includes(searchQuery.toLowerCase()) ||
      file.auto_description?.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const toggleFileSelection = (fileId: string) => {
    const newSelection = new Set(selectedFiles);
    if (newSelection.has(fileId)) {
      newSelection.delete(fileId);
    } else {
      newSelection.add(fileId);
    }
    setSelectedFiles(newSelection);
  };

  const handleSelectFiles = () => {
    const selected = files.filter((f) => selectedFiles.has(f.id));
    onFilesSelected(selected);
    setSelectedFiles(new Set());
    onOpenChange(false);
  };

  const startEditDescription = (fileId: string, currentDescription: string) => {
    setEditingFile(fileId);
    setEditedDescription(currentDescription || "");
  };

  const saveDescription = async (fileId: string) => {
    try {
      const clientId = localStorage.getItem("activeClientId");
      if (!clientId) return;

      const response = await fetch(
        `${API_BASE_URL}/uploads/${fileId}/description`,
        {
          method: "PUT",
          credentials: "include",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            description: editedDescription,
            client_id: clientId,
            project_id: projectId,
          }),
        }
      );

      if (response.ok) {
        // Update local state
        setFiles((prev) =>
          prev.map((file) =>
            file.id === fileId
              ? { ...file, description: editedDescription }
              : file
          )
        );
      }
    } catch (error) {
      // Failed to update description
    } finally {
      setEditingFile(null);
      setEditedDescription("");
    }
  };

  const cancelEdit = () => {
    setEditingFile(null);
    setEditedDescription("");
  };

  const deleteFile = async (fileId: string) => {
    try {
      const clientId = localStorage.getItem("activeClientId");
      if (!clientId) return;

      const response = await fetch(
        `${API_BASE_URL}/uploads/${fileId}?client_id=${clientId}`,
        {
          method: "DELETE",
          credentials: "include",
        }
      );

      if (response.ok) {
        // Remove from local state
        setFiles((prev) => prev.filter((file) => file.id !== fileId));
        // Remove from selection if selected
        setSelectedFiles((prev) => {
          const newSelection = new Set(prev);
          newSelection.delete(fileId);
          return newSelection;
        });
      }
    } catch (error) {
      // Failed to delete file
      logger.error("FileBrowser: Failed to delete file:", error);
    }
  };

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const selectedFiles = Array.from(e.target.files || []);
    if (selectedFiles.length === 0) return;

    const clientId = localStorage.getItem("activeClientId");
    if (!clientId) return;

    setIsUploading(true);
    const uploadedFiles: FileUpload[] = [];

    try {
      for (const file of selectedFiles) {
        setUploadProgress((prev) => ({ ...prev, [file.name]: 0 }));

        const formData = new FormData();
        formData.append("file", file);

        const xhr = new XMLHttpRequest();

        xhr.upload.onprogress = (event) => {
          if (event.lengthComputable) {
            const percentComplete = (event.loaded / event.total) * 100;
            setUploadProgress((prev) => ({
              ...prev,
              [file.name]: percentComplete,
            }));
          }
        };

        await new Promise((resolve, reject) => {
          xhr.onload = () => {
            if (xhr.status === 200) {
              const result = JSON.parse(xhr.responseText);
              uploadedFiles.push(result);
              resolve(result);
            } else {
              reject(new Error(`Failed to upload ${file.name}`));
            }
          };

          xhr.onerror = () =>
            reject(new Error(`Failed to upload ${file.name}`));

          xhr.open(
            "POST",
            `${API_BASE_URL}/upload?client_id=${clientId}&project_id=${projectId}`
          );
          xhr.withCredentials = true;
          xhr.send(formData);
        });

        setUploadProgress((prev) => {
          const newProgress = { ...prev };
          delete newProgress[file.name];
          return newProgress;
        });
      }

      // Refresh the file list
      await loadFiles();

      // Auto-select newly uploaded files
      const newSelection = new Set<string>();
      uploadedFiles.forEach((file) => newSelection.add(file.id));
      setSelectedFiles(newSelection);
    } catch (error) {
      // Upload failed
      setUploadProgress({});
    } finally {
      setIsUploading(false);
      if (fileInputRef.current) {
        fileInputRef.current.value = "";
      }
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="min-w-[95vw] sm:min-w-[90vw] max-w-[1400px] h-[85vh] sm:h-[90vh] flex flex-col p-0 gap-0"
        showCloseButton={false}
      >
        <div className="flex-1 flex overflow-hidden">
          <div className="flex flex-col items-stretch flex-1">
            <div className="flex flex-row p-2 border-b gap-2">
              {/* Search and Upload controls in header */}
              <div className="flex gap-2 flex-1 relative min-w-0">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  placeholder="Search files..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-10 flex-1"
                />
              </div>

              {isUploading && Object.keys(uploadProgress).length > 0 ? (
                <div className="space-y-2 p-3 border rounded-lg bg-muted/50 w-full sm:w-auto">
                  <div className="text-xs font-medium">Uploading...</div>
                  {Object.entries(uploadProgress).map(
                    ([fileName, progress]) => (
                      <div key={fileName} className="space-y-1">
                        <div className="flex items-center justify-between text-xs">
                          <span className="truncate">{fileName}</span>
                          <span>{Math.round(progress)}%</span>
                        </div>
                        <div className="h-1 bg-secondary rounded-full overflow-hidden">
                          <div
                            className="h-full bg-primary transition-all duration-300"
                            style={{ width: `${progress}%` }}
                          />
                        </div>
                      </div>
                    )
                  )}
                </div>
              ) : (
                <>
                  <Button
                    variant="outline"
                    onClick={() => fileInputRef.current?.click()}
                    disabled={isUploading}
                  >
                    <Upload className="h-4 w-4 mr-2" />
                    Upload New
                  </Button>
                  <input
                    ref={fileInputRef}
                    type="file"
                    multiple
                    onChange={handleFileSelect}
                    className="hidden"
                    accept="*"
                  />
                </>
              )}
            </div>
            {/* Main content area */}
            <ScrollArea className="flex-1">
              {isLoading ? (
                <div className="text-center py-8 text-muted-foreground">
                  Loading files...
                </div>
              ) : filteredFiles.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  {searchQuery
                    ? "No files found matching your search"
                    : "No uploaded files in this project"}
                </div>
              ) : (
                <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4 p-4">
                  {filteredFiles.map((file) => (
                    <div
                      key={file.id}
                      className={cn(
                        "flex flex-col p-3 rounded-lg border cursor-pointer transition-colors h-full",
                        selectedFiles.has(file.id)
                          ? "bg-accent border-primary"
                          : "hover:bg-accent/50"
                      )}
                      onClick={() => toggleFileSelection(file.id)}
                    >
                      <div className="flex items-start gap-2">
                        <div className="mt-1 shrink-0">
                          {/* Show image preview for image files */}
                          {file.mime_type?.startsWith("image/") ||
                          ["jpg", "jpeg", "png", "gif", "webp", "svg"].includes(
                            file.original_name
                              .split(".")
                              .pop()
                              ?.toLowerCase() || ""
                          ) ? (
                            <div className="relative w-12 h-12 sm:w-16 sm:h-16 border rounded overflow-hidden bg-secondary flex-shrink-0">
                              <img
                                src={`${API_BASE_URL}/uploads/${localStorage.getItem(
                                  "activeClientId"
                                )}/${projectId}/${
                                  file.file_name ||
                                  file.file_path.split("/").pop()
                                }`}
                                alt={file.original_name}
                                className="w-full h-full object-cover"
                                onError={(e) => {
                                  // If image fails to load, show icon instead
                                  const target = e.target as HTMLImageElement;
                                  target.style.display = "none";
                                  const parent = target.parentElement;
                                  if (parent) {
                                    parent.innerHTML =
                                      '<div class="w-full h-full flex items-center justify-center"><svg class="h-6 w-6 text-muted-foreground" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2" ry="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/></svg></div>';
                                  }
                                }}
                              />
                            </div>
                          ) : (
                            <div className="w-12 h-12 sm:w-16 sm:h-16 border rounded flex items-center justify-center bg-secondary flex-shrink-0">
                              {getFileIcon(file.mime_type, file.original_name)}
                            </div>
                          )}
                        </div>
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-1">
                            <span className="font-medium truncate text-sm">
                              {file.original_name}
                            </span>
                            {selectedFiles.has(file.id) && (
                              <Check className="h-4 w-4 text-primary shrink-0" />
                            )}
                          </div>
                          {editingFile === file.id ? (
                            <div
                              className="mt-2 flex gap-1"
                              onClick={(e) => e.stopPropagation()}
                            >
                              <Input
                                type="text"
                                value={editedDescription}
                                onChange={(e) =>
                                  setEditedDescription(e.target.value)
                                }
                                placeholder="Add a description..."
                                className="h-7 text-xs flex-1"
                                autoFocus
                                onKeyDown={(e) => {
                                  if (e.key === "Enter") {
                                    e.preventDefault();
                                    saveDescription(file.id);
                                  } else if (e.key === "Escape") {
                                    cancelEdit();
                                  }
                                }}
                              />
                              <Button
                                size="icon"
                                variant="ghost"
                                className="h-7 w-7"
                                onClick={() => saveDescription(file.id)}
                              >
                                <Check className="h-3 w-3" />
                              </Button>
                              <Button
                                size="icon"
                                variant="ghost"
                                className="h-7 w-7"
                                onClick={cancelEdit}
                              >
                                <X className="h-3 w-3" />
                              </Button>
                            </div>
                          ) : (
                            <div className="mt-1 flex items-start gap-1 group">
                              <div className="text-xs text-muted-foreground line-clamp-3 flex-1">
                                <div className="line-clamp-2">
                                  {file.description || file.auto_description || "No description"}
                                </div>
                                <div className="opacity-70 mt-1 text-[10px] sm:text-xs">
                                  {formatFileSize(file.file_size)}
                                </div>
                                <div className="opacity-70 text-[10px] sm:text-xs hidden sm:block">
                                  {new Date(file.created_at).toLocaleDateString()}
                                </div>
                              </div>
                              <Button
                                size="icon"
                                variant="ghost"
                                className="h-5 w-5 opacity-0 group-hover:opacity-100 transition-opacity"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  startEditDescription(
                                    file.id,
                                    file.description ||
                                      file.auto_description ||
                                      ""
                                  );
                                }}
                              >
                                <Edit2 className="h-3 w-3" />
                              </Button>
                            </div>
                          )}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </ScrollArea>
          </div>

          {/* Sidebar for selected file details */}
          <div className="hidden md:flex min-w-[280px] max-w-[280px] border-l pl-4 pt-4 flex-col gap-4 overflow-hidden">
            <div className="font-medium text-sm">
              {selectedFiles.size === 0
                ? "Select files to see details"
                : selectedFiles.size === 1
                ? "File Details"
                : `${selectedFiles.size} Files Selected`}
            </div>

            {selectedFiles.size > 0 && (
              <ScrollArea className="flex-1">
                <div className="space-y-4 pr-4">
                  {Array.from(selectedFiles).map((fileId) => {
                    const file = files.find((f) => f.id === fileId);
                    if (!file) return null;

                    return (
                      <div
                        key={fileId}
                        className="space-y-3 pb-4 border-b last:border-0"
                      >
                        {/* File preview */}
                        {(file.mime_type?.startsWith("image/") ||
                          ["jpg", "jpeg", "png", "gif", "webp", "svg"].includes(
                            file.original_name
                              .split(".")
                              .pop()
                              ?.toLowerCase() || ""
                          )) && (
                          <div className="relative w-full aspect-video border rounded overflow-hidden bg-secondary">
                            <img
                              src={`${API_BASE_URL}/uploads/${localStorage.getItem(
                                "activeClientId"
                              )}/${projectId}/${
                                file.file_name ||
                                file.file_path.split("/").pop()
                              }`}
                              alt={file.original_name}
                              className="w-full h-full object-contain"
                              onError={(e) => {
                                const target = e.target as HTMLImageElement;
                                target.style.display = "none";
                              }}
                            />
                          </div>
                        )}

                        {/* File name */}
                        <div>
                          <div className="text-xs text-muted-foreground mb-1">
                            Name
                          </div>
                          <div className="font-medium text-sm break-all">
                            {file.original_name}
                          </div>
                        </div>

                        {/* File info */}
                        <div className="grid grid-cols-2 gap-2 text-sm">
                          <div>
                            <div className="text-xs text-muted-foreground mb-1">
                              Size
                            </div>
                            <div>{formatFileSize(file.file_size)}</div>
                          </div>
                          <div>
                            <div className="text-xs text-muted-foreground mb-1">
                              Type
                            </div>
                            <div className="truncate">
                              {file.mime_type || "Unknown"}
                            </div>
                          </div>
                          <div className="col-span-2">
                            <div className="text-xs text-muted-foreground mb-1">
                              Uploaded
                            </div>
                            <div>
                              {new Date(file.created_at).toLocaleDateString()}
                            </div>
                          </div>
                        </div>

                        {/* Description */}
                        <div>
                          <div className="flex items-center justify-between mb-1">
                            <div className="text-xs text-muted-foreground">
                              Description
                            </div>
                            {editingFile !== file.id && (
                              <Button
                                size="icon"
                                variant="ghost"
                                className="h-5 w-5"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  startEditDescription(
                                    file.id,
                                    file.description ||
                                      file.auto_description ||
                                      ""
                                  );
                                }}
                              >
                                <Edit2 className="h-3 w-3" />
                              </Button>
                            )}
                          </div>
                          {editingFile === file.id ? (
                            <div className="flex gap-1">
                              <Input
                                type="text"
                                value={editedDescription}
                                onChange={(e) =>
                                  setEditedDescription(e.target.value)
                                }
                                placeholder="Add a description..."
                                className="h-7 text-xs flex-1"
                                autoFocus
                                onKeyDown={(e) => {
                                  if (e.key === "Enter") {
                                    e.preventDefault();
                                    saveDescription(file.id);
                                  } else if (e.key === "Escape") {
                                    cancelEdit();
                                  }
                                }}
                              />
                              <Button
                                size="icon"
                                variant="ghost"
                                className="h-7 w-7"
                                onClick={() => saveDescription(file.id)}
                              >
                                <Check className="h-3 w-3" />
                              </Button>
                              <Button
                                size="icon"
                                variant="ghost"
                                className="h-7 w-7"
                                onClick={cancelEdit}
                              >
                                <X className="h-3 w-3" />
                              </Button>
                            </div>
                          ) : (
                            <div className="text-sm">
                              {file.description ||
                                file.auto_description ||
                                "No description"}
                            </div>
                          )}
                        </div>
                        
                        {/* Delete button */}
                        <div className="pt-3">
                          <Button
                            variant="destructive"
                            size="sm"
                            className="w-full"
                            onClick={(e) => {
                              e.stopPropagation();
                              if (confirm(`Are you sure you want to delete "${file.original_name}"?`)) {
                                deleteFile(file.id);
                              }
                            }}
                          >
                            <Trash2 className="h-4 w-4 mr-2" />
                            Delete File
                          </Button>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </ScrollArea>
            )}

            {/* Summary stats for multiple selections */}
            {selectedFiles.size > 1 && (
              <div className="pt-4 border-t space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Total size:</span>
                  <span>
                    {formatFileSize(
                      Array.from(selectedFiles).reduce((sum, fileId) => {
                        const file = files.find((f) => f.id === fileId);
                        return sum + (file?.file_size || 0);
                      }, 0)
                    )}
                  </span>
                </div>
              </div>
            )}
          </div>
        </div>

        <div className="flex flex-col sm:flex-row sm:items-center justify-between pt-4 border-t mt-auto p-4 gap-3">
          <div className="text-sm text-muted-foreground">
            {selectedFiles.size > 0 && `${selectedFiles.size} file(s) selected`}
          </div>
          <div className="flex gap-2">
            <Button variant="outline" onClick={() => onOpenChange(false)} className="flex-1 sm:flex-none">
              Cancel
            </Button>
            <Button
              onClick={handleSelectFiles}
              disabled={selectedFiles.size === 0}
              className="flex-1 sm:flex-none"
            >
              Add Selected Files
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
