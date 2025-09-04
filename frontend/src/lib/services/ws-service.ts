import type {
  ServerMessage,
  ClientMessage,
  StreamingState,
} from "@/lib/types/ws";
import type { ToolUsage } from "@/lib/types/chat";

// Simple browser-compatible EventEmitter
class EventEmitter {
  private events: { [key: string]: Function[] } = {};

  on(event: string, listener: Function): void {
    if (!this.events[event]) {
      this.events[event] = [];
    }
    this.events[event].push(listener);
  }

  off(event: string, listener: Function): void {
    const listeners = this.events[event];
    if (listeners) {
      const index = listeners.indexOf(listener);
      if (index > -1) {
        listeners.splice(index, 1);
      }
    }
  }

  emit(event: string, ...args: any[]): void {
    const listeners = this.events[event];
    if (listeners) {
      listeners.forEach((listener) => listener(...args));
    }
  }

  setMaxListeners(_maxListeners: number): void {
    // No-op for browser compatibility
  }
}

class WebSocketService extends EventEmitter {
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectTimeout: NodeJS.Timeout | null = null;
  private pingInterval: NodeJS.Timeout | null = null;
  private isConnecting = false;
  private currentProjectId = "";
  private currentConversationId = "";
  private activeStreams = new Map<string, StreamingState>();
  private messageQueue: ClientMessage[] = [];

  // Singleton pattern
  private static instance: WebSocketService;
  static getInstance(): WebSocketService {
    if (!WebSocketService.instance) {
      WebSocketService.instance = new WebSocketService();
    }
    return WebSocketService.instance;
  }

  private constructor() {
    super();
    this.setMaxListeners(50); // Allow many listeners for chat events
  }

  connect(projectId?: string, conversationId?: string): void {
    if (this.ws?.readyState === WebSocket.OPEN || this.isConnecting) {
      return;
    }

    this.isConnecting = true;
    this.currentProjectId = projectId || "";
    this.currentConversationId = conversationId || "";

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const host = window.location.host;
    const url = `${protocol}//${host}/api/ws`;

    try {
      this.ws = new WebSocket(url);
      this.setupEventHandlers();
    } catch (error) {
      console.error("WebSocket connection failed:", error);
      this.handleReconnect();
    }
  }

  disconnect(): void {
    this.isConnecting = false;
    this.clearTimers();

    if (this.ws) {
      this.ws.close(1000, "Client disconnect");
      this.ws = null;
    }

    this.activeStreams.clear();
    this.messageQueue = [];
    this.emit("disconnected");
  }

  isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  isSubscribed(projectId: string, conversationId?: string): boolean {
    const targetConversationId = conversationId || "";
    return this.currentProjectId === projectId && this.currentConversationId === targetConversationId;
  }

  subscribe(projectId: string, conversationId?: string): void {
    const newConversationId = conversationId || "";
    
    // Don't send duplicate subscription
    if (this.currentProjectId === projectId && this.currentConversationId === newConversationId) {
      return;
    }

    this.currentProjectId = projectId;
    this.currentConversationId = newConversationId;

    const message: ClientMessage = {
      type: "subscribe",
      project_id: projectId,
      conversation_id: conversationId,
    };

    this.sendMessage(message);
  }

  unsubscribe(): void {
    const message: ClientMessage = { type: "unsubscribe" };
    this.sendMessage(message);
    this.currentProjectId = "";
    this.currentConversationId = "";
  }

  sendChatMessage(
    projectId: string,
    conversationId: string,
    content: string,
    uploadedFilePaths?: string[]
  ): void {
    const message: ClientMessage = {
      type: "send_message",
      project_id: projectId,
      conversation_id: conversationId,
      content,
      uploaded_file_paths: uploadedFilePaths,
    };

    this.sendMessage(message);
  }

  // New conversation management methods
  createConversation(projectId: string, title?: string): void {
    const message: ClientMessage = {
      type: "create_conversation",
      project_id: projectId,
      title,
    };
    this.sendMessage(message);
  }

  listConversations(projectId: string): void {
    const message: ClientMessage = {
      type: "list_conversations",
      project_id: projectId,
    };
    this.sendMessage(message);
  }

