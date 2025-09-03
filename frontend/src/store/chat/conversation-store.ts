import { proxy } from 'valtio';
import type { GlobalChatState, ConversationState, InputState } from './types';
import { CONVERSATION_STATES } from './constants';

// Initial state factory
const createInitialConversationState = (id: string): ConversationState => ({
  id,
  status: CONVERSATION_STATES.IDLE as 'idle',
  messages: [],
  error: null,
  uploadedFiles: [],
  forgottenAfterMessageId: null,
  forgottenCount: 0,
  messageQueue: [],
  activeTools: [],
  lastUpdated: Date.now(),
  version: 0,
});

// Main Valtio store
export const conversationStore = proxy<GlobalChatState>({
  currentProjectId: null,
  activeConversationId: null,
  conversations: {},
  projectContexts: {},
  pendingOperations: new Set(),
});

// Input state store (separate to avoid unnecessary re-renders)
export const inputStore = proxy<Record<string, InputState>>({});

// Helper to get or create conversation state
export const getOrCreateConversationState = (conversationId: string): ConversationState => {
  if (!conversationStore.conversations[conversationId]) {
    conversationStore.conversations[conversationId] = createInitialConversationState(conversationId);
  } else if (conversationId === 'new') {
    // Always reset 'new' conversation state to prevent stale data
    const freshState = createInitialConversationState(conversationId);
    conversationStore.conversations[conversationId] = freshState;
    console.debug('Reset "new" conversation state to prevent message bleeding');
  }
  return conversationStore.conversations[conversationId];
};

// Helper to get or create input state  
export const getOrCreateInputState = (conversationId: string): InputState => {
  if (!inputStore[conversationId]) {
    inputStore[conversationId] = {
      draftMessage: '',
      attachments: [],
      isTyping: false,
    };
  }
  return inputStore[conversationId];
};