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

// Helper function to ensure conversation state is initialized
const ensureConversationState = (conversationId: string) => {
  if (!multimodalStore[conversationId]) {
    multimodalStore[conversationId] = {
      localInput: "",
      filePreviews: {},
      fileDescriptions: {},
      editingDescriptions: {},
      uploadCancelled: false,
    };
  }
  return multimodalStore[conversationId];
};

// Multimodal actions
export const multimodalInputActions = {
  setLocalInput: (conversationId: string, input: string) => {
    const state = ensureConversationState(conversationId);
    state.localInput = input;
  },
  getFilePreview: (conversationId: string, filename: string) => {
    try {
      return multimodalStore[conversationId]?.filePreviews?.[filename] || "";
    } catch (error) {
      console.warn('Error accessing file preview:', error);
      return "";
    }
  },
  setFilePreview: (conversationId: string, filename: string, preview: string) => {
    const state = ensureConversationState(conversationId);
    state.filePreviews[filename] = preview;
  },
  clearFilePreview: (conversationId: string, filename: string) => {
    if (multimodalStore[conversationId]?.filePreviews) {
      delete multimodalStore[conversationId].filePreviews[filename];
    }
  },
  getFileDescription: (conversationId: string, filename: string) => {
    try {
      return multimodalStore[conversationId]?.fileDescriptions?.[filename] || "";
    } catch (error) {
      console.warn('Error accessing file description:', error);
      return "";
    }
  },
  setFileDescription: (conversationId: string, filename: string, description: string) => {
    const state = ensureConversationState(conversationId);
    state.fileDescriptions[filename] = description;
  },
  clearFileDescription: (conversationId: string, filename: string) => {
    if (multimodalStore[conversationId]?.fileDescriptions) {
      delete multimodalStore[conversationId].fileDescriptions[filename];
    }
  },
  getEditingDescription: (conversationId: string, filename: string) => {
    try {
      return multimodalStore[conversationId]?.editingDescriptions?.[filename] || false;
    } catch (error) {
      console.warn('Error accessing editing description:', error);
      return false;
    }
  },
  setEditingDescription: (conversationId: string, filename: string, editing: boolean) => {
    const state = ensureConversationState(conversationId);
    state.editingDescriptions[filename] = editing;
  },
  setUploadCancelled: (conversationId: string, cancelled: boolean) => {
    const state = ensureConversationState(conversationId);
    state.uploadCancelled = cancelled;
  },
  isUploadCancelled: (conversationId: string) => {
    try {
      return multimodalStore[conversationId]?.uploadCancelled || false;
    } catch (error) {
      console.warn('Error accessing upload cancelled status:', error);
      return false;
    }
  },
};