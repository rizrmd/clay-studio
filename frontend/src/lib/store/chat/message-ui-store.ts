import { proxy } from "valtio";

export interface MessageUIState {
  expandedMessages: Record<string, boolean>;
  isGenerating: boolean;
  streamingMessageId: string | null;
  isInitialized: boolean;
  isVirtuosoReady: boolean;
  conversationStates: Record<
    string,
    {
      isAtBottom: boolean;
      showNewMessageAlert: boolean;
      previousConversationId?: string;
      previousMessageCount: number;
      scrollHeight: number;
    }
  >;
  virtuosoRef: any; // Will store the VirtuosoHandle
  timeouts: {
    scrollTimeout: NodeJS.Timeout | null;
    contentStableTimeout: NodeJS.Timeout | null;
    scrollStabilityCheckTimeout: NodeJS.Timeout | null;
  };
}

export const messageUIStore = proxy<MessageUIState>({
  expandedMessages: {},
  isGenerating: false,
  streamingMessageId: null,
  isInitialized: false,
  isVirtuosoReady: false,
  conversationStates: {},
  virtuosoRef: null,
  timeouts: {
    scrollTimeout: null,
    contentStableTimeout: null,
    scrollStabilityCheckTimeout: null,
  },
});

export const messageUIActions = {
  toggleMessageExpansion: (messageId: string) => {
    messageUIStore.expandedMessages[messageId] =
      !messageUIStore.expandedMessages[messageId];
  },

  setMessageExpanded: (messageId: string, expanded: boolean) => {
    messageUIStore.expandedMessages[messageId] = expanded;
  },

  setGenerating: (isGenerating: boolean) => {
    messageUIStore.isGenerating = isGenerating;
  },

  setStreamingMessageId: (messageId: string | null) => {
    messageUIStore.streamingMessageId = messageId;
  },

  setInitialized: (initialized: boolean) => {
    messageUIStore.isInitialized = initialized;
  },

  setVirtuosoReady: (ready: boolean) => {
    messageUIStore.isVirtuosoReady = ready;
  },

  setShowNewMessageAlert: (show: boolean, conversationId?: string) => {
    if (!conversationId) return;

    if (!messageUIStore.conversationStates[conversationId]) {
      messageUIStore.conversationStates[conversationId] = {
        isAtBottom: true,
        showNewMessageAlert: false,
        previousMessageCount: 0,
        scrollHeight: 0,
      };
    }
    messageUIStore.conversationStates[conversationId].showNewMessageAlert =
      show;
  },

  setIsAtBottom: (isAtBottom: boolean, conversationId?: string) => {
    if (!conversationId) return;

    if (!messageUIStore.conversationStates[conversationId]) {
      messageUIStore.conversationStates[conversationId] = {
        isAtBottom: true,
        showNewMessageAlert: false,
        previousMessageCount: 0,
        scrollHeight: 0,
      };
    }
    messageUIStore.conversationStates[conversationId].isAtBottom = isAtBottom;
  },

  setVirtuosoRef: (ref: any) => {
    messageUIStore.virtuosoRef = ref;
  },

  setPreviousConversationId: (conversationId: string, prevId?: string) => {
    if (!messageUIStore.conversationStates[conversationId]) {
      messageUIStore.conversationStates[conversationId] = {
        isAtBottom: true,
        showNewMessageAlert: false,
        previousMessageCount: 0,
        scrollHeight: 0,
      };
    }
    messageUIStore.conversationStates[conversationId].previousConversationId =
      prevId;
  },

  setPreviousMessageCount: (conversationId: string, count: number) => {
    if (!messageUIStore.conversationStates[conversationId]) {
      messageUIStore.conversationStates[conversationId] = {
        isAtBottom: true,
        showNewMessageAlert: false,
        previousMessageCount: 0,
        scrollHeight: 0,
      };
    }
    messageUIStore.conversationStates[conversationId].previousMessageCount =
      count;
  },

  setScrollHeight: (conversationId: string, height: number) => {
    if (!messageUIStore.conversationStates[conversationId]) {
      messageUIStore.conversationStates[conversationId] = {
        isAtBottom: true,
        showNewMessageAlert: false,
        previousMessageCount: 0,
        scrollHeight: 0,
      };
    }
    messageUIStore.conversationStates[conversationId].scrollHeight = height;
  },

  scrollToBottom: (
    behavior: "smooth" | "auto" = "auto",
    itemsLength: number
  ) => {
    // Cancel any pending scroll operations
    if (messageUIStore.timeouts.scrollTimeout) {
      clearTimeout(messageUIStore.timeouts.scrollTimeout);
      messageUIStore.timeouts.scrollTimeout = null;
    }

    if (messageUIStore.virtuosoRef) {
      const lastIndex = itemsLength - 1;
      if (lastIndex >= 0) {
        messageUIStore.virtuosoRef.scrollToIndex({
          index: lastIndex,
          behavior,
          align: "end",
        });
        // Also call scrollTo to ensure we're at the absolute bottom
        messageUIStore.timeouts.scrollTimeout = setTimeout(() => {
          messageUIStore.virtuosoRef?.scrollTo({
            top: Number.MAX_SAFE_INTEGER,
            behavior,
          });
          messageUIStore.timeouts.scrollTimeout = null;
        }, 50);
      }
    }
  },

  clearTimeouts: () => {
    if (messageUIStore.timeouts.scrollTimeout) {
      clearTimeout(messageUIStore.timeouts.scrollTimeout);
      messageUIStore.timeouts.scrollTimeout = null;
    }
    if (messageUIStore.timeouts.contentStableTimeout) {
      clearTimeout(messageUIStore.timeouts.contentStableTimeout);
      messageUIStore.timeouts.contentStableTimeout = null;
    }
    if (messageUIStore.timeouts.scrollStabilityCheckTimeout) {
      clearTimeout(messageUIStore.timeouts.scrollStabilityCheckTimeout);
      messageUIStore.timeouts.scrollStabilityCheckTimeout = null;
    }
  },

  setContentStableTimeout: (timeout: NodeJS.Timeout | null) => {
    if (messageUIStore.timeouts.contentStableTimeout) {
      clearTimeout(messageUIStore.timeouts.contentStableTimeout);
    }
    messageUIStore.timeouts.contentStableTimeout = timeout;
  },

  setScrollStabilityCheckTimeout: (timeout: NodeJS.Timeout | null) => {
    if (messageUIStore.timeouts.scrollStabilityCheckTimeout) {
      clearTimeout(messageUIStore.timeouts.scrollStabilityCheckTimeout);
    }
    messageUIStore.timeouts.scrollStabilityCheckTimeout = timeout;
  },

  getConversationState: (conversationId?: string) => {
    if (!conversationId) {
      return {
        isGenerating: messageUIStore.isGenerating,
        streamingMessageId: messageUIStore.streamingMessageId,
        isAtBottom: true,
        showNewMessageAlert: false,
        previousMessageCount: 0,
        scrollHeight: 0,
        previousConversationId: undefined,
      };
    }

    const conversationState = messageUIStore.conversationStates[conversationId];
    return {
      isGenerating: messageUIStore.isGenerating,
      streamingMessageId: messageUIStore.streamingMessageId,
      isAtBottom: conversationState?.isAtBottom ?? true,
      showNewMessageAlert: conversationState?.showNewMessageAlert ?? false,
      previousMessageCount: conversationState?.previousMessageCount ?? 0,
      scrollHeight: conversationState?.scrollHeight ?? 0,
      previousConversationId: conversationState?.previousConversationId,
    };
  },

  reset: () => {
    messageUIActions.clearTimeouts();
    messageUIStore.expandedMessages = {};
    messageUIStore.isGenerating = false;
    messageUIStore.streamingMessageId = null;
    messageUIStore.isInitialized = false;
    messageUIStore.isVirtuosoReady = false;
    messageUIStore.conversationStates = {};
    messageUIStore.virtuosoRef = null;
  },
};
