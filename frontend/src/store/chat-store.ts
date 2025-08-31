import { proxy, subscribe } from "valtio";
import { logger } from "@/lib/logger";
import {
  Message,
  ConversationContext,
  ProjectContextResponse,
} from "../types/chat";

// Types for our global state
export interface QueuedMessage {
  id: string;
  content: string;
  files: File[];
  timestamp: Date;
}

export interface ConversationState {
  messages: Message[];
  isLoading: boolean;
  isLoadingMessages: boolean;
  error: string | null;
  isStreaming: boolean;
  uploadedFiles: any[];
  forgottenAfterMessageId: string | null;
  forgottenCount: number;
  currentAbortController: AbortController | null;
  conversationContext?: ConversationContext;
  messageQueue: QueuedMessage[];
  isProcessingQueue: boolean;
  activeTools: string[];
  needsStreamResume?: boolean;
  pendingResumeContent?: string | null;
  resumeWithoutRemovingMessage?: boolean;
  conversationIdForResume?: string;
}

export interface InputState {
  draftMessage: string;
  attachments: File[];
  isTyping: boolean;
}

export interface UIState {
  sidebarOpen: boolean;
  fileSidebarOpen: boolean;
  activeTab: string;
  viewportHeight: number;
  keyboardHeight: number;
}

export interface GlobalState {
  // Project and auth state
  currentProjectId: string | null;
  activeConversationId: string | null;

  // Chat state keyed by conversation ID
  conversations: Record<string, ConversationState>;

  // Input state keyed by conversation ID
  inputs: Record<string, InputState>;

  // Project context cache
  projectContexts: Record<string, ProjectContextResponse>;

  // UI state
  ui: UIState;
}

// Helper function to deep clone objects to avoid readonly issues
const deepClone = <T>(obj: T): T => {
  if (obj === null || typeof obj !== "object") return obj;
  if (obj instanceof Date) return new Date(obj.getTime()) as T;
  if (Array.isArray(obj)) return obj.map((item) => deepClone(item)) as T;

  const cloned = {} as T;
  for (const key in obj) {
    if (Object.prototype.hasOwnProperty.call(obj, key)) {
      cloned[key] = deepClone(obj[key]);
    }
  }
  return cloned;
};

// Initial state
const initialState: GlobalState = {
  currentProjectId: null,
  activeConversationId: null,
  conversations: {},
  inputs: {},
  projectContexts: {},
  ui: {
    sidebarOpen: true,
    fileSidebarOpen: false,
    activeTab: "chat",
    viewportHeight: typeof window !== "undefined" ? window.innerHeight : 0,
    keyboardHeight: 0,
  },
};

// Create the proxy state
export const store = proxy(initialState);

// Helper functions to manage conversation state
export const getConversationState = (
  conversationId: string
): ConversationState => {
  if (!store.conversations[conversationId]) {
    // Try to restore queue from sessionStorage
    let restoredQueue: QueuedMessage[] = [];
    const savedQueue = sessionStorage.getItem(`clay_queue_${conversationId}`);

    if (savedQueue) {
      try {
        const parsed = JSON.parse(savedQueue);
        restoredQueue = parsed.map((msg: any) => ({
          ...msg,
          files: [], // Files can't be restored from sessionStorage
          timestamp: new Date(msg.timestamp),
        }));
      } catch (e) {
        logger.error("ChatStore: Failed to restore queue:", e);
      }
    }

    store.conversations[conversationId] = {
      messages: [],
      isLoading: false,
      isLoadingMessages: false,
      error: null,
      isStreaming: false,
      uploadedFiles: [],
      forgottenAfterMessageId: null,
      forgottenCount: 0,
      currentAbortController: null,
      messageQueue: restoredQueue,
      isProcessingQueue: false,
      activeTools: [],
    };
  }
  return store.conversations[conversationId];
};

export const getInputState = (conversationId: string): InputState => {
  if (!store.inputs[conversationId]) {
    store.inputs[conversationId] = {
      draftMessage: "",
      attachments: [],
      isTyping: false,
    };
  }
  return store.inputs[conversationId];
};

