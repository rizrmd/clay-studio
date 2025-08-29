import { logger } from '@/lib/logger';
import { ConversationManager } from '../../store/chat/conversation-manager';
import { conversationStore } from '../../store/chat/conversation-store';
import { chatEventBus } from './event-bus';
import type { Message } from '../../types/chat';

// WebSocket message types from server
interface ServerMessage {
  type: 'connected' | 'authentication_required' | 'subscribed' | 'pong' | 'start' | 'progress' | 'tool_use' | 'content' | 'complete' | 'error' | 'title_updated' | 'ask_user' | 'context_usage';
  // Connection fields
  user_id?: string;
  authenticated?: boolean;
  client_id?: string;
  role?: string;
  // Streaming fields
  id?: string;
  conversation_id?: string;
  content?: string;
  tool?: string;
  processing_time_ms?: number;
  tools_used?: string[];
  error?: string;
  project_id?: string;
  // Title update fields
  title?: string;
  // Ask user fields
  prompt_type?: string;
  options?: any[];
  input_type?: string;
  placeholder?: string;
  tool_use_id?: string;
  // Context usage fields
  total_chars?: number;
  max_chars?: number;
  percentage?: number;
  message_count?: number;
  needs_compaction?: boolean;
}

// WebSocket message types to server
interface ClientMessage {
  type: 'subscribe' | 'unsubscribe' | 'ping';
  project_id?: string;
  conversation_id?: string;
}

export class WebSocketService {
  private static instance: WebSocketService;
  private conversationManager: ConversationManager;
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;
  private currentProjectId: string | null = null;
  private currentConversationId: string | null = null;
  private messageQueue: ClientMessage[] = [];
  private isConnecting = false;
  private activeStreams = new Map<string, { content: string; messageId?: string }>();
  private isAuthenticated = false;
  private userInfo: { user_id?: string; client_id?: string; role?: string } | null = null;

  private constructor() {
    this.conversationManager = ConversationManager.getInstance();
  }

  static getInstance(): WebSocketService {
    if (!WebSocketService.instance) {
      WebSocketService.instance = new WebSocketService();
    }
    return WebSocketService.instance;
  }

  async connect(): Promise<void> {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      return;
    }

    if (this.isConnecting) {
      return;
    }

    this.isConnecting = true;

