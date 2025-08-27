import { snapshot } from 'valtio';
import { logger } from '@/lib/logger';
import { conversationStore, getOrCreateConversationState } from './conversation-store';
import { chatEventBus } from '../../services/chat/event-bus';
import { abortControllerManager } from '../../utils/chat/abort-controller-manager';
import type { ConversationState, QueuedMessage, ConversationStatus } from './types';
import type { Message } from '../../types/chat';
import { CONVERSATION_CACHE_SIZE } from './constants';

export class ConversationManager {
  private static instance: ConversationManager;
  private operationLocks: Map<string, Promise<void>> = new Map();

  private constructor() {
    // Singleton pattern
  }

  static getInstance(): ConversationManager {
    if (!ConversationManager.instance) {
      ConversationManager.instance = new ConversationManager();
    }
    return ConversationManager.instance;
  }

  // Atomic operation wrapper to prevent race conditions
  private async atomicOperation<T>(
    conversationId: string,
    operation: () => T | Promise<T>
  ): Promise<T> {
    // Wait for any pending operations on this conversation
    const pendingOp = this.operationLocks.get(conversationId);
    if (pendingOp) {
      await pendingOp;
    }

    // Create new operation promise
    let resolve: () => void;
    const opPromise = new Promise<void>((res) => { resolve = res; });
    this.operationLocks.set(conversationId, opPromise);

    try {
      const result = await operation();
      return result;
    } finally {
      this.operationLocks.delete(conversationId);
      resolve!();
    }
  }

  // Switch active conversation with proper cleanup
  async switchConversation(newConversationId: string): Promise<void> {
    return this.atomicOperation(newConversationId, async () => {
      const previousId = conversationStore.activeConversationId;
      
      // Don't switch if already active
      if (previousId === newConversationId) {
        return;
      }

      // Clean up previous conversation
      if (previousId) {
        await this.cleanupConversation(previousId);
      }

      // Set new active conversation
      conversationStore.activeConversationId = newConversationId;
      
      // Initialize new conversation state if needed
      getOrCreateConversationState(newConversationId);

      // Emit event
      await chatEventBus.emit({
        type: 'CONVERSATION_SWITCHED',
        from: previousId,
        to: newConversationId,
      });

      // Clean up old conversations if cache is full
      this.pruneConversationCache();
    });
  }

  // Clean up conversation resources
  private async cleanupConversation(conversationId: string): Promise<void> {
    // Abort any ongoing requests
    abortControllerManager.abort(conversationId);

    const state = conversationStore.conversations[conversationId];
    if (!state) return;

    // Clear active tools and pending operations
    state.activeTools = [];
    state.status = 'idle';
    
    // Don't clear messages or queue - keep them for when user returns
  }

  // Update conversation status atomically
  async updateStatus(conversationId: string, status: ConversationStatus): Promise<void> {
    return this.atomicOperation(conversationId, () => {
      const state = getOrCreateConversationState(conversationId);
      logger.debug('ConversationManager: Updating status:', conversationId, 'from', state.status, 'to', status);
      if (status === 'idle' && state.status === 'streaming') {
        console.trace('[ConversationManager] Setting streaming to idle - stack trace:');
      }
      state.status = status;
      state.lastUpdated = Date.now();
      state.version++;
    });
  }

  // Add message atomically
  async addMessage(conversationId: string, message: Message): Promise<void> {
    return this.atomicOperation(conversationId, () => {
      const state = getOrCreateConversationState(conversationId);
      
      // Check for duplicates
      const exists = state.messages.some(m => m.id === message.id);
      if (!exists) {
        state.messages.push(message);
        state.lastUpdated = Date.now();
        state.version++;
      }
    });
  }

  // Update last message atomically
  async updateLastMessage(conversationId: string, updates: Partial<Message>): Promise<void> {
    return this.atomicOperation(conversationId, () => {
      const state = getOrCreateConversationState(conversationId);
      
      if (state.messages.length > 0) {
        const lastMessage = state.messages[state.messages.length - 1];
        Object.assign(lastMessage, updates);
        state.lastUpdated = Date.now();
        state.version++;
      }
    });
  }

  // Replace all messages (for loading from API)
  async setMessages(conversationId: string, messages: Message[]): Promise<void> {
    return this.atomicOperation(conversationId, () => {
      const state = getOrCreateConversationState(conversationId);
      state.messages = [...messages];
      state.lastUpdated = Date.now();
      state.version++;
    });
  }

  // Queue management with deduplication
  async addToQueue(conversationId: string, message: QueuedMessage): Promise<void> {
    return this.atomicOperation(conversationId, () => {
      const state = getOrCreateConversationState(conversationId);
      
      // Check for duplicate content in queue
      const isDuplicate = state.messageQueue.some(
        m => m.content === message.content && 
        Math.abs(m.timestamp.getTime() - message.timestamp.getTime()) < 1000
      );
      
      if (!isDuplicate) {
        state.messageQueue.push(message);
        state.lastUpdated = Date.now();
        state.version++;
      }
    });
  }

