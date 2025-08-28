import { api } from '@/lib/api';
import { logger } from '@/lib/logger';
import { ConversationManager } from '../../store/chat/conversation-manager';
import { conversationStore } from '../../store/chat/conversation-store';
import { chatEventBus } from './event-bus';
import { abortControllerManager } from '../../utils/chat/abort-controller-manager';
import { StreamingService } from './streaming-service';
import type { Message } from '../../types/chat';
import type { QueuedMessage } from '../../store/chat/types';

export class MessageService {
  private static instance: MessageService;
  private conversationManager: ConversationManager;
  private streamingService: StreamingService;
  private sendingMessages = new Set<string>(); // Track messages being sent

  private constructor() {
    this.conversationManager = ConversationManager.getInstance();
    this.streamingService = StreamingService.getInstance();
  }

  static getInstance(): MessageService {
    if (!MessageService.instance) {
      MessageService.instance = new MessageService();
    }
    return MessageService.instance;
  }

  // Send a message with queue management
  async sendMessage(
    projectId: string,
    conversationId: string,
    content: string,
    files?: File[],
    isFromQueue: boolean = false
  ): Promise<void> {
    // Create unique key for deduplication
    const messageKey = `${conversationId}-${content.substring(0, 50)}-${Date.now()}`;
    
    // Check if already sending this message
    if (this.sendingMessages.has(messageKey)) {
      logger.warn('MessageService: Duplicate send attempt blocked:', messageKey);
      return;
    }

    try {
      this.sendingMessages.add(messageKey);
      
      const state = conversationStore.conversations[conversationId];
      
      // Queue message if busy (unless it's from queue to prevent infinite loop)
      if (!isFromQueue && state && (state.status === 'streaming' || state.status === 'processing_queue')) {
        const queuedMessage: QueuedMessage = {
          id: `queue-${Date.now()}`,
          content,
          files: files || [],
          timestamp: new Date(),
        };
        
        await this.conversationManager.addToQueue(conversationId, queuedMessage);
        return;
      }

      // Create abort controller FIRST (before setting streaming status)
      const controller = abortControllerManager.create(conversationId);
      logger.debug('MessageService: Created abort controller for:', conversationId);

      // Update status
      logger.debug('MessageService: About to set status to streaming for:', conversationId);
      await this.conversationManager.updateStatus(conversationId, 'streaming');
      logger.debug('MessageService: Status set to streaming, clearing error for:', conversationId);
      await this.conversationManager.setError(conversationId, null);
      logger.debug('MessageService: Error cleared for:', conversationId);

      // Upload files if any
      let uploadedFilePaths: string[] = [];
      if (files && files.length > 0) {
        uploadedFilePaths = await this.uploadFiles(files, projectId, conversationId);
      }

      // Prepare message content
      let messageContent = content;
      if (uploadedFilePaths.length > 0) {
        messageContent += `\n\nAttached files:\n${uploadedFilePaths.map(f => `- ${f}`).join('\n')}`;
      }

      // Add user message to state
      const userMessage: Message = {
        id: `temp-${Date.now()}`,
        role: 'user',
        content: messageContent,
        createdAt: new Date().toISOString(),
      };
      await this.conversationManager.addMessage(conversationId, userMessage);

      // Start streaming
      await chatEventBus.emit({
        type: 'STREAMING_STARTED',
        conversationId,
      });

      logger.debug('MessageService: Starting handleStream for:', conversationId);
      await this.streamingService.handleStream({
        projectId,
        conversationId,
        content,
        uploadedFilePaths,
        abortController: controller,
      });
      logger.debug('MessageService: handleStream completed for:', conversationId);

    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to send message';
      await this.conversationManager.setError(conversationId, errorMessage);
      // Only set to idle if there was an error
      await this.conversationManager.updateStatus(conversationId, 'idle');
      throw error;
    } finally {
      logger.debug('MessageService: Finally block reached for:', conversationId);
      this.sendingMessages.delete(messageKey);
      // Don't set status to idle here - let streaming service handle it
      // The streaming has already been handled, so we can process queue
      await this.processQueue(projectId, conversationId);
    }
  }

  // Process message queue
  private async processQueue(projectId: string, conversationId: string): Promise<void> {
    const nextMessage = await this.conversationManager.getNextQueuedMessage(conversationId);
    
    if (!nextMessage) {
      return;
    }

    // Update status to show queue processing
    await this.conversationManager.updateStatus(conversationId, 'processing_queue');
    
    // Send the queued message
    await this.sendMessage(
      projectId,
      conversationId,
      nextMessage.content,
      nextMessage.files,
      true // Mark as from queue
    );
  }