    try {
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsUrl = `${protocol}//${window.location.host}/api/ws`;
      
      logger.info('WebSocketService: Connecting to', wsUrl);
      
      this.ws = new WebSocket(wsUrl);

      this.ws.onopen = () => {
        logger.info('WebSocketService: WebSocket opened successfully');
        this.isConnecting = false;
        this.reconnectAttempts = 0;
        
        // Process queued messages
        this.processMessageQueue();
        
        // Note: Re-subscription will happen automatically when we receive the 'connected' message
        logger.info('WebSocketService: Waiting for authentication message from server...');
      };

      this.ws.onmessage = (event) => {
        this.handleMessage(event.data);
      };

      this.ws.onclose = (event) => {
        logger.info('WebSocketService: Connection closed', event.code, event.reason);
        this.isConnecting = false;
        this.ws = null;
        
        if (!event.wasClean && this.reconnectAttempts < this.maxReconnectAttempts) {
          this.scheduleReconnect();
        }
      };

      this.ws.onerror = (error) => {
        logger.error('WebSocketService: Connection error', error);
        this.isConnecting = false;
      };

      // Wait for connection to be established
      await new Promise<void>((resolve, reject) => {
        if (!this.ws) {
          reject(new Error('WebSocket not initialized'));
          return;
        }

        const handleOpen = () => {
          this.ws?.removeEventListener('open', handleOpen);
          this.ws?.removeEventListener('error', handleError);
          resolve();
        };

        const handleError = () => {
          this.ws?.removeEventListener('open', handleOpen);
          this.ws?.removeEventListener('error', handleError);
          reject(new Error('WebSocket connection failed'));
        };

        this.ws.addEventListener('open', handleOpen);
        this.ws.addEventListener('error', handleError);
      });
    } catch (error) {
      this.isConnecting = false;
      throw error;
    }
  }

  private scheduleReconnect(): void {
    this.reconnectAttempts++;
    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);
    
    logger.info(`WebSocketService: Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);
    
    setTimeout(() => {
      this.connect().catch((error) => {
        logger.error('WebSocketService: Reconnect failed', error);
      });
    }, delay);
  }

  private sendMessage(message: ClientMessage): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    } else {
      // Queue message for when connection is established
      this.messageQueue.push(message);
      
      // Try to connect if not already connecting
      if (!this.isConnecting) {
        this.connect().catch((error) => {
          logger.error('WebSocketService: Failed to connect for queued message', error);
        });
      }
    }
  }

  private processMessageQueue(): void {
    while (this.messageQueue.length > 0) {
      const message = this.messageQueue.shift();
      if (message && this.ws && this.ws.readyState === WebSocket.OPEN) {
        this.ws.send(JSON.stringify(message));
      }
    }
  }

  subscribe(projectId: string): void {
    this.currentProjectId = projectId;
    
    logger.info('WebSocketService: Subscribing to project', projectId);
    
    // Only send subscription if we're authenticated
    if (this.isAuthenticated) {
      this.sendMessage({
        type: 'subscribe',
        project_id: projectId,
      });
    } else {
      logger.warn('WebSocketService: Cannot subscribe - not authenticated. Will subscribe after authentication.');
    }
  }
  
  setCurrentConversation(conversationId: string): void {
    this.currentConversationId = conversationId === 'new' ? null : conversationId;
    logger.debug('WebSocketService: Current conversation set to', this.currentConversationId);
  }

  unsubscribe(): void {
    this.currentProjectId = null;
    this.currentConversationId = null;
    
    logger.info('WebSocketService: Unsubscribing from all streams');
    
    this.sendMessage({
      type: 'unsubscribe',
    });
  }

  private async handleMessage(data: string): Promise<void> {
    try {
      const message: ServerMessage = JSON.parse(data);
      
      logger.debug('WebSocketService: Received message', message.type, message);

      switch (message.type) {
        case 'connected':
          this.isAuthenticated = message.authenticated || false;
          this.userInfo = {
            user_id: message.user_id,
            client_id: message.client_id,
            role: message.role
          };
          logger.info('WebSocketService: Connected and authenticated', {
            user_id: message.user_id,
            authenticated: this.isAuthenticated,
            client_id: message.client_id,
            role: message.role
          });
          
          // Auto-subscribe to current project if we have one and we're authenticated
          if (this.isAuthenticated && this.currentProjectId) {
            this.subscribe(this.currentProjectId);
          }
          break;

        case 'authentication_required':
          this.isAuthenticated = false;
          this.userInfo = null;
          logger.warn('WebSocketService: Authentication required');
          break;

        case 'subscribed':
          logger.info('WebSocketService: Subscribed to project', message.project_id, 'conversation', message.conversation_id);
          break;

        case 'start':
          if (message.id && message.conversation_id) {
            await this.handleStartEvent(message.conversation_id, message.id);
          }
          break;

        case 'progress':
          if (message.content && message.conversation_id) {
            await this.handleProgressEvent(message.content, message.conversation_id);
          }
          break;

        case 'tool_use':
          if (message.tool && message.conversation_id) {
            await this.handleToolUseEvent(message.tool, message.conversation_id);
          }
          break;

        case 'content':
          if (message.content && message.conversation_id) {
            await this.handleContentEvent(message.content, message.conversation_id);
          }
          break;

        case 'complete':
          if (message.id && message.conversation_id) {
            await this.handleCompleteEvent(message);
          }
          break;

        case 'error':
          if (message.error && message.conversation_id) {
            await this.handleErrorEvent(message.error, message.conversation_id);
          }
          break;
        
        case 'title_updated':
          if (message.title && message.conversation_id) {
            await this.handleTitleUpdatedEvent(message.title, message.conversation_id);
          }
          break;

        case 'ask_user':
          if (message.conversation_id) {
            await this.handleAskUserEvent(message);
          }
          break;

        case 'context_usage':
          if (message.conversation_id) {
            await this.handleContextUsageEvent(message);
          }
          break;

        case 'pong':
          // Heartbeat response
          break;

        default:
          logger.warn('WebSocketService: Unknown message type', message.type);
      }
    } catch (error) {
      logger.error('WebSocketService: Failed to parse message', error);
    }
  }

  private async handleStartEvent(conversationId: string, messageId: string): Promise<void> {
    logger.debug('WebSocketService: Stream started for conversation', conversationId, 'message', messageId);
    
    // Initialize active stream
    this.activeStreams.set(conversationId, { content: '', messageId });
    
    // Update current conversation ID if we're streaming a new conversation
    if (!this.currentConversationId || this.currentConversationId === 'new') {
      this.currentConversationId = conversationId;
    }
    
    // Update conversation status
    await this.conversationManager.updateStatus(conversationId, 'streaming');
  }

  private async handleProgressEvent(content: string, conversationId: string): Promise<void> {
    const streamState = this.activeStreams.get(conversationId);
    if (!streamState) {
      // Initialize stream state if it doesn't exist (in case we missed the start event)
      this.activeStreams.set(conversationId, { content: '', messageId: `streaming-${Date.now()}` });
      await this.conversationManager.updateStatus(conversationId, 'streaming');
    }

    // The content is now the accumulated text directly, not JSON
    if (content) {
      const stream = this.activeStreams.get(conversationId)!;
      stream.content = content; // Use the full accumulated text from backend
      
      // Ensure conversation state exists
      if (!conversationStore.conversations[conversationId]) {
        // Initialize minimal state to receive messages
        await this.conversationManager.setMessages(conversationId, []);
      }
      
      // Update or create assistant message
      const state = conversationStore.conversations[conversationId];
      if (state) {
        if (state.messages.length > 0) {
          const lastMessage = state.messages[state.messages.length - 1];
          
          if (lastMessage.role === 'assistant') {
            await this.conversationManager.updateLastMessage(conversationId, {
              content: stream.content,
            });
          } else {
            // Create new assistant message
            const assistantMessage: Message = {
              id: stream.messageId || `streaming-${Date.now()}`,
              role: 'assistant',
              content: stream.content,
              createdAt: new Date().toISOString(),
            };
            await this.conversationManager.addMessage(conversationId, assistantMessage);
          }
        } else {
          // No messages yet, create the first assistant message
          const assistantMessage: Message = {
            id: stream.messageId || `streaming-${Date.now()}`,
            role: 'assistant',
            content: stream.content,
            createdAt: new Date().toISOString(),
          };
          await this.conversationManager.addMessage(conversationId, assistantMessage);
        }
      }
    }
  }

  private async handleToolUseEvent(tool: string, conversationId: string): Promise<void> {
    logger.info('WebSocketService: Tool use detected', tool, 'for conversation', conversationId);
    await this.conversationManager.addActiveTool(conversationId, tool);
    logger.info('WebSocketService: Added tool to active tools:', tool);
  }

  private async handleContentEvent(content: string, conversationId: string): Promise<void> {
    const state = conversationStore.conversations[conversationId];
    if (!state || state.messages.length === 0) return;

    const lastMessage = state.messages[state.messages.length - 1];
    
    if (lastMessage.role === 'assistant') {
      await this.conversationManager.updateLastMessage(conversationId, {
        content: content,
      });
    } else {
      // Create new assistant message
      const assistantMessage: Message = {
        id: `streaming-${Date.now()}`,
        role: 'assistant',
        content: content,
        createdAt: new Date().toISOString(),
      };
      await this.conversationManager.addMessage(conversationId, assistantMessage);
    }
  }

  private async handleCompleteEvent(message: ServerMessage): Promise<void> {
    const conversationId = message.conversation_id!;
    const messageId = message.id!;
    
    logger.debug('WebSocketService: Stream completed for conversation', conversationId, 'message', messageId);
    
    // Clean up active stream
    this.activeStreams.delete(conversationId);
    
    // Update current conversation ID if it was a new conversation
    if (this.currentConversationId === 'new' || !this.currentConversationId) {
      this.currentConversationId = conversationId;
    }
    
    // Update message with final data
    const updates = {
      clay_tools_used: message.tools_used && message.tools_used.length > 0 ? message.tools_used : undefined,
      processing_time_ms: message.processing_time_ms,
    };
    
    await this.conversationManager.updateMessageById(conversationId, messageId, updates);
    
    // Set status back to idle
    await this.conversationManager.updateStatus(conversationId, 'idle');
    
    // Clear active tools after a brief delay
    setTimeout(() => {
      this.conversationManager.clearActiveTools(conversationId);
    }, 100);
    
    // Emit completion event
    await chatEventBus.emit({
      type: 'MESSAGE_SENT',
      conversationId,
      messageId,
    });
  }

  private async handleErrorEvent(error: string, conversationId: string): Promise<void> {
    logger.error('WebSocketService: Stream error', error);
    
    // Clean up active stream
    this.activeStreams.delete(conversationId);
    
    // Set error state
    await this.conversationManager.setError(conversationId, error);
    await this.conversationManager.updateStatus(conversationId, 'idle');
    await this.conversationManager.clearActiveTools(conversationId);
  }

  private async handleTitleUpdatedEvent(title: string, conversationId: string): Promise<void> {
    logger.info('WebSocketService: Title updated for conversation', conversationId, 'to', title);
    
    // Emit event to update sidebar and other UI components
    await chatEventBus.emit({
      type: 'CONVERSATION_TITLE_UPDATED',
      conversationId,
      title,
    });
  }

  private async handleContextUsageEvent(message: ServerMessage): Promise<void> {
    const { conversation_id, total_chars, max_chars, percentage, message_count, needs_compaction } = message;
    
    if (!conversation_id) return;
    
    logger.info('WebSocketService: Context usage update', { 
      conversation_id,
      percentage,
      message_count,
      needs_compaction
    });
    
    // Update conversation context usage
    await this.conversationManager.updateContextUsage(conversation_id, {
      totalChars: total_chars || 0,
      maxChars: max_chars || 800000,
      percentage: percentage || 0,
      messageCount: message_count || 0,
      needsCompaction: needs_compaction || false,
    });
  }

  private async handleAskUserEvent(message: ServerMessage): Promise<void> {
    const { conversation_id, prompt_type, title, options, input_type, placeholder, tool_use_id } = message;
    
    if (!conversation_id) return;
    
    logger.info('WebSocketService: Ask user event received', { 
      conversation_id, 
      prompt_type,
      title 
    });
    
    // Update the last assistant message with ask_user data
    const state = conversationStore.conversations[conversation_id];
    if (state && state.messages.length > 0) {
      const lastMessage = state.messages[state.messages.length - 1];
      
      if (lastMessage.role === 'assistant') {
        await this.conversationManager.updateLastMessage(conversation_id, {
          ask_user: {
            prompt_type: prompt_type as "checkbox" | "buttons" | "input",
            title: title || '',
            options,
            input_type: input_type as "text" | "password" | undefined,
            placeholder,
            tool_use_id,
          }
        });
      }
    }
    
    // Emit event for UI to respond
    chatEventBus.emit({
      type: 'ASK_USER',
      conversationId: conversation_id,
      promptType: prompt_type,
      title,
      options,
      inputType: input_type,
      placeholder,
      toolUseId: tool_use_id,
    });
  }

  ping(): void {
    this.sendMessage({ type: 'ping' });
  }

  disconnect(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.currentProjectId = null;
    this.currentConversationId = null;
    this.messageQueue = [];
    this.activeStreams.clear();
    this.isAuthenticated = false;
    this.userInfo = null;
  }

  // Getters for authentication status
  get authenticated(): boolean {
    return this.isAuthenticated;
  }

  get user(): typeof this.userInfo {
    return this.userInfo;
  }
}