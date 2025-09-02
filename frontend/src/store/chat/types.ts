import type { Message, ConversationContext, ProjectContextResponse } from "../../types/chat";

export interface QueuedMessage {
  id: string;
  content: string;
  files: File[];
  timestamp: Date;
}

export type ConversationStatus = 
  | 'idle'
  | 'loading'
  | 'streaming'
  | 'error'
  | 'processing_queue';

export interface ContextUsageInfo {
  totalChars: number;
  maxChars: number;
  percentage: number;
  messageCount: number;
  needsCompaction: boolean;
}

export interface ActiveToolInfo {
  tool_name: string;
  tool_usage_id?: string;
  status: 'executing' | 'completed';
  execution_time_ms?: number;
  started_at: string;
  completed_at?: string;
}

export interface ConversationState {
  id: string;
  status: ConversationStatus;
  messages: Message[];
  error: string | null;
  uploadedFiles: any[];
  forgottenAfterMessageId: string | null;
  forgottenCount: number;
  messageQueue: QueuedMessage[];
  activeTools: ActiveToolInfo[];
  context?: ConversationContext;
  contextUsage?: ContextUsageInfo;
  lastUpdated: number;
  version: number; // For detecting stale updates
}

export interface GlobalChatState {
  currentProjectId: string | null;
  activeConversationId: string | null;
  conversations: Record<string, ConversationState>;
  projectContexts: Record<string, ProjectContextResponse>;
  pendingOperations: Set<string>; // Track ongoing operations
}

export interface InputState {
  draftMessage: string;
  attachments: File[];
  isTyping: boolean;
}

export type ConversationEvent = 
  | { type: 'CONVERSATION_CREATED'; conversationId: string; projectId: string }
  | { type: 'CONVERSATION_SWITCHED'; from: string | null; to: string }
  | { type: 'CONVERSATION_REDIRECT'; oldConversationId: string; newConversationId: string }
  | { type: 'MESSAGE_SENT'; conversationId: string; messageId: string }
  | { type: 'STREAMING_STARTED'; conversationId: string }
  | { type: 'STREAMING_STOPPED'; conversationId: string }
  | { type: 'CONVERSATION_CLEARED'; conversationId: string }
  | { type: 'ERROR_OCCURRED'; conversationId: string; error: string }
  | { type: 'CONVERSATION_TITLE_UPDATED'; conversationId: string; title: string }
  | { type: 'ASK_USER'; conversationId: string; promptType?: string; title?: string; options?: any[]; inputType?: string; placeholder?: string; toolUseId?: string };