  // Load messages from API
  async loadMessages(
    conversationId: string,
    _projectId: string
  ): Promise<Message[]> {
    try {
      // IMPORTANT: Verify we're loading messages for the right conversation
      // This prevents message bleeding when multiple conversations are being loaded
      const activeConversationId = conversationStore.activeConversationId;
      
      // Only proceed if this is the active conversation or if there's no active conversation
      if (activeConversationId && activeConversationId !== conversationId) {
        logger.warn(`MessageService: Skipping loadMessages for inactive conversation ${conversationId}, active is ${activeConversationId}`);
        return [];
      }
      
      await this.conversationManager.updateStatus(conversationId, 'loading');
      
      const response = await api.fetchStream(
        `/conversations/${conversationId}/messages`
      );

      if (!response.ok) {
        throw new Error(`Failed to load messages: ${response.status}`);
      }

      const data = await response.json();
      
      // Handle both old array format and new object format with streaming info
      const messages: Message[] = Array.isArray(data) ? data : (data.messages || []);
      const hasActiveStream = !Array.isArray(data) && data.has_active_stream || false;
      
      // Double-check we're still the active conversation before updating
      // This prevents race conditions where another conversation became active
      // while we were loading
      const currentActiveId = conversationStore.activeConversationId;
      if (currentActiveId && currentActiveId !== conversationId) {
        logger.warn(`MessageService: Not updating messages for ${conversationId} as active conversation changed to ${currentActiveId}`);
        return messages;
      }
      
      // If backend indicates an active stream, use SSE resume
      if (hasActiveStream) {
        logger.info('MessageService: Backend indicates active stream for conversation:', conversationId);
        
        // Create a defensive copy of messages to prevent mutation issues
        const messagesCopy = messages.map(m => ({ ...m })); // Deep copy each message
        
        // Log all message IDs to track what we have
        logger.debug('MessageService: All message IDs before filtering:', 
          messagesCopy.map(m => `${m.role}:${m.id.substring(0,8)}:${m.content.length}`).join(', '));
        
        // Log any assistant messages with partial content
        const partialAssistantMessages = messagesCopy.filter(m => 
          m.role === 'assistant' && m.content.length > 0 && m.content.length < 50
        );
        if (partialAssistantMessages.length > 0) {
          logger.warn('MessageService: Found partial assistant messages:', 
            partialAssistantMessages.map(m => ({
              id: m.id,
              content: m.content,
              length: m.content.length
            }))
          );
        }
        
        // Filter out empty or incomplete assistant messages before setting
        // These will be replaced by the loading indicator
        // Also filter out assistant messages that appear to be incomplete JSON or partial content
        const filteredMessages = messagesCopy.filter(msg => {
          if (msg.role !== 'assistant') return true;
          
          // Remove empty assistant messages
          if (msg.content === '') {
            logger.warn('MessageService: Filtering out empty assistant message:', {
              id: msg.id,
              content: msg.content
            });
            return false;
          }
          
          // Remove assistant messages that look like incomplete JSON or partial streaming content
          // These typically start with {"type": or similar patterns and are under 100 chars
          if (msg.content.length < 100 && 
              (msg.content.startsWith('{"') || 
               msg.content.startsWith('[\n') ||
               msg.content.includes('"type":'))) {
            logger.warn('MessageService: Filtering out incomplete assistant message:', {
              id: msg.id,
              content: msg.content
            });
            return false;
          }
          
          return true;
        });
        
        logger.debug('MessageService: All message IDs after filtering:', 
          filteredMessages.map(m => `${m.role}:${m.id.substring(0,8)}:${m.content.length}`).join(', '));
        
        // Update messages first (without empty assistant stub)
        await this.conversationManager.setMessages(conversationId, filteredMessages);
        
        // Resume the stream using SSE
        const streamingService = StreamingService.getInstance();
        streamingService.resumeStream(conversationId, _projectId).catch(error => {
          logger.error('MessageService: Failed to resume stream:', error);
          // Fallback to idle if resume fails
          this.conversationManager.updateStatus(conversationId, 'idle');
        });
        
        return filteredMessages;
      }
      
      // Old polling logic - only use if backend doesn't have active stream
      // and message is very recent (for backward compatibility)
      if (messages.length > 0) {
        const lastMessage = messages[messages.length - 1];
        
        if (lastMessage.role === 'user') {
          const lastMessageTime = new Date(lastMessage.createdAt || '').getTime();
          const timeSinceLastMessage = Date.now() - lastMessageTime;
          
          // Only poll if message is very recent and backend doesn't have active stream
          if (timeSinceLastMessage < 5000 && !hasActiveStream) {
            logger.info('MessageService: Recent user message without backend stream, using polling fallback');
            await this.conversationManager.updateStatus(conversationId, 'streaming');
            this.pollForResponse(conversationId, messages.length);
          }
        }
      }
      
      // Update store with loaded messages
      await this.conversationManager.setMessages(conversationId, messages);
      
      return messages;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to load messages';
      await this.conversationManager.setError(conversationId, errorMessage);
      throw error;
    } finally {
      // Only set to idle if we're still in loading state (not if we're streaming)
      const state = conversationStore.conversations[conversationId];
      if (state && state.status === 'loading') {
        await this.conversationManager.updateStatus(conversationId, 'idle');
      }
    }
  }