// Actions
export const setCurrentProject = (projectId: string | null) => {
  store.currentProjectId = projectId;
};

export const setActiveConversation = (conversationId: string | null) => {
  store.activeConversationId = conversationId;
};

export const updateConversationMessages = (
  conversationId: string,
  messages: Message[]
) => {
  const state = getConversationState(conversationId);
  state.messages = deepClone(messages);
};

export const addMessage = (conversationId: string, message: Message) => {
  const state = getConversationState(conversationId);
  state.messages = [...state.messages, message];
};

export const updateLastMessage = (
  conversationId: string,
  updates: Partial<Message>
) => {
  const state = getConversationState(conversationId);
  if (state.messages.length > 0) {
    const lastMessage = state.messages[state.messages.length - 1];
    Object.assign(lastMessage, updates);
  }
};

export const setConversationLoading = (
  conversationId: string,
  isLoading: boolean
) => {
  const state = getConversationState(conversationId);
  state.isLoading = isLoading;
};

export const setConversationError = (
  conversationId: string,
  error: string | null
) => {
  const state = getConversationState(conversationId);
  state.error = error;
};

export const setConversationStreaming = (
  conversationId: string,
  isStreaming: boolean
) => {
  const state = getConversationState(conversationId);
  state.isStreaming = isStreaming;
};

export const setConversationUploadedFiles = (
  conversationId: string,
  files: any[]
) => {
  const state = getConversationState(conversationId);
  state.uploadedFiles = [...files];
};

export const addConversationUploadedFile = (
  conversationId: string,
  file: any
) => {
  const state = getConversationState(conversationId);
  state.uploadedFiles = [...state.uploadedFiles, file];
};

// Store AbortControllers outside of proxy to avoid "Illegal invocation" errors
const abortControllers = new Map<string, AbortController>();

export const setConversationAbortController = (
  conversationId: string,
  controller: AbortController | null
) => {
  const state = getConversationState(conversationId);

  // Store the controller in a separate Map to avoid proxy issues
  if (controller) {
    abortControllers.set(conversationId, controller);
    state.currentAbortController = controller as any; // Keep reference in state for reactivity
  } else {
    abortControllers.delete(conversationId);
    state.currentAbortController = null;
  }
};

export const getConversationAbortController = (
  conversationId: string
): AbortController | null => {
  return abortControllers.get(conversationId) || null;
};

export const setConversationForgotten = (
  conversationId: string,
  messageId: string | null,
  count: number = 0
) => {
  const state = getConversationState(conversationId);
  state.forgottenAfterMessageId = messageId;
  state.forgottenCount = count;
};

export const setConversationContext = (
  conversationId: string,
  context: ConversationContext | undefined
) => {
  const state = getConversationState(conversationId);
  state.conversationContext = context;
};

export const updateInputDraft = (conversationId: string, draft: string) => {
  const state = getInputState(conversationId);
  state.draftMessage = draft;
};

export const setInputAttachments = (
  conversationId: string,
  attachments: File[]
) => {
  const state = getInputState(conversationId);
  state.attachments = [...attachments];
};

export const addInputAttachment = (
  conversationId: string,
  attachment: File
) => {
  const state = getInputState(conversationId);
  state.attachments = [...state.attachments, attachment];
};

export const removeInputAttachment = (
  conversationId: string,
  index: number
) => {
  const state = getInputState(conversationId);
  state.attachments = state.attachments.filter((_, i) => i !== index);
};

export const setInputTyping = (conversationId: string, isTyping: boolean) => {
  const state = getInputState(conversationId);
  state.isTyping = isTyping;
};

export const cacheProjectContext = (
  projectId: string,
  context: ProjectContextResponse
) => {
  store.projectContexts[projectId] = context;
};

export const setSidebarOpen = (open: boolean) => {
  store.ui.sidebarOpen = open;
};

export const setFileSidebarOpen = (open: boolean) => {
  store.ui.fileSidebarOpen = open;
};

