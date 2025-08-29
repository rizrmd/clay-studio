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
      
      // Initialize new conversation state if needed, but ensure it starts fresh
      const state = getOrCreateConversationState(newConversationId);
      
      // IMPORTANT: Clear any stale messages if this is a new conversation
      // This prevents message bleeding from other conversations
      if (newConversationId === 'new' && state.messages.length > 0) {
        logger.warn('ConversationManager: Clearing stale messages from "new" conversation');
        state.messages = [];
        state.status = 'idle';
        state.error = null;
      }

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

  // Clear conversation state completely (for new chat creation)
  async clearConversation(conversationId: string): Promise<void> {
    return this.atomicOperation(conversationId, () => {
      const state = conversationStore.conversations[conversationId];
      if (!state) return;

      // Abort any ongoing requests
      abortControllerManager.abort(conversationId);

      // Clear all state
      state.messages = [];
      state.status = 'idle';
      state.error = null;
      state.uploadedFiles = [];
      state.forgottenAfterMessageId = null;
      state.forgottenCount = 0;
      state.messageQueue = [];
      state.activeTools = [];
      state.lastUpdated = Date.now();
      state.version++;
    });
  }

  // Update conversation status atomically
  async updateStatus(conversationId: string, status: ConversationStatus): Promise<void> {
    return this.atomicOperation(conversationId, () => {
      const state = getOrCreateConversationState(conversationId);
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

  // Helper function to update last assistant message (without atomic wrapper)
  private updateLastAssistantMessageInternal(state: any, updates: Partial<Message>, conversationId: string): void {
    if (state.messages.length > 0) {
      // Find the last ASSISTANT message, not just the last message
      // This prevents accidentally overwriting user messages
      for (let i = state.messages.length - 1; i >= 0; i--) {
        if (state.messages[i].role === 'assistant') {
          // CRITICAL SAFETY CHECK: Never update role field to prevent corruption
          if (updates.role && updates.role !== 'assistant') {
            logger.error('ConversationManager: Attempted to change assistant message role to', updates.role, '- blocking update');
            return;
          }
          
          // Apply updates safely
          Object.assign(state.messages[i], updates);
          state.lastUpdated = Date.now();
          state.version++;
          return;
        }
      }
      
      // If no assistant message found, log a warning with message details for debugging
      logger.warn('ConversationManager: No assistant message found to update in conversation', conversationId, 
        'Messages:', state.messages.map((m: Message) => `${m.role}:${m.id.substring(0,8)}`).join(', '));
    }
  }

  // Update message by ID atomically - ONLY updates ASSISTANT messages
  async updateMessageById(conversationId: string, messageId: string, updates: Partial<Message>): Promise<void> {
    return this.atomicOperation(conversationId, () => {
      const state = getOrCreateConversationState(conversationId);
      
      // Find message by ID
      const messageIndex = state.messages.findIndex(m => m.id === messageId);
      if (messageIndex >= 0) {
        const message = state.messages[messageIndex];
        
        // CRITICAL SAFETY CHECK: Only allow updating assistant messages
        if (message.role !== 'assistant') {
          logger.error('ConversationManager: Attempted to update non-assistant message', messageId, 'with role', message.role, '- blocking update');
          return;
        }
        
        // CRITICAL SAFETY CHECK: Never update role field to prevent corruption
        if (updates.role && updates.role !== 'assistant') {
          logger.error('ConversationManager: Attempted to change assistant message role to', updates.role, '- blocking update');
          return;
        }
        Object.assign(state.messages[messageIndex], updates);
        state.lastUpdated = Date.now();
        state.version++;
      } else {
        logger.warn('ConversationManager: Message not found by ID', messageId, 'in conversation', conversationId, '- falling back to update last assistant message');
        // Fallback: Update the last assistant message instead
        // This handles the case where the target message was filtered out but we still want to apply the completion data
        this.updateLastAssistantMessageInternal(state, updates, conversationId);
      }
    });
  }

  // Update last message atomically - ONLY updates the last ASSISTANT message
  async updateLastMessage(conversationId: string, updates: Partial<Message>): Promise<void> {
    return this.atomicOperation(conversationId, () => {
      const state = getOrCreateConversationState(conversationId);
      this.updateLastAssistantMessageInternal(state, updates, conversationId);
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
      if (!state.activeTools.includes(tool)) {
        state.activeTools.push(tool);
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

  // Update context usage information
  async updateContextUsage(conversationId: string, usage: import('./types').ContextUsageInfo): Promise<void> {
    return this.atomicOperation(conversationId, () => {
      const state = getOrCreateConversationState(conversationId);
      state.contextUsage = usage;
      logger.info('ConversationManager: Updated context usage', { conversationId, usage });
    });
  }

  // Get snapshot of conversation state (for debugging)
  getConversationSnapshot(conversationId: string): ConversationState | null {
    const state = conversationStore.conversations[conversationId];
    return state ? JSON.parse(JSON.stringify(snapshot(state))) : null;
  }
}