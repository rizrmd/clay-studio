import { proxy, subscribe } from 'valtio'
import { Message, ConversationContext, ProjectContextResponse } from '../hooks/use-clay-chat'

// Types for our global state
export interface ConversationState {
  messages: Message[]
  isLoading: boolean
  isLoadingMessages: boolean
  error: string | null
  isStreaming: boolean
  uploadedFiles: any[]
  forgottenAfterMessageId: string | null
  forgottenCount: number
  hasStartedNewConversation: boolean
  currentAbortController: AbortController | null
  conversationContext?: ConversationContext
}

export interface InputState {
  draftMessage: string
  attachments: File[]
  isTyping: boolean
}

export interface UIState {
  sidebarOpen: boolean
  fileSidebarOpen: boolean
  activeTab: string
}

export interface GlobalState {
  // Project and auth state
  currentProjectId: string | null
  activeConversationId: string | null
  
  // Chat state keyed by conversation ID
  conversations: Record<string, ConversationState>
  
  // Input state keyed by conversation ID
  inputs: Record<string, InputState>
  
  // Project context cache
  projectContexts: Record<string, ProjectContextResponse>
  
  // UI state
  ui: UIState
}

// Helper function to deep clone objects to avoid readonly issues
const deepClone = <T>(obj: T): T => {
  if (obj === null || typeof obj !== 'object') return obj;
  if (obj instanceof Date) return new Date(obj.getTime()) as T;
  if (Array.isArray(obj)) return obj.map(item => deepClone(item)) as T;
  
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
    activeTab: 'chat'
  }
}

// Create the proxy state
export const store = proxy(initialState)

// Helper functions to manage conversation state
export const getConversationState = (conversationId: string): ConversationState => {
  if (!store.conversations[conversationId]) {
    store.conversations[conversationId] = {
      messages: [],
      isLoading: false,
      isLoadingMessages: false,
      error: null,
      isStreaming: false,
      uploadedFiles: [],
      forgottenAfterMessageId: null,
      forgottenCount: 0,
      hasStartedNewConversation: false,
      currentAbortController: null
    }
  }
  return store.conversations[conversationId]
}

export const getInputState = (conversationId: string): InputState => {
  if (!store.inputs[conversationId]) {
    store.inputs[conversationId] = {
      draftMessage: '',
      attachments: [],
      isTyping: false
    }
  }
  return store.inputs[conversationId]
}

// Actions
export const setCurrentProject = (projectId: string | null) => {
  store.currentProjectId = projectId
}

export const setActiveConversation = (conversationId: string | null) => {
  store.activeConversationId = conversationId
}

export const updateConversationMessages = (conversationId: string, messages: Message[]) => {
  const state = getConversationState(conversationId)
  state.messages = deepClone(messages)
}

export const addMessage = (conversationId: string, message: Message) => {
  const state = getConversationState(conversationId)
  state.messages = [...state.messages, message]
}

export const updateLastMessage = (conversationId: string, updates: Partial<Message>) => {
  const state = getConversationState(conversationId)
  if (state.messages.length > 0) {
    const lastMessage = state.messages[state.messages.length - 1]
    Object.assign(lastMessage, updates)
  }
}

export const setConversationLoading = (conversationId: string, isLoading: boolean) => {
  const state = getConversationState(conversationId)
  state.isLoading = isLoading
}

export const setConversationError = (conversationId: string, error: string | null) => {
  const state = getConversationState(conversationId)
  state.error = error
}

export const setConversationStreaming = (conversationId: string, isStreaming: boolean) => {
  const state = getConversationState(conversationId)
  state.isStreaming = isStreaming
}

export const setConversationUploadedFiles = (conversationId: string, files: any[]) => {
  const state = getConversationState(conversationId)
  state.uploadedFiles = [...files]
}

export const addConversationUploadedFile = (conversationId: string, file: any) => {
  const state = getConversationState(conversationId)
  state.uploadedFiles = [...state.uploadedFiles, file]
}

// Store AbortControllers outside of proxy to avoid "Illegal invocation" errors
const abortControllers = new Map<string, AbortController>()

export const setConversationAbortController = (conversationId: string, controller: AbortController | null) => {
  const state = getConversationState(conversationId)
  
  // Store the controller in a separate Map to avoid proxy issues
  if (controller) {
    abortControllers.set(conversationId, controller)
    state.currentAbortController = controller as any // Keep reference in state for reactivity
  } else {
    abortControllers.delete(conversationId)
    state.currentAbortController = null
  }
}

export const getConversationAbortController = (conversationId: string): AbortController | null => {
  return abortControllers.get(conversationId) || null
}

export const setConversationForgotten = (conversationId: string, messageId: string | null, count: number = 0) => {
  const state = getConversationState(conversationId)
  state.forgottenAfterMessageId = messageId
  state.forgottenCount = count
}

export const setConversationStarted = (conversationId: string, started: boolean) => {
  const state = getConversationState(conversationId)
  state.hasStartedNewConversation = started
}

export const setConversationContext = (conversationId: string, context: ConversationContext | undefined) => {
  const state = getConversationState(conversationId)
  state.conversationContext = context
}

export const updateInputDraft = (conversationId: string, draft: string) => {
  const state = getInputState(conversationId)
  state.draftMessage = draft
}

export const setInputAttachments = (conversationId: string, attachments: File[]) => {
  const state = getInputState(conversationId)
  state.attachments = [...attachments]
}

export const addInputAttachment = (conversationId: string, attachment: File) => {
  const state = getInputState(conversationId)
  state.attachments = [...state.attachments, attachment]
}

export const removeInputAttachment = (conversationId: string, index: number) => {
  const state = getInputState(conversationId)
  state.attachments = state.attachments.filter((_, i) => i !== index)
}

export const setInputTyping = (conversationId: string, isTyping: boolean) => {
  const state = getInputState(conversationId)
  state.isTyping = isTyping
}

export const cacheProjectContext = (projectId: string, context: ProjectContextResponse) => {
  store.projectContexts[projectId] = context
}

export const setSidebarOpen = (open: boolean) => {
  store.ui.sidebarOpen = open
}

export const setFileSidebarOpen = (open: boolean) => {
  store.ui.fileSidebarOpen = open
}

export const setActiveTab = (tab: string) => {
  store.ui.activeTab = tab
}

// Cleanup function to remove unused conversation state (prevent memory leaks)
export const cleanupOldConversations = (activeConversationIds: string[]) => {
  const activeSet = new Set(activeConversationIds)
  
  // Keep only active conversations and the 10 most recent inactive ones
  const allConversationIds = Object.keys(store.conversations)
  const toRemove = allConversationIds
    .filter(id => !activeSet.has(id))
    .slice(10) // Keep 10 most recent inactive conversations
    
  toRemove.forEach(id => {
    delete store.conversations[id]
    delete store.inputs[id]
  })
}

// Subscribe to store changes for debugging (only in development)
if (process.env.NODE_ENV === 'development') {
  subscribe(store, () => {
    // You can add debugging here if needed
    // console.log('Store updated:', store)
  })
}