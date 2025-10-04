import { chatStore } from "../store/chat/chat-store";
import { Message } from "../types/chat";
import { ServerMessage } from "../types/ws";

export const stream = {
  start(msg: ServerMessage & { type: "start" }) {
    const conversation = chatStore.map[msg.conversation_id];
    if (conversation) {
      // Ensure messages array exists
      if (!conversation.messages) {
        conversation.messages = [];
      }

      // Check if message already exists (e.g., after refresh when replaying events)
      const existingMessage = conversation.messages.find(m => m.id === msg.id);

      if (!existingMessage) {
        // Only add new message if it doesn't exist
        const assistantMessage: Message = {
          id: msg.id,
          content: "",
          role: "assistant",
          createdAt: new Date().toISOString(),
        };
        conversation.messages.push(assistantMessage);
      }

      // Always initialize/reset streaming state for active streaming
      chatStore.streaming[msg.conversation_id] = {
        messageId: msg.id,
        partialContent: existingMessage?.content || "", // Preserve existing content if any
        activeTools: [],
        isComplete: false,
        events: [],
      };
    }
  },

  progress(msg: ServerMessage & { type: "progress" }) {
    const conversation = chatStore.map[msg.conversation_id];
    const streamState = chatStore.streaming[msg.conversation_id];

    if (conversation && conversation.messages && conversation.messages.length > 0) {
      const lastMessage =
        conversation.messages[conversation.messages.length - 1];
      if (lastMessage && lastMessage.role === "assistant") {
        // Handle different types of progress messages
        if (msg.content.message?.content) {
          // Extract text content from the message content array
          const textContent = msg.content.message.content
            .filter((item) => item.type === "text")
            .map((item) => item.text)
            .join("");

          if (textContent) {
            lastMessage.content = textContent;
            
            // Add content event to timeline if we have streaming state
            if (streamState && streamState.events) {
              streamState.events.push({
                type: "content",
                timestamp: Date.now(),
                content: textContent,
              });
            }
          }
        } else if (msg.content.result) {
          // Handle result content (final message content)
          lastMessage.content = msg.content.result;
          
          if (streamState && streamState.events) {
            streamState.events.push({
              type: "content",
              timestamp: Date.now(),
              content: msg.content.result,
            });
          }
        }
      }
    }
  },

  content(msg: ServerMessage & { type: "content" }) {
    const conversation = chatStore.map[msg.conversation_id];
    if (conversation && conversation.messages && conversation.messages.length > 0) {
      const lastMessage =
        conversation.messages[conversation.messages.length - 1];
      if (lastMessage && lastMessage.role === "assistant") {
        // Update the content with the final message text
        lastMessage.content = msg.content;
      }
    }
  },

  complete(msg: ServerMessage & { type: "complete" }) {
    const conversation = chatStore.map[msg.conversation_id];
    if (conversation) {
      // Ensure messages array exists
      if (!conversation.messages) {
        conversation.messages = [];
      }

      if (conversation.messages.length > 0) {
        const lastMessage =
          conversation.messages[conversation.messages.length - 1];
        if (lastMessage && lastMessage.id === msg.id) {
          lastMessage.processing_time_ms = msg.processing_time_ms;
          lastMessage.tool_usages = msg.tool_usages;
          // Clear progress_content when message is complete
          lastMessage.progress_content = undefined;
        }
      }
      conversation.message_count = conversation.messages.length;
      conversation.updated_at = new Date().toISOString();

      // Check if there are any incomplete todos before marking streaming as complete
      const streamState = chatStore.streaming[msg.conversation_id];
      if (streamState) {
        const hasIncompleteTodos = checkForIncompleteTodos(streamState);

        if (hasIncompleteTodos) {
          // Don't mark as complete yet - keep streaming state active to show todos
          // Set up a watcher to auto-complete when todos are done
          streamState.isComplete = false;
          watchTodosForCompletion(msg.conversation_id);
        } else {
          // Mark streaming as complete and clear the streaming state after a short delay
          streamState.isComplete = true;
          setTimeout(() => {
            delete chatStore.streaming[msg.conversation_id];
          }, 100);
        }
      }
    }
  },
};

// Helper to check if there are incomplete todos in streaming events
export function checkForIncompleteTodos(streamState: any): boolean {
  if (!streamState.events) return false;

  // Find the latest TodoWrite tool completion
  for (let i = streamState.events.length - 1; i >= 0; i--) {
    const event = streamState.events[i];
    if (event.type === "tool_complete" && event.tool?.toolName === "TodoWrite") {
      try {
        const dataSource = event.tool.output || (event.tool as any).parameters;
        let todos: Array<{ status: string }> | null = null;

        if (dataSource) {
          if (typeof dataSource === "object") {
            if (Array.isArray(dataSource)) {
              todos = dataSource;
            } else if (dataSource.todos) {
              todos = dataSource.todos;
            } else if (dataSource.parameters?.todos) {
              todos = dataSource.parameters.todos;
            }
          } else if (typeof dataSource === "string") {
            try {
              const parsed = JSON.parse(dataSource);
              if (Array.isArray(parsed)) {
                todos = parsed;
              } else if (parsed.todos) {
                todos = parsed.todos;
              } else if (parsed.parameters?.todos) {
                todos = parsed.parameters.todos;
              }
            } catch {
              // Not valid JSON
            }
          }
        }

        // Check if any todos are incomplete
        if (todos && Array.isArray(todos)) {
          return todos.some(
            (todo) => todo.status === "pending" || todo.status === "in_progress"
          );
        }
      } catch (e) {
        console.error("Failed to parse TodoWrite output for completion check:", e);
      }
      break; // Only check the latest TodoWrite
    }
  }

  return false;
}

// Set up a watcher that monitors todo updates and auto-completes when all todos are done
const activeWatchers = new Map<string, NodeJS.Timeout>();

export function watchTodosForCompletion(conversationId: string) {
  // Clear any existing watcher
  const existingWatcher = activeWatchers.get(conversationId);
  if (existingWatcher) {
    clearInterval(existingWatcher);
  }

  // Check every 500ms if todos are complete
  const watcher = setInterval(() => {
    const streamState = chatStore.streaming[conversationId];

    if (!streamState) {
      // Stream state was cleared externally, stop watching
      clearInterval(watcher);
      activeWatchers.delete(conversationId);
      return;
    }

    const hasIncompleteTodos = checkForIncompleteTodos(streamState);

    if (!hasIncompleteTodos) {
      // All todos are complete, mark streaming as complete
      streamState.isComplete = true;
      setTimeout(() => {
        delete chatStore.streaming[conversationId];
      }, 100);

      // Stop watching
      clearInterval(watcher);
      activeWatchers.delete(conversationId);
    }
  }, 500);

  activeWatchers.set(conversationId, watcher);
}

// Immediately check if todos are complete and mark streaming as done if so
// This is called when a TodoWrite tool completes to provide instant feedback
export function checkAndCompleteIfTodosDone(conversationId: string) {
  const streamState = chatStore.streaming[conversationId];

  if (!streamState) {
    return;
  }

  const hasIncompleteTodos = checkForIncompleteTodos(streamState);

  if (!hasIncompleteTodos && !streamState.isComplete) {
    // All todos are complete, mark streaming as complete immediately
    streamState.isComplete = true;
    setTimeout(() => {
      delete chatStore.streaming[conversationId];
    }, 100);

    // Stop any active watcher
    const existingWatcher = activeWatchers.get(conversationId);
    if (existingWatcher) {
      clearInterval(existingWatcher);
      activeWatchers.delete(conversationId);
    }
  }
}