  // Remove from queue
  async removeFromQueue(conversationId: string, messageId: string): Promise<void> {
    return this.atomicOperation(conversationId, () => {
      const state = getOrCreateConversationState(conversationId);
      state.messageQueue = state.messageQueue.filter(m => m.id !== messageId);
      state.lastUpdated = Date.now();
      state.version++;
    });
  }

  // Get next queued message
  async getNextQueuedMessage(conversationId: string): Promise<QueuedMessage | null> {
    return this.atomicOperation(conversationId, () => {
      const state = getOrCreateConversationState(conversationId);
      
      logger.debug('ConversationManager: getNextQueuedMessage - status:', state.status, 'queue length:', state.messageQueue.length);
      
      // Only process queue if not streaming and no errors
      if (state.status !== 'idle' || state.messageQueue.length === 0) {
        return null;
      }
      
      // Get and remove first message
      const message = state.messageQueue.shift();
      if (message) {
        state.lastUpdated = Date.now();
        state.version++;
      }
      
      return message || null;
    });
  }

  // Clear queue
  async clearQueue(conversationId: string): Promise<void> {
    return this.atomicOperation(conversationId, () => {
      const state = getOrCreateConversationState(conversationId);
      state.messageQueue = [];
      state.lastUpdated = Date.now();
      state.version++;
    });
  }

  // Set error state
  async setError(conversationId: string, error: string | null): Promise<void> {
    return this.atomicOperation(conversationId, async () => {
      const state = getOrCreateConversationState(conversationId);
      state.error = error;
      // Only change status if there's an error, not when clearing it
      if (error) {
        state.status = 'error';
      }
      state.lastUpdated = Date.now();
      state.version++;

      if (error) {
        await chatEventBus.emit({
          type: 'ERROR_OCCURRED',
          conversationId,
          error,
        });
      }
    });
  }

  // Handle active tools
  async addActiveTool(conversationId: string, tool: string): Promise<void> {
    return this.atomicOperation(conversationId, () => {
      const state = getOrCreateConversationState(conversationId);
      logger.debug('ConversationManager: Adding active tool:', tool, 'to conversation:', conversationId);
      if (!state.activeTools.includes(tool)) {
        state.activeTools.push(tool);
        logger.debug('ConversationManager: Active tools now:', state.activeTools);
        state.lastUpdated = Date.now();
        state.version++;
      }
    });
  }

  async clearActiveTools(conversationId: string): Promise<void> {
    return this.atomicOperation(conversationId, () => {
      const state = getOrCreateConversationState(conversationId);
      state.activeTools = [];
      state.lastUpdated = Date.now();
      state.version++;
    });
  }

  // Handle forgotten messages
  async setForgottenState(
    conversationId: string,
    messageId: string | null,
    count: number
  ): Promise<void> {
    return this.atomicOperation(conversationId, () => {
      const state = getOrCreateConversationState(conversationId);
      state.forgottenAfterMessageId = messageId;
      state.forgottenCount = count;
      state.lastUpdated = Date.now();
      state.version++;
    });
  }

  // Clean up old conversations to prevent memory leaks
  private pruneConversationCache(): void {
    const conversations = Object.entries(conversationStore.conversations);
    
    if (conversations.length <= CONVERSATION_CACHE_SIZE) {
      return;
    }

    // Sort by last updated, keep most recent
    conversations.sort((a, b) => b[1].lastUpdated - a[1].lastUpdated);
    
    const toRemove = conversations.slice(CONVERSATION_CACHE_SIZE);
    for (const [id] of toRemove) {
      // Don't remove active conversation
      if (id !== conversationStore.activeConversationId) {
        delete conversationStore.conversations[id];
        abortControllerManager.abort(id);
      }
    }
  }

  // Reset entire store (for logout, etc)
  async reset(): Promise<void> {
    // Abort all operations
    abortControllerManager.abortAll();
    
    // Clear all conversations
    conversationStore.conversations = {};
    conversationStore.currentProjectId = null;
    conversationStore.activeConversationId = null;
    conversationStore.projectContexts = {};
    conversationStore.pendingOperations.clear();
    
    // Clear operation locks
    this.operationLocks.clear();
    
    // Clear event bus
    chatEventBus.clear();
  }

  // Get snapshot of conversation state (for debugging)
  getConversationSnapshot(conversationId: string): ConversationState | null {
    const state = conversationStore.conversations[conversationId];
    return state ? JSON.parse(JSON.stringify(snapshot(state))) : null;
  }
}