  // Forget messages from a point
  async forgetMessagesFrom(
    conversationId: string,
    messageId: string
  ): Promise<void> {
    // Stop any ongoing streaming
    abortControllerManager.abort(conversationId);
    
    try {
      const response = await api.fetchStream(
        `/conversations/${conversationId}/forget-after`,
        {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ message_id: messageId }),
        }
      );

      if (!response.ok) {
        throw new Error(`Failed to forget messages: ${response.status}`);
      }

      const data = await response.json();
      
      // Update forgotten state
      await this.conversationManager.setForgottenState(
        conversationId,
        messageId,
        data.forgotten_count || 0
      );
      
      // Reload messages
      await this.loadMessages(conversationId, conversationStore.currentProjectId || '');
      
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to forget messages';
      await this.conversationManager.setError(conversationId, errorMessage);
      throw error;
    }
  }

  // Restore forgotten messages
  async restoreForgottenMessages(conversationId: string): Promise<void> {
    try {
      const response = await api.fetchStream(
        `/conversations/${conversationId}/forget-after`,
        {
          method: 'DELETE',
        }
      );

      if (!response.ok) {
        throw new Error(`Failed to restore messages: ${response.status}`);
      }

      // Clear forgotten state
      await this.conversationManager.setForgottenState(conversationId, null, 0);
      
      // Reload messages
      await this.loadMessages(conversationId, conversationStore.currentProjectId || '');
      
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to restore messages';
      await this.conversationManager.setError(conversationId, errorMessage);
      throw error;
    }
  }

  // Upload files
  private async uploadFiles(
    files: File[],
    projectId: string,
    conversationId: string
  ): Promise<string[]> {
    const formData = new FormData();
    files.forEach(file => formData.append('files', file));
    
    const clientId = localStorage.getItem('activeClientId');
    if (clientId) {
      formData.append('client_id', clientId);
    }
    formData.append('project_id', projectId);
    formData.append('conversation_id', conversationId);

    const response = await api.fetchStream('/upload', {
      method: 'POST',
      body: formData,
    });

    if (!response.ok) {
      throw new Error(`File upload failed: ${response.status}`);
    }

    const data = await response.json();
    return data.file_paths || [];
  }

  // Stop current message
  async stopMessage(conversationId: string): Promise<void> {
    abortControllerManager.abort(conversationId);
    await this.conversationManager.updateStatus(conversationId, 'idle');
    await this.conversationManager.clearQueue(conversationId);
  }
  
  // Poll for assistant response
  private async pollForResponse(conversationId: string, expectedMessageCount: number): Promise<void> {
    let attempts = 0;
    const maxAttempts = 30; // Poll for up to 30 seconds
    
    const pollInterval = setInterval(async () => {
      attempts++;
      
      try {
        // Check if we're still the active conversation
        if (conversationStore.activeConversationId !== conversationId) {
          clearInterval(pollInterval);
          return;
        }
        
        // Fetch latest messages
        const response = await api.fetchStream(
          `/conversations/${conversationId}/messages`
        );
        
        if (response.ok) {
          const data = await response.json();
          // Handle both old array format and new object format
          const messages: Message[] = Array.isArray(data) ? data : (data.messages || []);
          
          // Check if we got a proper assistant response
          // We're looking for a non-empty assistant message after the expected count
          if (messages.length > expectedMessageCount) {
            const lastMessage = messages[messages.length - 1];
            if (lastMessage.role === 'assistant' && lastMessage.content.length > 0) {
              logger.info('MessageService: Assistant response received via polling');
              // Update messages and clear streaming status
              await this.conversationManager.setMessages(conversationId, messages);
              await this.conversationManager.updateStatus(conversationId, 'idle');
              clearInterval(pollInterval);
              return;
            } else if (lastMessage.role === 'assistant' && lastMessage.content.length === 0) {
              // Still an empty message, keep polling
              logger.debug('MessageService: Still empty assistant message, continuing to poll');
            }
          }
        }
      } catch (error) {
        logger.error('MessageService: Error polling for response:', error);
      }
      
      // Stop polling after max attempts
      if (attempts >= maxAttempts) {
        logger.debug('MessageService: Polling timeout, clearing streaming status');
        const state = conversationStore.conversations[conversationId];
        if (state && state.status === 'streaming') {
          await this.conversationManager.updateStatus(conversationId, 'idle');
          // Show error to user that streaming was interrupted
          await this.conversationManager.setError(conversationId, 
            'Response was interrupted. Please try sending your message again.');
        }
        clearInterval(pollInterval);
      }
    }, 1000); // Poll every second
  }
}