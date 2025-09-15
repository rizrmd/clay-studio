import { proxy } from "valtio";

interface ChatInputState {
  input: string;
  pendingFiles: File[];
}

const initialChatInputState: ChatInputState = {
  input: "",
  pendingFiles: [],
};

export const chatInputStore = proxy(initialChatInputState);

export const chatInputActions = {
  setInput: (input: string) => {
    chatInputStore.input = input;
  },

  clearInput: () => {
    chatInputStore.input = "";
  },

  addPendingFile: (file: File) => {
    chatInputStore.pendingFiles.push(file);
  },

  addPendingFiles: (files: File[]) => {
    chatInputStore.pendingFiles.push(...files);
  },

  removePendingFile: (index: number) => {
    chatInputStore.pendingFiles.splice(index, 1);
  },

  clearPendingFiles: () => {
    chatInputStore.pendingFiles = [];
  },

  setPendingFiles: (files: File[]) => {
    chatInputStore.pendingFiles = files;
  },

  reset: () => {
    chatInputStore.input = "";
    chatInputStore.pendingFiles = [];
  },
};