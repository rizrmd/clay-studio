import { logger } from '@/lib/utils/logger';
import { ConversationManager } from '@/store/chat/conversation-manager';
import { conversationStore } from '@/store/chat/conversation-store';
import { chatEventBus } from './event-bus';
import { MessageCacheService } from './message-cache';
import { uiActions } from '@/store/ui-store';
import type { Message } from '@/types/chat';

// WebSocket message types from server
interface ServerMessage {
  type: 'connected' | 'authentication_required' | 'subscribed' | 'conversation_history' | 'conversation_redirect' | 'pong' | 'start' | 'progress' | 'tool_use' | 'tool_complete' | 'content' | 'complete' | 'error' | 'title_updated' | 'ask_user' | 'context_usage';
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
  tool_usages?: any[];
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
  // Tool complete fields  
  tool_usage_id?: string;  // For ToolUse and ToolComplete events
  execution_time_ms?: number;
  output?: any;
  // Context usage fields
  total_chars?: number;
  max_chars?: number;
  percentage?: number;
  message_count?: number;
  needs_compaction?: boolean;
  // Conversation redirect fields
  old_conversation_id?: string;
  new_conversation_id?: string;
  // Conversation history fields
  messages?: Message[];
}

// WebSocket message types to server
interface ClientMessage {
  type: 'subscribe' | 'unsubscribe' | 'ping' | 'ask_user_response' | 'stop_streaming' | 'send_message';
  project_id?: string;
  conversation_id?: string;
  // Ask user response fields
  interaction_id?: string;
  response?: string | string[];
  // Send message fields
  content?: string;
  uploaded_file_paths?: string[];
}

export class WebSocketService {
  private static instance: WebSocketService;
  private conversationManager: ConversationManager;
  private messageCache: MessageCacheService;
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
  private authenticationPromise: Promise<void> | null = null;
  private authenticationResolver: (() => void) | null = null;
  private isSubscribed = false;

  private constructor() {
    this.conversationManager = ConversationManager.getInstance();
    this.messageCache = MessageCacheService.getInstance();
  }

  static getInstance(): WebSocketService {
    if (!WebSocketService.instance) {
      WebSocketService.instance = new WebSocketService();
    }
    return WebSocketService.instance;
  }

  async connect(): Promise<void> {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      // Already connected, wait for authentication if needed
      if (!this.isAuthenticated && this.authenticationPromise) {
        await this.authenticationPromise;
      }
      return;
    }

    if (this.isConnecting && this.authenticationPromise) {
      // Already connecting, wait for it to complete
      await this.authenticationPromise;
      return;
    }

    this.isConnecting = true;
    
    // Create authentication promise with timeout for mobile environments
    this.authenticationPromise = new Promise<void>((resolve, _reject) => {
      this.authenticationResolver = resolve;
      
      // Add timeout to prevent hanging on mobile
      const timeout = setTimeout(() => {
        if (!this.isAuthenticated) {
          logger.warn('WebSocketService: Authentication timeout after 5 seconds');
          resolve(); // Resolve anyway to prevent blocking
        }
      }, 5000);
      
      // Store original resolver to clear timeout
      const originalResolver = this.authenticationResolver;
      this.authenticationResolver = () => {
        clearTimeout(timeout);
        if (originalResolver) originalResolver();
      };
    });

