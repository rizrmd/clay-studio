import { api } from '@/lib/api';
import { logger } from '@/lib/logger';
import { ConversationManager } from '../../store/chat/conversation-manager';
import { conversationStore } from '../../store/chat/conversation-store';
import { chatEventBus } from './event-bus';
import { abortControllerManager } from '../../utils/chat/abort-controller-manager';
import type { Message } from '../../types/chat';

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
  private activeStreams = new Map<string, boolean>();

  private constructor() {
    this.conversationManager = ConversationManager.getInstance();
  }

  static getInstance(): StreamingService {
    if (!StreamingService.instance) {
      StreamingService.instance = new StreamingService();
    }
    return StreamingService.instance;
  }

  async resumeStream(conversationId: string, projectId: string): Promise<void> {
    logger.info('StreamingService: Resuming stream for conversation:', conversationId);
    
    // Create abort controller
    const abortController = abortControllerManager.create(conversationId);
    
    // Mark as active stream
    this.activeStreams.set(conversationId, true);
    let assistantContent = '';
    let completedSuccessfully = false;

    try {
      // Update status to streaming
      await this.conversationManager.updateStatus(conversationId, 'streaming');
      
      const response = await api.fetchStream('/chat/resume', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ conversation_id: conversationId }),
        signal: abortController.signal,
      });

      if (!response.ok) {
        if (response.status === 404) {
          logger.warn('StreamingService: No active stream found to resume');
          // Stream already completed or doesn't exist - set to idle
          await this.conversationManager.updateStatus(conversationId, 'idle');
          return; // Exit gracefully, messages are already loaded
        }
        throw new Error(`Resume failed: ${response.status}`);
      }

      const reader = response.body?.getReader();
      const decoder = new TextDecoder();

      if (!reader) {
        throw new Error('No response body');
      }

      let buffer = '';
      let hasInitialContent = false;

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        const chunk = decoder.decode(value, { stream: true });
        buffer += chunk;
        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          const trimmedLine = line.trim();
          if (trimmedLine.startsWith('data:')) {
            const data = trimmedLine.slice(5).trim();
            if (data === '[DONE]') continue;
            if (!data) continue;

            try {
              const event = JSON.parse(data);
              
              switch (event.type) {
                case 'start':
                  logger.debug('Resume: Received start event', event);
                  break;
                  
                case 'content':
                  // Initial partial content
                  if (!hasInitialContent && event.content) {
                    assistantContent = event.content;
                    hasInitialContent = true;
                    await this.handleProgressEvent(
                      { content: JSON.stringify({ type: 'text', text: assistantContent }) },
                      conversationId,
                      ''
                    );
                  }
                  break;
                  
                case 'tool_use':
                  logger.debug('Resume: Received tool_use event', event.tool);
                  if (event.tool) {
                    await this.conversationManager.addActiveTool(conversationId, event.tool);
                  }
                  break;
                  
                case 'progress':
                  assistantContent = await this.handleProgressEvent(event, conversationId, assistantContent);
                  break;
                  
                case 'complete':
                  completedSuccessfully = true;
                  await this.handleCompleteEvent(event, conversationId, projectId);
                  break;
                  
                case 'error':
                  throw new Error(event.error);
              }
            } catch (e) {
              logger.error('Resume: Failed to parse SSE event:', e);
            }
          }
        }
      }
    } catch (error: any) {
      if (error.name === 'AbortError') {
        logger.info('Resume: Stream aborted by user');
      } else {
        logger.error('Resume: Stream error:', error);
        await this.conversationManager.setError(conversationId, error.message);
      }
    } finally {
      this.activeStreams.delete(conversationId);
      
      if (!completedSuccessfully) {
        await this.conversationManager.updateStatus(conversationId, 'idle');
      }
      
      // Abort controller is already cleaned up by abortControllerManager.abort() if called
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
    let assistantContent = '';
    let realConversationId = conversationId;
    let completedSuccessfully = false;

    try {
      logger.debug('StreamingService: Fetching stream for:', conversationId);
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

      logger.info('StreamingService: Response received, status:', response.status);

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const reader = response.body?.getReader();
      const decoder = new TextDecoder();

      if (!reader) {
        throw new Error('No response body');
      }

      logger.debug('StreamingService: Got reader, starting to read stream for:', conversationId);
      let buffer = '';
      let chunkCount = 0;

      while (true) {
        logger.debug('StreamingService: About to read chunk #', chunkCount);
        const readStart = Date.now();
        const { done, value } = await reader.read();
        const readTime = Date.now() - readStart;
        logger.debug('StreamingService: Chunk #', chunkCount, 'read in', readTime, 'ms, done:', done, 'has value:', !!value);
        chunkCount++;
        
        if (value) {
          const chunk = decoder.decode(value, { stream: true });
          buffer += chunk;
          logger.debug('StreamingService: Decoded chunk, buffer size:', buffer.length);
        }
        
        const lines = buffer.split('\n');
        
        // Keep the last line in buffer if stream is not done
        if (!done) {
          buffer = lines.pop() || '';
        } else {
          // Process all lines including the last one when done
          buffer = '';
        }

        for (const line of lines) {
          const trimmedLine = line.trim();
          if (trimmedLine.startsWith('data:')) {
            const data = trimmedLine.slice(5).trim();
            if (data === '[DONE]') continue;
            if (!data) continue;

            try {
              const event = JSON.parse(data);
              logger.debug('StreamingService: Received event:', event.type, 'for', conversationId);
              const result = await this.handleStreamEvent(
                event, 
                conversationId,
                realConversationId, 
                projectId,
                assistantContent
              );
              
              // Update conversation ID if it changed
              if (result.conversationId) {
                realConversationId = result.conversationId;
              }
              
              // Update content
              if (result.content === 'COMPLETED') {
                completedSuccessfully = true;
              } else if (result.content) {
                assistantContent = result.content;
              }
            } catch (e) {
              logger.error('StreamingService: Failed to parse SSE event:', e);
            }
          }
        }
        
        if (done) {
          logger.debug('StreamingService: Reader done, processed all events for:', conversationId);
          break;
        }
      }
      
      logger.info('StreamingService: Stream loop ended for:', conversationId, 'completed:', completedSuccessfully);
      
      // If stream ended without a complete event, set status to idle
      if (!completedSuccessfully) {
        logger.warn('StreamingService: Stream ended without complete event, setting to idle');
        await this.conversationManager.updateStatus(realConversationId, 'idle');
      }
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') {
        logger.info('StreamingService: Stream aborted for:', conversationId);
        // Set status back to idle when aborted
        await this.conversationManager.updateStatus(realConversationId, 'idle');
      } else {
        logger.error('StreamingService: Stream error for:', conversationId, error);
        // Set error status and re-throw
        await this.conversationManager.setError(realConversationId, error instanceof Error ? error.message : 'Streaming failed');
        await this.conversationManager.updateStatus(realConversationId, 'idle');
        throw error;
      }
    } finally {
      this.activeStreams.delete(conversationId);
      this.activeStreams.delete(realConversationId);
      
      logger.debug('StreamingService: Finally block - cleaning up for:', realConversationId);
      
      // Emit streaming stopped event
      await chatEventBus.emit({
        type: 'STREAMING_STOPPED',
        conversationId: realConversationId,
      });
      
      logger.debug('StreamingService: handleStream completed for:', realConversationId);
    }
  }

  private async handleStreamEvent(
    event: any, 
    originalConversationId: string,
    currentConversationId: string,
    projectId: string,
    assistantContent: string
  ): Promise<{ conversationId?: string; content?: string }> {
    let updatedConversationId = currentConversationId;
    let updatedContent = assistantContent;

    switch (event.type) {
      case 'start':
        if (event.conversation_id && event.conversation_id !== 'new' && originalConversationId === 'new') {
          // Handle transition from 'new' to real conversation
          updatedConversationId = await this.handleNewConversationTransition(
            originalConversationId,
            event.conversation_id,
            projectId
          );
        }
        break;

      case 'progress':
        updatedContent = await this.handleProgressEvent(event, currentConversationId, assistantContent);
        break;

      case 'content':
        await this.handleContentEvent(event, currentConversationId);
        break;

      case 'tool_use':
        if (event.tool) {
          await this.conversationManager.addActiveTool(currentConversationId, event.tool);
        }
        break;

      case 'complete':
        await this.handleCompleteEvent(event, currentConversationId, projectId);
        updatedContent = 'COMPLETED'; // Special marker
        break;

      case 'error':
        await this.conversationManager.setError(currentConversationId, event.error);
        await this.conversationManager.clearActiveTools(currentConversationId);
        break;
    }

    return { conversationId: updatedConversationId, content: updatedContent };
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
    // This is important for the UI to display loading state properly
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
    logger.info('StreamingService: Emitting CONVERSATION_CREATED event:', {
      type: 'CONVERSATION_CREATED',
      conversationId: newId,
      projectId,
    });
    
    await chatEventBus.emit({
      type: 'CONVERSATION_CREATED',
      conversationId: newId,
      projectId,
    });

    return newId;
  }

  private async handleProgressEvent(
    event: any,
    conversationId: string,
    currentContent: string
  ): Promise<string> {
    try {
      const streamJson = JSON.parse(event.content);
      if (streamJson.type === 'text' || streamJson.type === 'progress') {
        const textContent = streamJson.text || streamJson.content || '';
        if (textContent) {
          const newContent = currentContent + textContent;
          
          // Check if we need to create or update assistant message
          const state = conversationStore.conversations[conversationId];
          if (state && state.messages.length > 0) {
            const lastMessage = state.messages[state.messages.length - 1];
            
            if (lastMessage.role === 'assistant') {
              await this.conversationManager.updateLastMessage(conversationId, {
                content: newContent,
              });
            } else {
              // Create new assistant message
              const assistantMessage: Message = {
                id: `streaming-${Date.now()}`,
                role: 'assistant',
                content: newContent,
                createdAt: new Date().toISOString(),
              };
              await this.conversationManager.addMessage(conversationId, assistantMessage);
            }
          }
          
          return newContent;
        }
      }
    } catch (e) {
      // Ignore parse errors
    }
    
    return currentContent;
  }

  private async handleContentEvent(event: any, conversationId: string): Promise<void> {
    if (!event.content) return;

    const state = conversationStore.conversations[conversationId];
    if (!state || state.messages.length === 0) return;

    const lastMessage = state.messages[state.messages.length - 1];
    
    if (lastMessage.role === 'assistant') {
      await this.conversationManager.updateLastMessage(conversationId, {
        content: event.content,
      });
    } else {
      // Create new assistant message
      const assistantMessage: Message = {
        id: `streaming-${Date.now()}`,
        role: 'assistant',
        content: event.content,
        createdAt: new Date().toISOString(),
      };
      await this.conversationManager.addMessage(conversationId, assistantMessage);
    }
  }

  private async handleCompleteEvent(
    event: any,
    conversationId: string,
    _projectId: string
  ): Promise<void> {
    logger.debug('StreamingService: Handling complete event for:', conversationId);
    
    // Update specific message by ID with final data - this is more precise than updateLastMessage
    // and prevents accidentally updating the wrong message when filtering changes message order
    const updates = {
      clay_tools_used: event.tools_used?.length > 0 ? event.tools_used : undefined,
      processing_time_ms: event.processing_time_ms,
    };
    
    if (event.id) {
      // Try to update by specific ID first
      await this.conversationManager.updateMessageById(conversationId, event.id, updates);
    } else {
      // Fallback to updating last message if no specific ID provided
      logger.debug('StreamingService: No event ID, using fallback to update last message');
      await this.conversationManager.updateLastMessage(conversationId, updates);
    }
    
    // Set status back to idle after streaming completes
    logger.debug('StreamingService: Setting status to idle after complete for:', conversationId);
    await this.conversationManager.updateStatus(conversationId, 'idle');
    
    // Clear active tools after a brief delay to allow UI transition from active to completed tools
    setTimeout(() => {
      this.conversationManager.clearActiveTools(conversationId);
    }, 100);
    
    // Emit completion event
    await chatEventBus.emit({
      type: 'MESSAGE_SENT',
      conversationId,
      messageId: event.id,
    });
  }
}