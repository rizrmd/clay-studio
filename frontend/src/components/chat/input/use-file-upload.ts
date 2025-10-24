import { API_BASE_URL } from "@/lib/utils/url";
import { multimodalInputActions, inputActions, type StoreFileWithDescription } from "@/lib/store/chat/input-store";

interface ComponentFileWithDescription extends File {
  description?: string;
  autoDescription?: string;
  preview?: string;
  backendId?: string;
  filePath?: string;
}


// Helper function to safely extract file properties from various object types
const getSafeFileProps = (fileObj: any): { name: string; type: string; size: number } => {
  // Skip JSON.stringify entirely and go straight to defensive property access
  try {
    const name = (() => {
      try { return fileObj.name; } catch {
        try { return (fileObj as any).originalName; } catch { return undefined; }
      }
    })();

    const type = (() => {
      try { return fileObj.type; } catch { return undefined; }
    })();

    const size = (() => {
      try { return fileObj.size; } catch { return undefined; }
    })();

    // If we have basic properties, return them
    if (name) {
      return {
        name: name,
        type: type || 'application/octet-stream',
        size: size || 0
      };
    }

    // If we don't even have a name, generate one
    const timestamp = Date.now();
    const randomStr = Math.random().toString(36).substring(2, 8);

    return {
      name: `file_${timestamp}_${randomStr}`,
      type: type || 'application/octet-stream',
      size: size || 0
    };
  } catch (error) {
    console.error("Error in getSafeFileProps:", error);
    return {
      name: `file_${Date.now()}`,
      type: 'application/octet-stream',
      size: 0
    };
  }
};

export function useFileUpload(activeConversationId: string, projectId?: string) {
  const uploadFiles = async (files: File[]) => {
    const clientId = localStorage.getItem("activeClientId");
    if (!clientId || !projectId) {
      return;
    }

    inputActions.setUploading(true);
    multimodalInputActions.setUploadCancelled(activeConversationId, false);

    const uploadedFilesList: ComponentFileWithDescription[] = [];

    const cleanup = () => {
      inputActions.setUploading(false);
      inputActions.clearUploadProgress();
    };

    try {
      for (const file of files) {
        // Validate file object before processing
        if (!file || typeof file !== 'object') {
          console.error("Invalid file object:", file);
          continue;
        }

        // Safely extract file properties
        const fileProps = getSafeFileProps(file);

        inputActions.updateUploadProgress(fileProps.name, 0);

        const formData = new FormData();
        formData.append("file", file);

        const xhr = new XMLHttpRequest();

        xhr.upload.onprogress = (event) => {
          if (event.lengthComputable) {
            const percentComplete = (event.loaded / event.total) * 100;
            inputActions.updateUploadProgress(fileProps.name, Math.round(percentComplete));
          }
        };

        await new Promise((resolve, reject) => {
          xhr.onload = () => {
            if (xhr.status === 200) {
              const result = JSON.parse(xhr.responseText);
              const uploadedFile: ComponentFileWithDescription = {
                ...file,
                name: fileProps.name,
                type: fileProps.type,
                size: fileProps.size,
                description: result.description || multimodalInputActions.getFileDescription(activeConversationId, fileProps.name),
                autoDescription: result.auto_description,
                preview: multimodalInputActions.getFilePreview(activeConversationId, fileProps.name),
                backendId: result.id, // Add backend ID to uploaded file
                filePath: result.file_path // Add file path to uploaded file
              };
              uploadedFilesList.push(uploadedFile);

              if (result.description || multimodalInputActions.getFileDescription(activeConversationId, fileProps.name)) {
                multimodalInputActions.setFileDescription(activeConversationId, fileProps.name,
                  result.description || multimodalInputActions.getFileDescription(activeConversationId, fileProps.name) || "");
              }

              resolve(result);
            } else {
              reject(new Error(`Failed to upload ${fileProps.name}`));
            }
          };

          xhr.onerror = () => reject(new Error(`Failed to upload ${fileProps.name}`));
          xhr.onabort = () => reject(new Error(`Upload cancelled for ${fileProps.name}`));

          xhr.open("POST", `${API_BASE_URL}/uploads?client_id=${clientId}&project_id=${projectId}`);
          xhr.withCredentials = true;
          xhr.send(formData);
        });
      }

      uploadedFilesList.forEach(file => {
        const fileWithDesc: StoreFileWithDescription = {
          file,
          description: file.description || '',
          id: file.backendId || Date.now().toString() + Math.random().toString(36).substring(2, 11) // Use backend ID if available
        };
        inputActions.addSelectedFile(fileWithDesc);
      });
    } catch (error) {
      console.error('Upload failed:', error);
      inputActions.clearUploadProgress();
    } finally {
      cleanup();
    }
  };

  const cancelUpload = () => {
    multimodalInputActions.setUploadCancelled(activeConversationId, true);
    inputActions.setUploading(false);
    inputActions.clearUploadProgress();
  };

  const generatePreview = (file: File) => {
    if (file.type.startsWith("image/")) {
      const reader = new FileReader();
      reader.onload = (e) => {
        if (e.target?.result) {
          multimodalInputActions.setFilePreview(activeConversationId, file.name, e.target!.result as string);
        }
      };
      reader.readAsDataURL(file);
    }
  };

  return {
    uploadFiles,
    cancelUpload,
    generatePreview,
  };
}