    try {
      // Use relative URL to ensure Vite proxy handles WebSocket connections correctly
      let wsUrl = `/api/ws`;
      
      // First try without session parameter to use standard cookie auth
      // Only add session parameter if cookies aren't working (fallback)
      
      // For browsers that might have issues with cookies in WebSocket, try session token
      // But only if we're authenticated (check for session cookie)
      const hasCookie = document.cookie.includes('clay_session');
      if (hasCookie) {
        try {
          const response = await fetch('/api/auth/session-token', {
            credentials: 'include'
          });
          
          if (response.ok) {
            const data = await response.json();
            if (data.session_token) {
              // Only use session token if needed (we'll try cookie first)
              wsUrl = `${wsUrl}?session=${encodeURIComponent(data.session_token)}`;
            }
          } else if (response.status === 401) {
            logger.warn('WebSocketService: Session token endpoint returned 401, likely not authenticated');
          }
        } catch (error) {
        }
      } else {
      }
      
      
      this.ws = new WebSocket(wsUrl);

      this.ws.onopen = () => {
        this.isConnecting = false;
        this.reconnectAttempts = 0;
        
        // Process queued messages
        this.processMessageQueue();
        
        // Note: Re-subscription will happen automatically when we receive the 'connected' message
      };

      this.ws.onmessage = (event) => {
        this.handleMessage(event.data);
      };

        this.ws.onclose = (event) => {
          this.isConnecting = false;
          this.ws = null;
          this.isSubscribed = false; // Reset subscription status on disconnect
          uiActions.setWsSubscribed(false); // Update UI store

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

  subscribe(projectId: string, conversationId?: string): void {
    // Use conversation ID as provided
    const effectiveConversationId = conversationId || null;
    
    // Update current subscription state
    this.currentProjectId = projectId;
    this.currentConversationId = effectiveConversationId;
    this.isSubscribed = false; // Reset until we get confirmation
    uiActions.setWsSubscribed(false); // Update UI store
    
    // Only send subscription if we're authenticated
    if (this.isAuthenticated) {
      this.sendMessage({
        type: 'subscribe',
        project_id: projectId,
        conversation_id: effectiveConversationId || undefined,
      });
      // Mark as pending subscription (will be confirmed by 'subscribed' message)
    } else {
      logger.warn('WebSocketService: Cannot subscribe - not authenticated. Will retry after authentication.');
      // Store project ID to subscribe after authentication
      this.currentProjectId = projectId;
      this.currentConversationId = effectiveConversationId;
    }
  }
  
  setCurrentConversation(conversationId: string): void {
    this.currentConversationId = conversationId === 'new' ? null : conversationId;
  }

  sendAskUserResponse(interactionId: string, response: string | string[]): void {
    if (!this.currentConversationId) {
      logger.error('WebSocketService: Cannot send ask_user response - no active conversation');
      return;
    }

    this.sendMessage({
      type: 'ask_user_response',
      conversation_id: this.currentConversationId,
      interaction_id: interactionId,
      response: response,
    });
  }
  
  stopStreaming(conversationId: string): void {
    
    this.sendMessage({
      type: 'stop_streaming',
      conversation_id: conversationId,
    });
  }

  sendChatMessage(
    projectId: string,
    conversationId: string, 
    content: string,
    uploadedFilePaths?: string[]
  ): void {
    const message = {
      type: 'send_message' as const,
      project_id: projectId,
      conversation_id: conversationId,
      content: content,
      uploaded_file_paths: uploadedFilePaths || undefined,
    };
    
    this.sendMessage(message);
  }

  unsubscribe(): void {
    this.currentProjectId = null;
    this.currentConversationId = null;
    this.isSubscribed = false;
    uiActions.setWsSubscribed(false); // Update UI store
    
    
    this.sendMessage({
      type: 'unsubscribe',
    });
  }

  private async handleMessage(data: string): Promise<void> {
    try {
      const message: ServerMessage = JSON.parse(data);
      

      switch (message.type) {
        case 'connected':
          this.isAuthenticated = message.authenticated || false;
          this.userInfo = {
            user_id: message.user_id,
            client_id: message.client_id,
            role: message.role
          };
          
          // Resolve authentication promise
          if (this.authenticationResolver) {
            this.authenticationResolver();
            this.authenticationResolver = null;
          }
          
          // Auto-subscribe to current project and conversation if we have them and we're authenticated
          // Only if we're not already subscribed (to prevent duplicates on reconnection)
          // Don't subscribe if the current conversation is 'new' (which would be null)
          if (this.isAuthenticated && this.currentProjectId && !this.isSubscribed && this.currentConversationId !== null) {
            const convId = this.currentConversationId;
            this.subscribe(this.currentProjectId, convId);
          }
          break;

        case 'authentication_required':
          this.isAuthenticated = false;
          this.userInfo = null;
          logger.warn('WebSocketService: Authentication required');
          
          // Resolve authentication promise (even though not authenticated)
          if (this.authenticationResolver) {
            this.authenticationResolver();
            this.authenticationResolver = null;
          }
          break;

        case 'subscribed':
          this.isSubscribed = true;
          uiActions.setWsSubscribed(true);
          break;

        case 'conversation_history':
          if (message.messages && message.conversation_id) {
            await this.handleConversationHistory(message.conversation_id, message.messages);
          }
          break;

        case 'conversation_redirect':
          if (message.old_conversation_id && message.new_conversation_id) {
            await this.handleConversationRedirect(message.old_conversation_id, message.new_conversation_id);
          }
          break;

        case 'start':
          if (message.id && message.conversation_id) {
            await this.handleStartEvent(message.conversation_id, message.id);
          }
          break;

        case 'progress':
          if (message.content && message.conversation_id) {
            await this.handleProgressEvent(message.content, message.conversation_id);
          } else {
            logger.warn('WebSocketService: Progress event missing content or conversation_id', message);
          }
          break;

        case 'tool_use':
          if (message.tool && message.conversation_id) {
            await this.handleToolUseEvent(message.tool, message.tool_usage_id, message.conversation_id);
          }
          break;

        case 'tool_complete':
          if (message.tool && message.conversation_id) {
            await this.handleToolCompleteEvent(message);
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

  private async handleConversationHistory(conversationId: string, messages: Message[]): Promise<void> {
    
    // Cache the messages for instant access later
    await this.messageCache.cacheMessages(conversationId, messages);
    
    // Check if we have an active stream with a thinking message
    const streamState = this.activeStreams.get(conversationId);
    const hasActiveStream = !!streamState;
    
    if (hasActiveStream && streamState.messageId) {
      // We're in the middle of streaming - preserve the thinking message
      const thinkingMessage: Message = {
        id: streamState.messageId,
        role: 'assistant',
        content: streamState.content || '💭 Thinking...',
        createdAt: new Date().toISOString(),
      };
      
      // Add thinking message after the history
      await this.conversationManager.setMessages(conversationId, [...messages, thinkingMessage]);
      
      // Keep streaming status
      await this.conversationManager.updateStatus(conversationId, 'streaming');
    } else {
      // No active stream, just set the messages normally
      await this.conversationManager.setMessages(conversationId, messages);
      
      // Update conversation status to idle since we have all the messages
      await this.conversationManager.updateStatus(conversationId, 'idle');
    }
  }

  private async handleConversationRedirect(oldConversationId: string, newConversationId: string): Promise<void> {
    
    // Transfer stream state if it exists (but there likely won't be one yet for "new")
    const streamState = this.activeStreams.get(oldConversationId);
    if (streamState) {
      this.activeStreams.set(newConversationId, streamState);
      this.activeStreams.delete(oldConversationId);
    }
    
    // Transfer conversation state from old to new ID
    const oldState = conversationStore.conversations[oldConversationId];
    if (oldState) {
      // Copy the entire conversation state to the new ID, ensuring status is preserved
      conversationStore.conversations[newConversationId] = { 
        ...oldState,
        id: newConversationId, // Update the ID
        status: 'streaming' // Ensure we're in streaming status, not loading
      };
      
      // Keep the old state around for a while to handle any race conditions
      // Components might still be referencing it during the transition
      setTimeout(() => {
        if (conversationStore.conversations[oldConversationId]) {
          delete conversationStore.conversations[oldConversationId];
        }
      }, 500); // Increased delay to 500ms
    } else {
      // If no old state exists, create a new one with the user message
      logger.warn('WebSocketService: No old state to transfer, creating new state for', newConversationId);
      conversationStore.conversations[newConversationId] = {
        id: newConversationId,
        status: 'streaming',
        messages: [],
        error: null,
        uploadedFiles: [],
        forgottenAfterMessageId: null,
        forgottenCount: 0,
        messageQueue: [],
        activeTools: [],
        lastUpdated: Date.now(),
        version: 0,
      };
    }
    
    // Update the active conversation ID in the store
    if (conversationStore.activeConversationId === oldConversationId || conversationStore.activeConversationId === 'new') {
      conversationStore.activeConversationId = newConversationId;
    }
    
    // No need to pre-initialize stream state - it will be set when Start event arrives
    
    // Update current conversation ID
    if (this.currentConversationId === oldConversationId || this.currentConversationId === null) {
      this.currentConversationId = newConversationId;
    }
    
    // Update subscription tracking - we're now subscribed to the new conversation
    // The backend has already updated our subscription, so we just need to track it locally
    if (this.currentProjectId) {
      this.isSubscribed = true; // We remain subscribed, just to a different conversation
      uiActions.setWsSubscribed(true); // Update UI store
    }
    
    // Ensure the conversation manager knows about the new conversation
    await this.conversationManager.updateStatus(newConversationId, 'streaming');
    
    // Emit event to update URL and UI
    await chatEventBus.emit({
      type: 'CONVERSATION_REDIRECT',
      oldConversationId,
      newConversationId,
    });
  }

  private async handleStartEvent(conversationId: string, messageId: string): Promise<void> {
    
    // Import setConversationAbortController at the top of the file if not already imported
    const { setConversationAbortController } = await import('@/store/chat-store');
    
    // Create an abort controller for this streaming session
    // This enables the stop button to appear in the UI
    const abortController = new AbortController();
    setConversationAbortController(conversationId, abortController);
    
    // Initialize stream state with the message ID from start event
    // Don't add a thinking message here - the Messages component shows its own loading indicator
    this.activeStreams.set(conversationId, { content: '', messageId });
    
    // Before starting a new stream, ensure any previous executing tools are marked complete
    const state = conversationStore.conversations[conversationId];
    if (state && state.messages.length > 0) {
      // Check all messages for executing tools and mark them as complete
      for (let i = 0; i < state.messages.length; i++) {
        const msg = state.messages[i];
        if (msg.tool_usages && msg.tool_usages.some(tu => tu.output?.status === 'executing')) {
          const updatedToolUsages = msg.tool_usages.map(tu => {
            if (tu.output?.status === 'executing') {
              return {
                ...tu,
                output: { status: 'completed', result: 'Tool execution completed' }
              };
            }
            return tu;
          });
          await this.conversationManager.updateMessageById(conversationId, msg.id, {
            tool_usages: updatedToolUsages
          });
        }
      }
    }
    
    // Update current conversation ID if we're streaming a new conversation
    if (!this.currentConversationId || this.currentConversationId === 'new') {
      this.currentConversationId = conversationId;
    }
    
    // Update conversation status
    await this.conversationManager.updateStatus(conversationId, 'streaming');
  }

  private async handleProgressEvent(content: any, conversationId: string): Promise<void> {
    
    const streamState = this.activeStreams.get(conversationId);
    if (!streamState) {
      // Initialize stream state if it doesn't exist (in case we missed the start event)
      this.activeStreams.set(conversationId, { content: '', messageId: `streaming-${Date.now()}` });
      await this.conversationManager.updateStatus(conversationId, 'streaming');
    }

    // Extract text content from the JSON structure
    let textContent = '';
    let isIncremental = false; // Track if this is incremental text that should be appended
    let todoWriteData = null; // Track TodoWrite updates
    
    if (content) {
      if (typeof content === 'string') {
        textContent = content;
      } else if (typeof content === 'object') {
        // Extract text from various message types
        if (content.type === 'result' && content.result) {
          textContent = content.result;
        } else if (content.type === 'assistant' && content.message?.content) {
          // Extract text from assistant message content
          const messageContent = content.message.content;
          if (Array.isArray(messageContent)) {
            // Extract text blocks and check for TodoWrite
            for (const block of messageContent) {
              if (block.type === 'text') {
                textContent += block.text || '';
              } else if (block.type === 'tool_use' && block.name === 'TodoWrite') {
                // Extract TodoWrite data
                todoWriteData = block.input || block.arguments;
              }
            }
          } else if (typeof messageContent === 'string') {
            textContent = messageContent;
          }
        } else if (content.type === 'text' && content.text) {
          textContent = content.text;
        } else if (content.type === 'content_block_delta' && content.delta?.text) {
          // Delta messages are incremental - they should be appended
          textContent = content.delta.text;
          isIncremental = true;
        } else if (content.type === 'show_table' || content.type === 'show_chart') {
          // For visualization events, pass the entire JSON as a stringified content
          // The frontend components will parse and render these appropriately
          textContent = JSON.stringify(content);
        }
        // Skip other message types that don't contain displayable text
      }
    }

    if (textContent || todoWriteData) {
      const stream = this.activeStreams.get(conversationId)!;
      
      // Handle text content
      if (textContent) {
        // For assistant messages during streaming, always append to show incremental progress
        // Only replace content for non-incremental deltas or single-shot responses
        if (isIncremental || content.type === 'assistant') {
          stream.content = (stream.content || '') + textContent;
        } else {
          stream.content = textContent;
        }
      }
      
      // Ensure conversation state exists
      if (!conversationStore.conversations[conversationId]) {
        // Initialize minimal state to receive messages
        await this.conversationManager.setMessages(conversationId, []);
      }
      
      // Update or create assistant message
      const state = conversationStore.conversations[conversationId];
      if (state) {
        // Look for an existing assistant message with the same ID
        const existingMessageIndex = state.messages.findIndex(
          msg => msg.role === 'assistant' && msg.id === stream.messageId
        );
        
        if (existingMessageIndex !== -1) {
          // Update the existing message
          const updates: any = {};
          if (textContent) {
            updates.content = stream.content;
          }
          if (todoWriteData) {
            updates.todoWrite = todoWriteData;
          }
          
          // Update the specific message by index
          state.messages[existingMessageIndex] = {
            ...state.messages[existingMessageIndex],
            ...updates
          };
        } else if (state.messages.length > 0) {
          const lastMessage = state.messages[state.messages.length - 1];
          
          if (lastMessage.role === 'assistant' && !lastMessage.id.startsWith('thinking-')) {
            // Update the last assistant message if it doesn't have a specific ID
            const updates: any = {};
            if (textContent) {
              updates.content = stream.content;
            }
            if (todoWriteData) {
              updates.todoWrite = todoWriteData;
            }
            await this.conversationManager.updateLastMessage(conversationId, updates);
          } else {
            // Create new assistant message only if we don't have one for this stream
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

  private async handleToolUseEvent(tool: string, toolUsageId: string | undefined, conversationId: string): Promise<void> {
    
    // Use the provided tool_usage_id or generate a temporary one
    const effectiveId = toolUsageId || `temp-${Date.now()}-${Math.random().toString(36).substring(2, 11)}`;
    await this.conversationManager.addActiveTool(conversationId, tool, effectiveId);
    
    // Create the new tool usage with the actual ID from backend
    const toolUsage = {
      id: effectiveId,
      message_id: '',  // Will be set by addToolUsage
      tool_name: tool,
      parameters: null,
      output: { status: 'executing' }, // Mark as executing during stream
      createdAt: new Date().toISOString(),
    };
    
    // Add to tool_usages atomically
    await this.conversationManager.addToolUsage(conversationId, toolUsage);
  }

  private async handleToolCompleteEvent(message: ServerMessage): Promise<void> {
    const { tool, tool_usage_id, execution_time_ms, output, conversation_id } = message;
    
    if (!conversation_id || !tool || !tool_usage_id) return;
    
    
    // Update the specific tool usage to completed status
    const state = conversationStore.conversations[conversation_id];
    if (state && state.messages.length > 0) {
      const lastMessage = state.messages[state.messages.length - 1];
      if (lastMessage.role === 'assistant' && lastMessage.tool_usages) {
        let toolUpdated = false;
        
        // Find and update the specific tool by ID
        const updatedToolUsages = lastMessage.tool_usages.map(tu => {
          // Match by tool_usage_id first, fallback to matching by tool name and executing status
          if (tu.id === tool_usage_id || (!toolUpdated && tu.tool_name === tool && tu.output?.status === 'executing')) {
            toolUpdated = true;
            return {
              ...tu,
              id: tool_usage_id, // Ensure we have the correct ID
              output: output || { status: 'completed', result: 'Tool execution completed' },
              execution_time_ms: execution_time_ms,
            };
          }
          return tu;
        });
        
        if (toolUpdated) {
          await this.conversationManager.updateLastMessage(conversation_id, {
            tool_usages: updatedToolUsages,
          });
          
        } else {
          logger.warn('WebSocketService: Could not find executing tool to update:', tool);
        }
      }
    }
    
    // Update the tool status in activeTools to completed
    await this.conversationManager.updateToolCompleted(
      conversation_id, 
      tool, 
      tool_usage_id,
      execution_time_ms || 0
    );
    
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
    
    
    // Clear abort controller since streaming is complete
    const { setConversationAbortController } = await import('@/store/chat-store');
    setConversationAbortController(conversationId, null);
    
    // Clean up active stream
    this.activeStreams.delete(conversationId);
    
    // Update current conversation ID if it was a new conversation
    if (this.currentConversationId === 'new' || !this.currentConversationId) {
      this.currentConversationId = conversationId;
    }
    
    // Update message with final data from backend
    // ALWAYS mark executing tools as complete when message completes
    const state = conversationStore.conversations[conversationId];
    let finalToolUsages = message.tool_usages || [];
    
    if (state && state.messages.length > 0) {
      const lastMessage = state.messages[state.messages.length - 1];
      
      if (lastMessage.tool_usages && lastMessage.tool_usages.length > 0) {
        // Always mark our tracked tools as complete since the message is done
        const completedTools = lastMessage.tool_usages.map(tu => {
          if (tu.output?.status === 'executing') {
            return {
              ...tu,
              output: { status: 'completed', result: 'Tool execution completed' }
            };
          }
          return tu;
        });
        
        if (finalToolUsages.length > 0) {
          // Merge backend data with our completed tracking
          const backendToolMap = new Map();
          finalToolUsages.forEach(tu => {
            backendToolMap.set(tu.tool_name, tu);
          });
          
          // Use backend data where available, otherwise use our completed versions
          finalToolUsages = completedTools.map(tool => {
            return backendToolMap.get(tool.tool_name) || tool;
          });
        } else {
          // No backend data - use our completed tools
          finalToolUsages = completedTools;
        }
      }
    }
    
    const updates = {
      tool_usages: finalToolUsages,
      processing_time_ms: message.processing_time_ms,
    };
    
    await this.conversationManager.updateMessageById(conversationId, messageId, updates);
    
    // Update cache with the final message state
    if (state && state.messages) {
      await this.messageCache.cacheMessages(conversationId, state.messages);
    }
    
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
    
    // Clear abort controller since streaming has stopped due to error
    const { setConversationAbortController } = await import('@/store/chat-store');
    setConversationAbortController(conversationId, null);
    
    // Clean up active stream
    this.activeStreams.delete(conversationId);
    
    // Set error state
    await this.conversationManager.setError(conversationId, error);
    await this.conversationManager.updateStatus(conversationId, 'idle');
    await this.conversationManager.clearActiveTools(conversationId);
  }

  private async handleTitleUpdatedEvent(title: string, conversationId: string): Promise<void> {
    
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
    this.authenticationPromise = null;
    this.authenticationResolver = null;
    this.isSubscribed = false;
    uiActions.setWsSubscribed(false); // Update UI store
  }

  // Force reconnect with new authentication
  async reconnect(): Promise<void> {
    this.disconnect();
    // Wait a bit for cleanup
    await new Promise(resolve => setTimeout(resolve, 100));
    return this.connect();
  }

  // Getters for authentication status
  get authenticated(): boolean {
    return this.isAuthenticated;
  }

  get user(): typeof this.userInfo {
    return this.userInfo;
  }

  // Getter for subscription status
  get subscribed(): boolean {
    return this.isSubscribed;
  }
}