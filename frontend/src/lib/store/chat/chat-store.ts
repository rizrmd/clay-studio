import { proxy } from "valtio";
import { Conversation, CONVERSATION_ID, PROJECT_ID } from "../../types/chat";
import type { StreamingEvent } from "@/components/chat/display/message/in-progress-message";

interface ToolState {
  tool: string;
  toolUsageId: string;
  startTime: number;
  status: "active" | "completed" | "error";
  executionTime?: number;
  completedAt?: number;
}

export interface StreamingState {
  messageId: string;
  partialContent: string;
  activeTools: Array<ToolState>;
  isComplete: boolean;
  events?: StreamingEvent[];
}

export const chatStore = proxy({
  project_id: "" as PROJECT_ID,
  conversation_id: "" as CONVERSATION_ID,
  pendingFirstChat: "",
  expectingInitialMessage: undefined as CONVERSATION_ID | undefined,
  list: [] as CONVERSATION_ID[],
  map: {} as Record<CONVERSATION_ID, Conversation>,
  errors: {} as Record<CONVERSATION_ID, string>,
  streaming: {} as Record<CONVERSATION_ID, StreamingState>,
  loadingMessages: {} as Record<CONVERSATION_ID, boolean>,
});

export const setConversationError = (
  conversationId: CONVERSATION_ID,
  error: string
) => {
  chatStore.errors[conversationId] = error;
};
