import { proxy } from 'valtio';

interface MessageUIState {
  // Alert states
  showNewMessageAlert: boolean;

  // Queue editing states
  editingQueuedId: string | null;
  editingContent: string;

  // Scroll states
  isAtBottom: boolean;

  // Initialization states
  isInitialized: boolean;
  showVirtuoso: boolean;
  isVirtuosoReady: boolean;

  // Per-conversation states
  conversationStates: Record<string, {
    showNewMessageAlert: boolean;
    isAtBottom: boolean;
  }>;
}

export const messageUIStore = proxy<MessageUIState>({
  // Alert states
  showNewMessageAlert: false,

  // Queue editing states
  editingQueuedId: null,
  editingContent: "",

  // Scroll states
  isAtBottom: true,

  // Initialization states
  isInitialized: false,
  showVirtuoso: false,
  isVirtuosoReady: false,

  // Per-conversation states
  conversationStates: {},
});

export const messageUIActions = {
  // Alert actions
  setShowNewMessageAlert: (show: boolean, conversationId?: string) => {
    if (conversationId) {
      if (!messageUIStore.conversationStates[conversationId]) {
        messageUIStore.conversationStates[conversationId] = {
          showNewMessageAlert: false,
          isAtBottom: true,
        };
      }
      messageUIStore.conversationStates[conversationId].showNewMessageAlert = show;
    } else {
      messageUIStore.showNewMessageAlert = show;
    }
  },

  // Queue editing actions
  startEditingQueued: (messageId: string, content: string) => {
    messageUIStore.editingQueuedId = messageId;
    messageUIStore.editingContent = content;
  },

  updateEditingContent: (content: string) => {
    messageUIStore.editingContent = content;
  },

  stopEditingQueued: () => {
    messageUIStore.editingQueuedId = null;
    messageUIStore.editingContent = "";
  },

  // Scroll actions
  setIsAtBottom: (isAtBottom: boolean, conversationId?: string) => {
    if (conversationId) {
      if (!messageUIStore.conversationStates[conversationId]) {
        messageUIStore.conversationStates[conversationId] = {
          showNewMessageAlert: false,
          isAtBottom: true,
        };
      }
      messageUIStore.conversationStates[conversationId].isAtBottom = isAtBottom;
    } else {
      messageUIStore.isAtBottom = isAtBottom;
    }
  },

  // Initialization actions
  setInitialized: (initialized: boolean) => {
    messageUIStore.isInitialized = initialized;
  },

  setShowVirtuoso: (show: boolean) => {
    messageUIStore.showVirtuoso = show;
  },

  setVirtuosoReady: (ready: boolean) => {
    messageUIStore.isVirtuosoReady = ready;
  },

  // Conversation-specific actions
  getConversationState: (conversationId: string) => {
    return messageUIStore.conversationStates[conversationId] || {
      showNewMessageAlert: false,
      isAtBottom: true,
    };
  },

  // Cleanup
  clearConversationState: (conversationId: string) => {
    delete messageUIStore.conversationStates[conversationId];
  },
};