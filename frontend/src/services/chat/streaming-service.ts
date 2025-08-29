import { api } from '@/lib/api';
import { logger } from '@/lib/logger';
import { ConversationManager } from '../../store/chat/conversation-manager';
import { conversationStore } from '../../store/chat/conversation-store';
import { chatEventBus } from './event-bus';
import { abortControllerManager } from '../../utils/chat/abort-controller-manager';
import { WebSocketService } from './websocket-service';

interface StreamingOptions {
  projectId: string;
  conversationId: string;
  content: string;
  uploadedFilePaths?: string[];
  abortController: AbortController;
  isResend?: boolean;
}

export class StreamingService {
  private static instance: StreamingService;
  private conversationManager: ConversationManager;
  private wsService: WebSocketService;
  private activeStreams = new Map<string, boolean>();

  private constructor() {
    this.conversationManager = ConversationManager.getInstance();
    this.wsService = WebSocketService.getInstance();
  }

  static getInstance(): StreamingService {
    if (!StreamingService.instance) {
      StreamingService.instance = new StreamingService();
    }
    return StreamingService.instance;
  }

  async resumeStream(conversationId: string, projectId: string): Promise<void> {
    logger.info('StreamingService: Resuming stream for conversation:', conversationId);
    
    // With WebSocket, resuming is handled automatically by subscribing to the conversation
    // The WebSocket service will receive any active streaming events for this conversation
    try {
      // Connect will now handle authentication waiting internally
      await this.wsService.connect();
      
      if (!this.wsService.authenticated) {
        logger.error('StreamingService: WebSocket not authenticated after connect');
        throw new Error('WebSocket authentication failed');
      }
      
      this.wsService.subscribe(projectId);
      this.wsService.setCurrentConversation(conversationId);
    } catch (error: any) {
      logger.error('Resume: WebSocket connection/auth failed:', error);
      await this.conversationManager.setError(conversationId, error.message);
    }
  }

  async handleStream(options: StreamingOptions): Promise<void> {
    const { projectId, conversationId, content, abortController } = options;
    
    logger.debug('StreamingService: handleStream called for:', conversationId);
    
    // Check if already streaming for this conversation
    if (this.activeStreams.get(conversationId)) {
      logger.warn('StreamingService: Already streaming for conversation:', conversationId);
      return;
    }

    this.activeStreams.set(conversationId, true);
    let realConversationId = conversationId;

    try {
      // Ensure WebSocket connection is established and authenticated
      await this.wsService.connect();
      
      if (!this.wsService.authenticated) {
        logger.error('StreamingService: WebSocket not authenticated after connect');
        throw new Error('WebSocket authentication failed');
      }
      
      logger.debug('StreamingService: WebSocket authenticated successfully');
      
      // Subscribe to this project/conversation for streaming events
      this.wsService.subscribe(projectId);
      this.wsService.setCurrentConversation(conversationId);
      
      logger.debug('StreamingService: Initiating stream for:', conversationId);
      const response = await api.fetchStream('/chat/stream', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          messages: [
            {
              id: `msg-${Date.now()}`,
              role: 'user',
              content,
            },
          ],
          project_id: projectId,
          conversation_id: conversationId,
        }),
        signal: abortController.signal,
      });

      logger.info('StreamingService: Chat stream initiated, status:', response.status);

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      // Parse response to get conversation ID
      const result = await response.json();
      if (result.conversation_id && result.conversation_id !== conversationId) {
        realConversationId = await this.handleNewConversationTransition(
          conversationId,
          result.conversation_id,
          projectId
        );
        // Subscribe to the new conversation
        this.wsService.subscribe(projectId);
        this.wsService.setCurrentConversation(realConversationId);
      }
      
      // The actual streaming will be handled by WebSocket events
      logger.debug('StreamingService: Stream initiated successfully, WebSocket will handle events');
      
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') {
        logger.info('StreamingService: Stream aborted for:', conversationId);
        await this.conversationManager.updateStatus(realConversationId, 'idle');
      } else {
        logger.error('StreamingService: Stream error for:', conversationId, error);
        await this.conversationManager.setError(realConversationId, error instanceof Error ? error.message : 'Streaming failed');
        await this.conversationManager.updateStatus(realConversationId, 'idle');
        throw error;
      }
    } finally {
      this.activeStreams.delete(conversationId);
      this.activeStreams.delete(realConversationId);
      
      logger.debug('StreamingService: handleStream completed for:', realConversationId);
    }
  }


  private async handleNewConversationTransition(
    oldId: string,
    newId: string,
    projectId: string
  ): Promise<string> {
    logger.info('StreamingService: Transitioning from new to:', newId);
    
    // Get current state from 'new'
    const oldState = conversationStore.conversations[oldId];
    if (!oldState) return newId;

    // Copy messages to new conversation
    await this.conversationManager.setMessages(newId, [...oldState.messages]);
    
    // Set status to 'streaming' to ensure loading indicator shows during transition
    await this.conversationManager.updateStatus(newId, 'streaming');
    if (oldState.error) {
      await this.conversationManager.setError(newId, oldState.error);
    }
    
    // Transfer abort controller from old to new conversation ID
    abortControllerManager.transfer(oldId, newId);
    
    // Clear the 'new' conversation state
    await this.conversationManager.setMessages(oldId, []);
    await this.conversationManager.updateStatus(oldId, 'idle');
    await this.conversationManager.clearQueue(oldId);
    
    // Switch active conversation
    await this.conversationManager.switchConversation(newId);
    
    // Emit event for UI updates
    await chatEventBus.emit({
      type: 'CONVERSATION_CREATED',
      conversationId: newId,
      projectId,
    });

    return newId;
  }



}