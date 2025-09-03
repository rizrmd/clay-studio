import { logger } from '@/lib/utils/logger';
import type { Message } from '@/types/chat';

interface CachedConversation {
  messages: Message[];
  timestamp: number;
  version: number;
  lastMessageId?: string;
}

interface CacheMetadata {
  conversationIds: string[];
  totalSize: number;
  lastCleanup: number;
}

export class MessageCacheService {
  private static instance: MessageCacheService;
  private readonly CACHE_PREFIX = 'clay_msg_cache_';
  private readonly METADATA_KEY = 'clay_cache_metadata';
  private readonly MAX_CACHE_SIZE = 50 * 1024 * 1024; // 50MB
  private readonly MAX_CONVERSATIONS = 50;
  private readonly CACHE_TTL = 60 * 60 * 1000; // 1 hour
  private readonly CLEANUP_INTERVAL = 5 * 60 * 1000; // 5 minutes
  
  private memoryCache: Map<string, CachedConversation> = new Map();
  private lastCleanup = 0;

  private constructor() {
    // Initialize cleanup interval
    this.scheduleCleanup();
    
    // Load metadata on startup
    this.loadMetadata();
    
    // Listen for storage events from other tabs
    window.addEventListener('storage', this.handleStorageEvent.bind(this));
  }

  static getInstance(): MessageCacheService {
    if (!MessageCacheService.instance) {
      MessageCacheService.instance = new MessageCacheService();
    }
    return MessageCacheService.instance;
  }

  // Save messages to cache
  async cacheMessages(conversationId: string, messages: Message[]): Promise<void> {
    try {
      const cached: CachedConversation = {
        messages: messages,
        timestamp: Date.now(),
        version: Date.now(), // Use timestamp as version
        lastMessageId: messages.length > 0 ? messages[messages.length - 1].id : undefined
      };

      // Update memory cache first (instant)
      this.memoryCache.set(conversationId, cached);

      // Compress and store in localStorage (async)
      const key = this.CACHE_PREFIX + conversationId;
      const serialized = JSON.stringify(cached);
      
      // Check size before storing
      if (serialized.length > 5 * 1024 * 1024) { // Skip if > 5MB per conversation
        logger.warn(`MessageCache: Conversation ${conversationId} too large to cache (${serialized.length} bytes)`);
        return;
      }

      localStorage.setItem(key, serialized);
      
      // Update metadata
      await this.updateMetadata(conversationId);
      
      logger.debug(`MessageCache: Cached ${messages.length} messages for ${conversationId}`);
    } catch (error) {
      // Handle QuotaExceededError
      if (error instanceof DOMException && error.name === 'QuotaExceededError') {
        logger.warn('MessageCache: Storage quota exceeded, running cleanup');
        await this.cleanup();
        // Try once more after cleanup
        try {
          const key = this.CACHE_PREFIX + conversationId;
          localStorage.setItem(key, JSON.stringify({
            messages: messages,
            timestamp: Date.now(),
            version: Date.now()
          }));
        } catch {
          logger.error('MessageCache: Failed to cache after cleanup');
        }
      } else {
        logger.error('MessageCache: Failed to cache messages', error);
      }
    }
  }

  // Retrieve messages from cache
  getCachedMessages(conversationId: string): Message[] | null {
    // Check memory cache first
    const memCached = this.memoryCache.get(conversationId);
    if (memCached && this.isValid(memCached)) {
      logger.debug(`MessageCache: Memory cache hit for ${conversationId}`);
      return memCached.messages;
    }

    // Fall back to localStorage
    try {
      const key = this.CACHE_PREFIX + conversationId;
      const cached = localStorage.getItem(key);
      
      if (!cached) {
        return null;
      }

      const parsed: CachedConversation = JSON.parse(cached);
      
      // Check if cache is still valid
      if (!this.isValid(parsed)) {
        logger.debug(`MessageCache: Cache expired for ${conversationId}`);
        this.invalidate(conversationId);
        return null;
      }

      // Update memory cache
      this.memoryCache.set(conversationId, parsed);
      
      logger.debug(`MessageCache: Disk cache hit for ${conversationId} (${parsed.messages.length} messages)`);
      return parsed.messages;
    } catch (error) {
      logger.error('MessageCache: Failed to retrieve cached messages', error);
      return null;
    }
  }

  // Update cached messages incrementally
  async updateCachedMessages(
    conversationId: string, 
    updater: (messages: Message[]) => Message[]
  ): Promise<void> {
    const cached = this.getCachedMessages(conversationId);
    if (cached) {
      const updated = updater(cached);
      await this.cacheMessages(conversationId, updated);
    }
  }

  // Invalidate cache for a conversation
  invalidate(conversationId: string): void {
    this.memoryCache.delete(conversationId);
    localStorage.removeItem(this.CACHE_PREFIX + conversationId);
    logger.debug(`MessageCache: Invalidated cache for ${conversationId}`);
  }

  // Clear all caches
  clearAll(): void {
    this.memoryCache.clear();
    
    // Clear localStorage
    const keys = Object.keys(localStorage);
    keys.forEach(key => {
      if (key.startsWith(this.CACHE_PREFIX)) {
        localStorage.removeItem(key);
      }
    });
    
    localStorage.removeItem(this.METADATA_KEY);
    logger.info('MessageCache: Cleared all caches');
  }

