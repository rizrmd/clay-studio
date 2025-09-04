import { proxy } from "valtio";
import { Conversation, CONVERSATION_ID, PROJECT_ID } from "../../types/chat";

interface StreamingState {
  messageId: string;
  partialContent: string;
  activeTools: Array<{ tool: string; toolUsageId: string; startTime: number }>;
  isComplete: boolean;
}

export const chatStore = proxy({
  project_id: "" as PROJECT_ID,
  active: "" as CONVERSATION_ID,
  list: [] as CONVERSATION_ID[],
  map: {} as Record<CONVERSATION_ID, Conversation>,
  errors: {} as Record<CONVERSATION_ID, string>,
  streaming: {} as Record<CONVERSATION_ID, StreamingState>,
});

export const chatActions = {
  startStreaming(conversationId: CONVERSATION_ID, messageId: string) {
    chatStore.streaming[conversationId] = {
      messageId,
      partialContent: "",
      activeTools: [],
      isComplete: false
    };
  },
  
  updateStreamingContent(conversationId: CONVERSATION_ID, content: any) {
    const stream = chatStore.streaming[conversationId];
    if (!stream) return;
    
    // Extract text content from various formats
    let textContent = "";
    if (typeof content === "string") {
      textContent = content;
    } else if (content?.delta) {
      textContent = content.delta;
    } else if (content?.type === "assistant" && content?.message) {
      // Handle Claude CLI format
      const msg = content.message;
      if (msg.content) {
        if (Array.isArray(msg.content)) {
          textContent = msg.content
            .filter((block: any) => block.type === "text")
            .map((block: any) => block.text || "")
            .join("");
        } else if (typeof msg.content === "string") {
          textContent = msg.content;
        }
      }
    }
    
    // Update partial content
    if (content?.delta) {
      stream.partialContent += textContent;
    } else {
      stream.partialContent = textContent;
    }
    
    // Update message in conversation
    const conversation = chatStore.map[conversationId];
    if (conversation?.messages) {
      const message = conversation.messages.find(m => m.id === stream.messageId);
      if (message) {
        message.content = stream.partialContent;
      }
    }
  },
  
  completeStreaming(conversationId: CONVERSATION_ID) {
    if (chatStore.streaming[conversationId]) {
      chatStore.streaming[conversationId].isComplete = true;
      // Clean up after a delay
      setTimeout(() => {
        delete chatStore.streaming[conversationId];
      }, 1000);
    }
  },
  
  addActiveToolToStream(conversationId: CONVERSATION_ID, tool: string, toolUsageId: string) {
    const stream = chatStore.streaming[conversationId];
    if (stream) {
      stream.activeTools.push({ tool, toolUsageId, startTime: Date.now() });
    }
  },
  
  removeActiveToolFromStream(conversationId: CONVERSATION_ID, toolUsageId: string) {
    const stream = chatStore.streaming[conversationId];
    if (stream) {
      stream.activeTools = stream.activeTools.filter(t => t.toolUsageId !== toolUsageId);
    }
  },
};

export const setConversationError = (conversationId: CONVERSATION_ID, error: string) => {
  chatStore.errors[conversationId] = error;
};
