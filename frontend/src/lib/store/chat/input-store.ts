import { proxy } from "valtio";

export interface StoreFileWithDescription {
  file: File;
  description?: string;
  id: string;
}

// Input store interface
interface InputStore {
  isDragging: boolean;
  isUploading: boolean;
  showFileBrowser: boolean;
  uploadProgress: Record<string, number>;
  selectedFiles: StoreFileWithDescription[];
}

// Input store
export const inputStore = proxy<InputStore>({
  isDragging: false,
  isUploading: false,
  showFileBrowser: false,
  uploadProgress: {},
  selectedFiles: [],
});

// Input actions
export const inputActions = {
  setDragging: (dragging: boolean) => {
    inputStore.isDragging = dragging;
  },
  setUploading: (uploading: boolean) => {
    inputStore.isUploading = uploading;
  },
  setShowFileBrowser: (show: boolean) => {
    inputStore.showFileBrowser = show;
  },
  clearUploadProgress: () => {
    inputStore.uploadProgress = {};
  },
  updateUploadProgress: (filename: string, progress: number) => {
    inputStore.uploadProgress[filename] = progress;
  },
  removeSelectedFile: (id: string) => {
    inputStore.selectedFiles = inputStore.selectedFiles.filter(f => f.id !== id);
  },
  addSelectedFile: (file: StoreFileWithDescription) => {
    inputStore.selectedFiles = [...inputStore.selectedFiles, file];
  },
  setSelectedFiles: (files: StoreFileWithDescription[]) => {
    inputStore.selectedFiles = files;
  },
  clearSelectedFiles: () => {
    inputStore.selectedFiles = [];
  },
};


// Multimodal input store interface
interface MultimodalState {
  [conversationId: string]: {
    localInput: string;
    filePreviews: Record<string, string>;
    fileDescriptions: Record<string, string>;
    editingDescriptions: Record<string, boolean>;
    uploadCancelled: boolean;
  };
}

// Multimodal store
export const multimodalStore = proxy<MultimodalState>({});

// Multimodal actions
export const multimodalInputActions = {
  setLocalInput: (conversationId: string, input: string) => {
    if (!multimodalStore[conversationId]) {
      multimodalStore[conversationId] = {
        localInput: "",
        filePreviews: {},
        fileDescriptions: {},
        editingDescriptions: {},
        uploadCancelled: false,
      };
    }
    multimodalStore[conversationId].localInput = input;
  },
  getFilePreview: (conversationId: string, filename: string) => {
    return multimodalStore[conversationId]?.filePreviews[filename] || "";
  },
  setFilePreview: (conversationId: string, filename: string, preview: string) => {
    if (!multimodalStore[conversationId]) {
      multimodalStore[conversationId] = {
        localInput: "",
        filePreviews: {},
        fileDescriptions: {},
        editingDescriptions: {},
        uploadCancelled: false,
      };
    }
    multimodalStore[conversationId].filePreviews[filename] = preview;
  },
  clearFilePreview: (conversationId: string, filename: string) => {
    if (multimodalStore[conversationId]?.filePreviews) {
      delete multimodalStore[conversationId].filePreviews[filename];
    }
  },
  getFileDescription: (conversationId: string, filename: string) => {
    return multimodalStore[conversationId]?.fileDescriptions[filename] || "";
  },
  setFileDescription: (conversationId: string, filename: string, description: string) => {
    if (!multimodalStore[conversationId]) {
      multimodalStore[conversationId] = {
        localInput: "",
        filePreviews: {},
        fileDescriptions: {},
        editingDescriptions: {},
        uploadCancelled: false,
      };
    }
    multimodalStore[conversationId].fileDescriptions[filename] = description;
  },
  clearFileDescription: (conversationId: string, filename: string) => {
    if (multimodalStore[conversationId]?.fileDescriptions) {
      delete multimodalStore[conversationId].fileDescriptions[filename];
    }
  },
  getEditingDescription: (conversationId: string, filename: string) => {
    return multimodalStore[conversationId]?.editingDescriptions[filename] || false;
  },
  setEditingDescription: (conversationId: string, filename: string, editing: boolean) => {
    if (!multimodalStore[conversationId]) {
      multimodalStore[conversationId] = {
        localInput: "",
        filePreviews: {},
        fileDescriptions: {},
        editingDescriptions: {},
        uploadCancelled: false,
      };
    }
    multimodalStore[conversationId].editingDescriptions[filename] = editing;
  },
  setUploadCancelled: (conversationId: string, cancelled: boolean) => {
    if (!multimodalStore[conversationId]) {
      multimodalStore[conversationId] = {
        localInput: "",
        filePreviews: {},
        fileDescriptions: {},
        editingDescriptions: {},
        uploadCancelled: false,
      };
    }
    multimodalStore[conversationId].uploadCancelled = cancelled;
  },
  isUploadCancelled: (conversationId: string) => {
    return multimodalStore[conversationId]?.uploadCancelled || false;
  },
};