  // Check if cache is still valid
  private isValid(cached: CachedConversation): boolean {
    const age = Date.now() - cached.timestamp;
    return age < this.CACHE_TTL;
  }

  // Load metadata from localStorage
  private loadMetadata(): CacheMetadata {
    try {
      const stored = localStorage.getItem(this.METADATA_KEY);
      if (stored) {
        return JSON.parse(stored);
      }
    } catch (error) {
      logger.error('MessageCache: Failed to load metadata', error);
    }
    
    return {
      conversationIds: [],
      totalSize: 0,
      lastCleanup: Date.now()
    };
  }

  // Update metadata after caching
  private async updateMetadata(conversationId: string): Promise<void> {
    const metadata = this.loadMetadata();
    
    // Update conversation list (LRU order)
    const index = metadata.conversationIds.indexOf(conversationId);
    if (index > -1) {
      metadata.conversationIds.splice(index, 1);
    }
    metadata.conversationIds.unshift(conversationId);
    
    // Limit number of cached conversations
    if (metadata.conversationIds.length > this.MAX_CONVERSATIONS) {
      const toRemove = metadata.conversationIds.slice(this.MAX_CONVERSATIONS);
      toRemove.forEach(id => this.invalidate(id));
      metadata.conversationIds = metadata.conversationIds.slice(0, this.MAX_CONVERSATIONS);
    }
    
    // Calculate total size
    metadata.totalSize = this.calculateTotalSize();
    
    localStorage.setItem(this.METADATA_KEY, JSON.stringify(metadata));
  }

  // Calculate total cache size
  private calculateTotalSize(): number {
    let total = 0;
    const keys = Object.keys(localStorage);
    
    keys.forEach(key => {
      if (key.startsWith(this.CACHE_PREFIX)) {
        const item = localStorage.getItem(key);
        if (item) {
          total += item.length * 2; // Approximate bytes (UTF-16)
        }
      }
    });
    
    return total;
  }

  // Cleanup old and oversized caches
  private async cleanup(): Promise<void> {
    const now = Date.now();
    
    // Skip if cleaned up recently
    if (now - this.lastCleanup < this.CLEANUP_INTERVAL) {
      return;
    }
    
    this.lastCleanup = now;
    logger.debug('MessageCache: Running cleanup');
    
    const metadata = this.loadMetadata();
    const keys = Object.keys(localStorage);
    let removed = 0;
    
    // Remove expired caches
    keys.forEach(key => {
      if (key.startsWith(this.CACHE_PREFIX)) {
        try {
          const item = localStorage.getItem(key);
          if (item) {
            const cached: CachedConversation = JSON.parse(item);
            if (!this.isValid(cached)) {
              localStorage.removeItem(key);
              const conversationId = key.replace(this.CACHE_PREFIX, '');
              this.memoryCache.delete(conversationId);
              removed++;
            }
          }
        } catch {
          // Remove corrupted entries
          localStorage.removeItem(key);
          removed++;
        }
      }
    });
    
    // If still over size limit, remove oldest conversations
    const totalSize = this.calculateTotalSize();
    if (totalSize > this.MAX_CACHE_SIZE) {
      const toRemove = Math.ceil(metadata.conversationIds.length * 0.2); // Remove 20%
      const oldest = metadata.conversationIds.slice(-toRemove);
      oldest.forEach(id => this.invalidate(id));
      removed += oldest.length;
    }
    
    if (removed > 0) {
      logger.info(`MessageCache: Cleaned up ${removed} cached conversations`);
    }
    
    metadata.lastCleanup = now;
    localStorage.setItem(this.METADATA_KEY, JSON.stringify(metadata));
  }

  // Schedule periodic cleanup
  private scheduleCleanup(): void {
    setInterval(() => {
      this.cleanup().catch(error => {
        logger.error('MessageCache: Cleanup failed', error);
      });
    }, this.CLEANUP_INTERVAL);
  }

  // Handle storage events from other tabs
  private handleStorageEvent(event: StorageEvent): void {
    if (!event.key || !event.key.startsWith(this.CACHE_PREFIX)) {
      return;
    }
    
    const conversationId = event.key.replace(this.CACHE_PREFIX, '');
    
    if (event.newValue) {
      // Cache updated in another tab
      try {
        const cached: CachedConversation = JSON.parse(event.newValue);
        this.memoryCache.set(conversationId, cached);
        logger.debug(`MessageCache: Synced cache for ${conversationId} from another tab`);
      } catch {
        // Ignore parse errors
      }
    } else {
      // Cache cleared in another tab
      this.memoryCache.delete(conversationId);
    }
  }

  // Prefetch messages for a conversation
  async prefetch(conversationId: string): Promise<void> {
    // This will be called by WebSocket when receiving conversation_history
    // The actual fetching happens via WebSocket, we just ensure it's cached
    logger.debug(`MessageCache: Prefetch requested for ${conversationId}`);
  }

  // Get cache info for debugging
  getCacheInfo(): {
    memoryCount: number;
    diskCount: number;
    totalSize: number;
    conversationIds: string[];
  } {
    const metadata = this.loadMetadata();
    return {
      memoryCount: this.memoryCache.size,
      diskCount: metadata.conversationIds.length,
      totalSize: metadata.totalSize,
      conversationIds: metadata.conversationIds
    };
  }
}