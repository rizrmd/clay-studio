import { chatStore } from "../store/chat/chat-store";
import { Message } from "../types/chat";
import { ServerMessage } from "../types/ws";

export const stream = {
  start(msg: ServerMessage & { type: "start" }) {
    const conversation = chatStore.map[msg.conversation_id];
    if (conversation) {
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
    
    if (conversation && conversation.messages.length > 0) {
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
    if (conversation && conversation.messages.length > 0) {
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
      const lastMessage =
        conversation.messages[conversation.messages.length - 1];
      if (lastMessage && lastMessage.id === msg.id) {
        lastMessage.processing_time_ms = msg.processing_time_ms;
        lastMessage.tool_usages = msg.tool_usages;
        // Clear progress_content when message is complete
        lastMessage.progress_content = undefined;
      }
      conversation.message_count = conversation.messages.length;
      conversation.updated_at = new Date().toISOString();
      
      // Mark streaming as complete and clear the streaming state after a short delay
      if (chatStore.streaming[msg.conversation_id]) {
        chatStore.streaming[msg.conversation_id].isComplete = true;
        // Clear streaming state after a short delay to allow UI to update
        setTimeout(() => {
          delete chatStore.streaming[msg.conversation_id];
        }, 100);
      }
    }
  },
};
