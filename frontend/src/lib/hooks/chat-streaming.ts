import { chatStore } from "../store/chat/chat-store";
import { Message } from "../types/chat";
import { ServerMessage } from "../types/ws";

export const stream = {
  start(msg: ServerMessage & { type: "start" }) {
    const conversation = chatStore.map[msg.conversation_id];
    if (conversation) {
      const assistantMessage: Message = {
        id: msg.id,
        content: "",
        role: "assistant",
        createdAt: new Date().toISOString(),
      };
      conversation.messages.push(assistantMessage);
    }
  },

  progress(msg: ServerMessage & { type: "progress" }) {
    const conversation = chatStore.map[msg.conversation_id];
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
          }
        } else if (msg.content.result) {
          // Handle result content (final message content)
          lastMessage.content = msg.content.result;
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
      }
      conversation.message_count = conversation.messages.length;
      conversation.updated_at = new Date().toISOString();
    }
  },
};
