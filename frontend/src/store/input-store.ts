import { proxy } from 'valtio';

interface FileWithDescription {
  file: File;
  description?: string;
  id: string;
}

interface UploadProgress {
  [key: string]: number;
}

interface InputState {
  // File handling
  selectedFiles: FileWithDescription[];
  isDragging: boolean;
  showFileBrowser: boolean;
  isUploading: boolean;
  uploadProgress: UploadProgress;
  
  // Input states per conversation
  draftMessages: Record<string, string>;
  attachments: Record<string, File[]>;
  isTyping: Record<string, boolean>;
  
  // Focus management
  shouldFocusTextarea: boolean;
}

export const inputStore = proxy<InputState>({
  // File handling
  selectedFiles: [],
  isDragging: false,
  showFileBrowser: false,
  isUploading: false,
  uploadProgress: {},
  
  // Input states per conversation  
  draftMessages: {},
  attachments: {},
  isTyping: {},
  
  // Focus management
  shouldFocusTextarea: false,
});

export const inputActions = {
  // File management
  setSelectedFiles: (files: FileWithDescription[]) => {
    inputStore.selectedFiles = files;
  },
  
  addSelectedFile: (file: FileWithDescription) => {
    inputStore.selectedFiles.push(file);
  },
  
  removeSelectedFile: (fileId: string) => {
    inputStore.selectedFiles = inputStore.selectedFiles.filter(f => f.id !== fileId);
  },
  
  clearSelectedFiles: () => {
    inputStore.selectedFiles = [];
  },
  
  setDragging: (isDragging: boolean) => {
    inputStore.isDragging = isDragging;
  },
  
  setShowFileBrowser: (show: boolean) => {
    inputStore.showFileBrowser = show;
  },
  
  setUploading: (isUploading: boolean) => {
    inputStore.isUploading = isUploading;
  },
  
  setUploadProgress: (progress: UploadProgress) => {
    inputStore.uploadProgress = progress;
  },
  
  updateUploadProgress: (fileId: string, progress: number) => {
    inputStore.uploadProgress[fileId] = progress;
  },
  
  clearUploadProgress: () => {
    inputStore.uploadProgress = {};
  },
  
  // Draft message management
  setDraftMessage: (conversationId: string, message: string) => {
    inputStore.draftMessages[conversationId] = message;
  },
  
  getDraftMessage: (conversationId: string): string => {
    return inputStore.draftMessages[conversationId] || '';
  },
  
  clearDraftMessage: (conversationId: string) => {
    delete inputStore.draftMessages[conversationId];
  },
  
  // Attachment management
  setAttachments: (conversationId: string, files: File[]) => {
    inputStore.attachments[conversationId] = files;
  },
  
  getAttachments: (conversationId: string): File[] => {
    return inputStore.attachments[conversationId] || [];
  },
  
  clearAttachments: (conversationId: string) => {
    delete inputStore.attachments[conversationId];
  },
  
  // Typing state management
  setTyping: (conversationId: string, isTyping: boolean) => {
    inputStore.isTyping[conversationId] = isTyping;
  },
  
  getTyping: (conversationId: string): boolean => {
    return inputStore.isTyping[conversationId] || false;
  },
  
  clearTyping: (conversationId: string) => {
    delete inputStore.isTyping[conversationId];
  },
  
  // Focus management
  setFocusTextarea: (shouldFocus: boolean) => {
    inputStore.shouldFocusTextarea = shouldFocus;
  },
};