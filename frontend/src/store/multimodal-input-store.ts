import { proxy } from 'valtio';

interface MultimodalInputState {
  // Local input state per conversation
  localInputs: Record<string, string>;

  // Upload state
  uploadAbortControllers: Record<string, AbortController | null>;

  // File editing state
  editingDescriptions: Record<string, { [key: string]: boolean }>;
  fileDescriptions: Record<string, { [key: string]: string }>;
  filePreviews: Record<string, { [key: string]: string }>;
}

export const multimodalInputStore = proxy<MultimodalInputState>({
  // Local input state per conversation
  localInputs: {},

  // Upload state
  uploadAbortControllers: {},

  // File editing state
  editingDescriptions: {},
  fileDescriptions: {},
  filePreviews: {},
});

export const multimodalInputActions = {
  // Local input actions
  setLocalInput: (conversationId: string, input: string) => {
    multimodalInputStore.localInputs[conversationId] = input;
  },

  getLocalInput: (conversationId: string): string => {
    return multimodalInputStore.localInputs[conversationId] || '';
  },

  clearLocalInput: (conversationId: string) => {
    delete multimodalInputStore.localInputs[conversationId];
  },

  // Upload actions
  setUploadAbortController: (conversationId: string, controller: AbortController | null) => {
    multimodalInputStore.uploadAbortControllers[conversationId] = controller;
  },

  getUploadAbortController: (conversationId: string): AbortController | null => {
    return multimodalInputStore.uploadAbortControllers[conversationId] || null;
  },

  clearUploadAbortController: (conversationId: string) => {
    delete multimodalInputStore.uploadAbortControllers[conversationId];
  },

  // File editing actions
  setEditingDescription: (conversationId: string, fileId: string, editing: boolean) => {
    if (!multimodalInputStore.editingDescriptions[conversationId]) {
      multimodalInputStore.editingDescriptions[conversationId] = {};
    }
    multimodalInputStore.editingDescriptions[conversationId][fileId] = editing;
  },

  getEditingDescription: (conversationId: string, fileId: string): boolean => {
    return multimodalInputStore.editingDescriptions[conversationId]?.[fileId] || false;
  },

  // File description actions
  setFileDescription: (conversationId: string, fileId: string, description: string) => {
    if (!multimodalInputStore.fileDescriptions[conversationId]) {
      multimodalInputStore.fileDescriptions[conversationId] = {};
    }
    multimodalInputStore.fileDescriptions[conversationId][fileId] = description;
  },

  getFileDescription: (conversationId: string, fileId: string): string => {
    return multimodalInputStore.fileDescriptions[conversationId]?.[fileId] || '';
  },

  clearFileDescription: (conversationId: string, fileId: string) => {
    if (multimodalInputStore.fileDescriptions[conversationId]) {
      delete multimodalInputStore.fileDescriptions[conversationId][fileId];
    }
  },

  // File preview actions
  setFilePreview: (conversationId: string, fileId: string, preview: string) => {
    if (!multimodalInputStore.filePreviews[conversationId]) {
      multimodalInputStore.filePreviews[conversationId] = {};
    }
    multimodalInputStore.filePreviews[conversationId][fileId] = preview;
  },

  getFilePreview: (conversationId: string, fileId: string): string => {
    return multimodalInputStore.filePreviews[conversationId]?.[fileId] || '';
  },

  clearFilePreview: (conversationId: string, fileId: string) => {
    if (multimodalInputStore.filePreviews[conversationId]) {
      delete multimodalInputStore.filePreviews[conversationId][fileId];
    }
  },

  // Cleanup actions
  clearConversationState: (conversationId: string) => {
    delete multimodalInputStore.localInputs[conversationId];
    delete multimodalInputStore.uploadAbortControllers[conversationId];
    delete multimodalInputStore.editingDescriptions[conversationId];
    delete multimodalInputStore.fileDescriptions[conversationId];
    delete multimodalInputStore.filePreviews[conversationId];
  },
};