  getConversation(conversationId: string): void {
    const message: ClientMessage = {
      type: "get_conversation",
      conversation_id: conversationId,
    };
    this.sendMessage(message);
  }

  updateConversation(conversationId: string, title?: string): void {
    const message: ClientMessage = {
      type: "update_conversation",
      conversation_id: conversationId,
      title,
    };
    this.sendMessage(message);
  }

  deleteConversation(conversationId: string): void {
    const message: ClientMessage = {
      type: "delete_conversation",
      conversation_id: conversationId,
    };
    this.sendMessage(message);
  }

  bulkDeleteConversations(conversationIds: string[]): void {
    const message: ClientMessage = {
      type: "bulk_delete_conversations",
      conversation_ids: conversationIds,
    };
    this.sendMessage(message);
  }

  getConversationMessages(conversationId: string): void {
    const message: ClientMessage = {
      type: "get_conversation_messages",
      conversation_id: conversationId,
    };
    this.sendMessage(message);
  }

  sendAskUserResponse(
    conversationId: string,
    interactionId: string,
    response: any
  ): void {
    const message: ClientMessage = {
      type: "ask_user_response",
      conversation_id: conversationId,
      interaction_id: interactionId,
      response,
    };

    this.sendMessage(message);
  }

  stopStreaming(conversationId: string): void {
    const message: ClientMessage = {
      type: "stop_streaming",
      conversation_id: conversationId,
    };

    this.sendMessage(message);
  }

  private sendMessage(message: ClientMessage): void {
    if (this.isConnected()) {
      this.ws!.send(JSON.stringify(message));
    } else {
      // Queue message for when connection is restored
      this.messageQueue.push(message);
      if (!this.isConnecting) {
        this.connect();
      }
    }
  }

  private setupEventHandlers(): void {
    if (!this.ws) return;

    this.ws.onopen = () => {
      console.log("WebSocket connected");
      this.isConnecting = false;
      this.reconnectAttempts = 0;

      // Send queued messages
      while (this.messageQueue.length > 0) {
        const message = this.messageQueue.shift();
        if (message) {
          this.sendMessage(message);
        }
      }

      // Auto-subscribe if we have project/conversation
      if (this.currentProjectId) {
        this.subscribe(
          this.currentProjectId,
          this.currentConversationId || undefined
        );
      }

      this.startPingInterval();
      this.emit("connected");
    };

    this.ws.onmessage = (event) => {
      try {
        const message: ServerMessage = JSON.parse(event.data);
        this.handleServerMessage(message);
      } catch (error) {
        console.error("Failed to parse WebSocket message:", error, event.data);
      }
    };

    this.ws.onclose = (event) => {
      console.log("WebSocket closed:", event.code, event.reason);
      this.isConnecting = false;
      this.clearTimers();
      this.emit("disconnected");

      // Auto-reconnect unless it was a clean close
      if (event.code !== 1000) {
        this.handleReconnect();
      }
    };

    this.ws.onerror = (error) => {
      console.error("WebSocket error:", error);
      this.emit("error", error);
    };
  }

  private handleServerMessage(message: ServerMessage): void {
    // Emit the message for components to listen to
    this.emit(message.type, message);

    // Handle specific message types
    switch (message.type) {
      case "connected":
        if (!message.authenticated) {
          console.warn("WebSocket not authenticated");
          this.emit("authentication_required");
        }
        break;

      case "conversation_redirect":
        // Update current conversation ID when redirected from "new"
        if (this.currentConversationId === message.old_conversation_id) {
          this.currentConversationId = message.new_conversation_id;
          this.emit("conversation_redirected", message);
        }
        break;

      case "subscribed":
        // Update our local subscription state when server confirms subscription
        this.currentProjectId = message.project_id;
        this.currentConversationId = message.conversation_id || "";
        break;

      case "conversation_created": {
        // Backend auto-subscribes to new conversations, so update our local state
        // to prevent duplicate subscription attempts
        const createdMessage = message as ServerMessage & { type: "conversation_created" };
        this.currentProjectId = createdMessage.conversation.project_id;
        this.currentConversationId = createdMessage.conversation.id;
        break;
      }

      case "start":
        this.handleStreamStart(message.id, message.conversation_id);
        break;

      case "progress":
        this.handleStreamProgress(message.content, message.conversation_id);
        break;

      case "tool_use":
        this.handleToolUse(
          message.tool,
          message.tool_usage_id,
          message.conversation_id
        );
        break;

      case "tool_complete":
        this.handleToolComplete(
          message.tool,
          message.tool_usage_id,
          message.execution_time_ms,
          message.output,
          message.conversation_id
        );
        break;

      case "complete":
        this.handleStreamComplete(
          message.id,
          message.conversation_id,
          message.processing_time_ms,
          message.tool_usages
        );
        break;

      case "error":
        this.handleStreamError(message.error, message.conversation_id);
        break;
    }
  }