export const setActiveTab = (tab: string) => {
  store.ui.activeTab = tab;
};

export const setViewportHeight = (height: number) => {
  store.ui.viewportHeight = height;
};

export const setKeyboardHeight = (height: number) => {
  store.ui.keyboardHeight = height;
};

// Message queue management functions
export const addToMessageQueue = (
  conversationId: string,
  message: QueuedMessage
) => {
  const state = getConversationState(conversationId);
  state.messageQueue = [...state.messageQueue, message];
  // Save to sessionStorage
  saveQueueToStorage(conversationId, state.messageQueue);
};

export const removeFromMessageQueue = (
  conversationId: string,
  messageId: string
) => {
  const state = getConversationState(conversationId);
  state.messageQueue = state.messageQueue.filter((m) => m.id !== messageId);
  // Save to sessionStorage
  saveQueueToStorage(conversationId, state.messageQueue);
};

export const updateMessageInQueue = (
  conversationId: string,
  messageId: string,
  updates: Partial<QueuedMessage>
) => {
  const state = getConversationState(conversationId);
  state.messageQueue = state.messageQueue.map((m) =>
    m.id === messageId ? { ...m, ...updates } : m
  );
  // Save to sessionStorage
  saveQueueToStorage(conversationId, state.messageQueue);
};

export const clearMessageQueue = (conversationId: string) => {
  const state = getConversationState(conversationId);
  state.messageQueue = [];
  // Clear from sessionStorage
  sessionStorage.removeItem(`clay_queue_${conversationId}`);
};

export const setIsProcessingQueue = (
  conversationId: string,
  isProcessing: boolean
) => {
  const state = getConversationState(conversationId);
  state.isProcessingQueue = isProcessing;
};

// Active tools management
export const addActiveTool = (conversationId: string, tool: string) => {
  const state = getConversationState(conversationId);
  if (!state.activeTools.includes(tool)) {
    state.activeTools = [...state.activeTools, tool];
  }
};

export const removeActiveTool = (conversationId: string, tool: string) => {
  const state = getConversationState(conversationId);
  state.activeTools = state.activeTools.filter((t) => t !== tool);
};

export const clearActiveTools = (conversationId: string) => {
  const state = getConversationState(conversationId);
  state.activeTools = [];
};

// Helper to save queue to sessionStorage
const saveQueueToStorage = (conversationId: string, queue: QueuedMessage[]) => {
  if (queue.length > 0) {
    const serializable = queue.map((m) => ({
      id: m.id,
      content: m.content,
      timestamp: m.timestamp,
      hasFiles: m.files.length > 0,
    }));
    sessionStorage.setItem(
      `clay_queue_${conversationId}`,
      JSON.stringify(serializable)
    );
  } else {
    sessionStorage.removeItem(`clay_queue_${conversationId}`);
  }
};

// Cleanup function to remove unused conversation state (prevent memory leaks)
export const cleanupOldConversations = (activeConversationIds: string[]) => {
  const activeSet = new Set(activeConversationIds);

  // Keep only active conversations and the 10 most recent inactive ones
  const allConversationIds = Object.keys(store.conversations);
  const toRemove = allConversationIds
    .filter((id) => !activeSet.has(id))
    .slice(10); // Keep 10 most recent inactive conversations

  toRemove.forEach((id) => {
    delete store.conversations[id];
    delete store.inputs[id];
  });
};

// Cleanup function for a deleted conversation
export const cleanupDeletedConversation = (conversationId: string) => {
  // Remove conversation state
  delete store.conversations[conversationId];
  delete store.inputs[conversationId];

  // Clear active conversation if it matches
  if (store.activeConversationId === conversationId) {
    store.activeConversationId = null;
  }

  // Clear abort controller if exists
  abortControllers.delete(conversationId);

  // Clear session storage
  sessionStorage.removeItem(`clay_queue_${conversationId}`);
};

// Subscribe to store changes for debugging (only in development)
if (process.env.NODE_ENV === "development") {
  subscribe(store, () => {
    // You can add debugging here if needed
    // console.log('Store updated:', store)
  });
}
