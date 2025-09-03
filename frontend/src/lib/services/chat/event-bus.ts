import { logger } from '@/lib/utils/logger';
import type { ConversationEvent } from '@/store/chat/types';

type EventHandler = (event: ConversationEvent) => void | Promise<void>;

class ChatEventBus {
  private handlers: Map<string, Set<EventHandler>> = new Map();
  private eventQueue: ConversationEvent[] = [];
  private isProcessing = false;

  subscribe(eventType: ConversationEvent['type'], handler: EventHandler): () => void {
    if (!this.handlers.has(eventType)) {
      this.handlers.set(eventType, new Set());
    }
    this.handlers.get(eventType)!.add(handler);

    // Return unsubscribe function
    return () => {
      this.handlers.get(eventType)?.delete(handler);
    };
  }

  async emit(event: ConversationEvent): Promise<void> {
    // Add to queue to ensure order
    this.eventQueue.push(event);
    
    if (!this.isProcessing) {
      await this.processQueue();
    }
  }

  private async processQueue(): Promise<void> {
    this.isProcessing = true;

    while (this.eventQueue.length > 0) {
      const event = this.eventQueue.shift()!;
      const handlers = this.handlers.get(event.type);
      
      if (handlers) {
        // Process handlers sequentially to maintain order
        for (const handler of handlers) {
          try {
            await handler(event);
          } catch (error) {
            logger.error(`Error handling event ${event.type}:`, error);
          }
        }
      }
    }

    this.isProcessing = false;
  }

  clear(): void {
    this.handlers.clear();
    this.eventQueue = [];
    this.isProcessing = false;
  }
}

export const chatEventBus = new ChatEventBus();