  private handleStreamStart(messageId: string, conversationId: string): void {
    const streamState: StreamingState = {
      messageId,
      partialContent: "",
      activeTools: [],
      isComplete: false,
    };

    this.activeStreams.set(conversationId, streamState);
    this.emit("stream_started", { messageId, conversationId });
  }

  private handleStreamProgress(content: any, conversationId: string): void {
    const stream = this.activeStreams.get(conversationId);
    if (!stream) return;

    // Accumulate content
    if (typeof content === "string") {
      stream.partialContent += content;
    } else if (content.delta) {
      stream.partialContent += content.delta;
    }

    this.emit("stream_progress", {
      conversationId,
      content: stream.partialContent,
      partial: content,
    });
  }

  private handleToolUse(
    tool: string,
    toolUsageId: string,
    conversationId: string
  ): void {
    const stream = this.activeStreams.get(conversationId);
    if (!stream) return;

    stream.activeTools.push({
      tool,
      toolUsageId,
      startTime: Date.now(),
    });

    this.emit("tool_started", { tool, toolUsageId, conversationId });
  }

  private handleToolComplete(
    tool: string,
    toolUsageId: string,
    executionTimeMs: number,
    output: any,
    conversationId: string
  ): void {
    const stream = this.activeStreams.get(conversationId);
    if (!stream) return;

    // Remove from active tools
    stream.activeTools = stream.activeTools.filter(
      (t) => t.toolUsageId !== toolUsageId
    );

    this.emit("tool_completed", {
      tool,
      toolUsageId,
      executionTimeMs,
      output,
      conversationId,
    });
  }

  private handleStreamComplete(
    messageId: string,
    conversationId: string,
    processingTimeMs: number,
    toolUsages?: ToolUsage[]
  ): void {
    const stream = this.activeStreams.get(conversationId);
    if (stream) {
      stream.isComplete = true;
      this.activeStreams.delete(conversationId);
    }

    this.emit("stream_completed", {
      messageId,
      conversationId,
      processingTimeMs,
      toolUsages,
      finalContent: stream?.partialContent || "",
    });
  }

  private handleStreamError(error: string, conversationId: string): void {
    const stream = this.activeStreams.get(conversationId);
    if (stream) {
      this.activeStreams.delete(conversationId);
    }

    this.emit("stream_error", { error, conversationId });
  }

  private startPingInterval(): void {
    this.pingInterval = setInterval(() => {
      if (this.isConnected()) {
        this.sendMessage({ type: "ping" });
      }
    }, 30000); // Ping every 30 seconds
  }

  private clearTimers(): void {
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }

    if (this.pingInterval) {
      clearInterval(this.pingInterval);
      this.pingInterval = null;
    }
  }

  private handleReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error("Max reconnection attempts reached");
      this.emit("max_reconnect_attempts_reached");
      return;
    }

    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
    console.log(
      `Attempting to reconnect in ${delay}ms (attempt ${
        this.reconnectAttempts + 1
      }/${this.maxReconnectAttempts})`
    );

    this.reconnectTimeout = setTimeout(() => {
      this.reconnectAttempts++;
      this.connect();
    }, delay);
  }

  // Get current streaming state for a conversation
  getStreamingState(conversationId: string): StreamingState | null {
    return this.activeStreams.get(conversationId) || null;
  }

  // Check if a conversation is currently streaming
  isStreaming(conversationId: string): boolean {
    const stream = this.activeStreams.get(conversationId);
    return stream ? !stream.isComplete : false;
  }
}

export const wsService = WebSocketService.getInstance();
