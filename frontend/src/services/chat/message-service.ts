import { API_BASE_URL } from '@/lib/url';
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
      await this.conversationManager.updateStatus(conversationId, 'loading');
      
      const response = await fetch(
        `${API_BASE_URL}/conversations/${conversationId}/messages`,
        { credentials: 'include' }
      );

      if (!response.ok) {
        throw new Error(`Failed to load messages: ${response.status}`);
      }

      const messages: Message[] = await response.json();
      
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
      const response = await fetch(
        `${API_BASE_URL}/conversations/${conversationId}/forget`,
        {
          method: 'POST',
          credentials: 'include',
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
      const response = await fetch(
        `${API_BASE_URL}/conversations/${conversationId}/restore`,
        {
          method: 'POST',
          credentials: 'include',
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

    const response = await fetch(`${API_BASE_URL}/upload`, {
      method: 'POST',
      credentials: 'include',
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
}