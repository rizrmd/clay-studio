import { api } from "@/lib/api";
import { logger } from "@/lib/logger";
import {
  store,
  getConversationState,
  setActiveConversation,
  addMessage,
  updateLastMessage,
  setConversationLoading,
  setConversationError,
  setConversationStreaming,
  setConversationAbortController,
  getConversationAbortController,
  addActiveTool,
  clearActiveTools,
} from "../../store/chat-store";

interface StreamingOptions {
  projectId: string;
  conversationId: string;
  content: string;
  uploadedFilePaths?: string[];
  abortController: AbortController;
  isResend?: boolean;
}

export class StreamingHandler {
  /**
   * Handle streaming response from the chat API
   */
  static async handleStream(options: StreamingOptions): Promise<void> {
    const { projectId, conversationId, content, abortController, isResend = false } = options;
    
    // Set initial states
    setConversationStreaming(conversationId, true);
    
    // Dispatch streaming started event
    if (!isResend) {
      window.dispatchEvent(new CustomEvent('streaming-started', { 
        detail: { conversationId, projectId } 
      }));
    }

    let assistantContent = "";

    try {
      const response = await api.fetchStream('/chat/stream', {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          messages: [
            {
              id: `msg-${Date.now()}`,
              role: "user",
              content,
            },
          ],
          project_id: projectId,
          conversation_id: conversationId,
        }),
        signal: abortController.signal,
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const reader = response.body?.getReader();
      const decoder = new TextDecoder();

      if (!reader) {
        throw new Error("No response body");
      }

      let buffer = "";

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        const chunk = decoder.decode(value, { stream: true });
        buffer += chunk;
        const lines = buffer.split("\n");

        buffer = lines.pop() || "";

        for (const line of lines) {
          const trimmedLine = line.trim();
          if (trimmedLine.startsWith("data:")) {
            const data = trimmedLine.slice(5).trim();
            if (data === "[DONE]") continue;
            if (!data) continue;

            try {
              const event = JSON.parse(data);
              await this.handleStreamEvent(event, conversationId, projectId, assistantContent);
              
              // Update assistant content if needed
              if (event.type === "progress") {
                const streamJson = JSON.parse(event.content);
                if (streamJson.type === "text" || streamJson.type === "progress") {
                  const textContent = streamJson.text || streamJson.content || "";
                  if (textContent) {
                    assistantContent += textContent;
                  }
                }
              }
            } catch (e) {
              // Failed to parse SSE event - skip
            }
          }
        }
      }
    } finally {
      setConversationStreaming(conversationId, false);
      
      // Dispatch streaming stopped event
      window.dispatchEvent(new CustomEvent('streaming-stopped', { 
        detail: { conversationId, projectId } 
      }));
    }
  }

  /**
   * Handle individual stream events
   */
  private static async handleStreamEvent(
    event: any, 
    targetConversationId: string, 
    projectId: string,
    assistantContent: string
  ): Promise<void> {
    // For start events, handle them first as they may change the conversation ID
    if (event.type === "start") {
      await this.handleStartEvent(event, targetConversationId, projectId);
      return; // Don't process other logic for start events
    }
    
    // After start event processing, get the current update ID
    // This will be consistent for all subsequent events in this stream
    const updateId = this.getUpdateConversationId(targetConversationId);
    
    switch (event.type) {
      case "progress":
        await this.handleProgressEvent(event, updateId, assistantContent);
        break;
        
      case "tool_use":
        if (event.tool) {
          addActiveTool(updateId, event.tool);
        }
        break;
        
      case "content":
        await this.handleContentEvent(event, updateId, assistantContent);
        break;
        
      case "complete":
        await this.handleCompleteEvent(event, updateId, projectId);
        break;
        
      case "error":
        setConversationError(updateId, event.error);
        clearActiveTools(updateId);
        break;
    }
  }

  private static getUpdateConversationId(targetConversationId: string): string {
    if (targetConversationId === "new" && store.activeConversationId && store.activeConversationId !== "new") {
      return store.activeConversationId;
    }
    return targetConversationId;
  }

  private static async handleStartEvent(event: any, targetConversationId: string, projectId: string): Promise<void> {
    if (event.conversation_id && event.conversation_id !== "new" && targetConversationId === "new") {
      logger.info("StreamingHandler: Received real conversation ID:", event.conversation_id);

      // Transfer state from 'new' to real conversation
      const newState = getConversationState("new");
      const realState = getConversationState(event.conversation_id);
      
      realState.messages = [...newState.messages];
      realState.isLoading = newState.isLoading;
      realState.isStreaming = newState.isStreaming;
      realState.error = newState.error;
      realState.uploadedFiles = [...newState.uploadedFiles];
      realState.messageQueue = [...newState.messageQueue];
      realState.activeTools = [...newState.activeTools];
      
      // Transfer abort controller
      const newController = getConversationAbortController("new");
      if (newController) {
        setConversationAbortController(event.conversation_id, newController);
        setConversationAbortController("new", null);
      }

      setActiveConversation(event.conversation_id);
      
      // Notify sidebar to refresh
      window.dispatchEvent(new CustomEvent('conversation-created', {
        detail: { conversationId: event.conversation_id, projectId }
      }));
    }
  }

  private static async handleProgressEvent(event: any, updateId: string, currentAssistantContent: string): Promise<void> {
    try {
      const streamJson = JSON.parse(event.content);
      if (streamJson.type === "text" || streamJson.type === "progress") {
        const textContent = streamJson.text || streamJson.content || "";
        if (textContent) {
          const newContent = currentAssistantContent + textContent;
          const currentMessages = getConversationState(updateId).messages;
          const lastMessage = currentMessages[currentMessages.length - 1];
          
          if (lastMessage && lastMessage.role === "assistant") {
            updateLastMessage(updateId, { content: newContent });
          } else {
            // Use a stable ID for the streaming message
            addMessage(updateId, {
              id: `streaming-assistant-${updateId}`,
              role: "assistant",
              content: newContent,
              createdAt: new Date().toISOString(),
            });
          }
        }
      }
    } catch (parseError) {
      // Skip non-JSON messages
    }
  }

  private static async handleContentEvent(event: any, updateId: string, assistantContent: string): Promise<void> {
    if (event.content) {
      const currentMessages = getConversationState(updateId).messages;
      const lastMessage = currentMessages[currentMessages.length - 1];
      
      if (lastMessage && lastMessage.role === "assistant") {
        updateLastMessage(updateId, { content: event.content });
      } else if (!assistantContent) {
        // Use a stable ID for the streaming message
        addMessage(updateId, {
          id: `streaming-assistant-${updateId}`,
          role: "assistant",
          content: event.content,
          createdAt: new Date().toISOString(),
        });
      }
    }
  }

  private static async handleCompleteEvent(event: any, updateId: string, projectId: string): Promise<void> {
    console.log("StreamingHandler: handleCompleteEvent", event);
    console.log("StreamingHandler: tool_usages in event", event.tool_usages);
    
    updateLastMessage(updateId, {
      id: event.id,
      clay_tools_used: event.tools_used.length > 0 ? event.tools_used : undefined, // For backward compatibility
      tool_usages: event.tool_usages, // Now includes full tool_usages from backend
      processing_time_ms: event.processing_time_ms,
    });
    
    clearActiveTools(updateId);
    setConversationLoading(updateId, false);
    
    // Clear loading state for 'new' if we transitioned
    if (updateId !== "new") {
      setConversationLoading("new", false);
    }
    
    // Dispatch event to update sidebar
    window.dispatchEvent(new CustomEvent('message-sent', { 
      detail: { conversationId: updateId, projectId } 
    }));
  }
}