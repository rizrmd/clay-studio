import { API_BASE_URL } from "@/lib/utils/url";
import { multimodalInputActions, inputActions, type StoreFileWithDescription } from "@/lib/store/chat/input-store";

interface ComponentFileWithDescription extends File {
  description?: string;
  autoDescription?: string;
  preview?: string;
  backendId?: string;
}


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
        inputActions.updateUploadProgress(file?.name, 0);

        const formData = new FormData();
        formData.append("file", file);

        const xhr = new XMLHttpRequest();

        xhr.upload.onprogress = (event) => {
          if (event.lengthComputable) {
            const percentComplete = (event.loaded / event.total) * 100;
            inputActions.updateUploadProgress(file?.name, Math.round(percentComplete));
          }
        };

        await new Promise((resolve, reject) => {
          xhr.onload = () => {
            if (xhr.status === 200) {
              const result = JSON.parse(xhr.responseText);
              const uploadedFile: ComponentFileWithDescription = {
                ...file,
                description: result.description || multimodalInputActions.getFileDescription(activeConversationId, file.name),
                autoDescription: result.auto_description,
                preview: multimodalInputActions.getFilePreview(activeConversationId, file.name),
                backendId: result.id // Add backend ID to uploaded file
              };
              uploadedFilesList.push(uploadedFile);

              if (result.description || multimodalInputActions.getFileDescription(activeConversationId, file.name)) {
                multimodalInputActions.setFileDescription(activeConversationId, file.name,
                  result.description || multimodalInputActions.getFileDescription(activeConversationId, file.name) || "");
              }

              resolve(result);
            } else {
              reject(new Error(`Failed to upload ${file?.name}`));
            }
          };

          xhr.onerror = () => reject(new Error(`Failed to upload ${file?.name}`));
          xhr.onabort = () => reject(new Error(`Upload cancelled for ${file?.name}`));

          xhr.open("POST", `${API_BASE_URL}/upload?client_id=${clientId}&project_id=${projectId}`);
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