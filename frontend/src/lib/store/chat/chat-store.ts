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
  conversation_id: "" as CONVERSATION_ID,
  list: [] as CONVERSATION_ID[],
  map: {} as Record<CONVERSATION_ID, Conversation>,
  errors: {} as Record<CONVERSATION_ID, string>,
  streaming: {} as Record<CONVERSATION_ID, StreamingState>,
});

export const setConversationError = (
  conversationId: CONVERSATION_ID,
  error: string
) => {
  chatStore.errors[conversationId] = error;
};
