// Re-export types from lib/types for backwards compatibility
export type {
  Message,
  MessageRole,
  Conversation,
  ToolUsage,
  FileUploadResponse as FileAttachment,
  AskUserData,
  PROJECT_ID,
  CONVERSATION_ID
} from "@/lib/types/chat";

// Re-export the function
export { getToolNamesFromMessage } from "@/lib/types/chat";

// Additional display-specific types
export interface MessageWithUI extends Message {
  isGenerating?: boolean;
  isError?: boolean;
}

export interface DisplayMessage extends Message {
  isQueued?: boolean;
  isEditing?: boolean;
  ask_user?: unknown; // Legacy field for backward compatibility
}

export interface ToolContext {
  messageId: string;
  conversationId: string;
}

export interface ActiveToolInfo {
  toolName: string;
  status: string;
}

// Props interfaces
export interface MessagesProps {
  messages: DisplayMessage[];
  isLoading?: boolean;
  onForgetFrom?: (messageId: string) => void;
  conversationId?: string;
  isStreaming?: boolean;
  canStop?: boolean;
  onStop?: () => void;
  activeTools?: unknown[];
  onResendMessage?: (message: Message) => void;
  onNewChatFromHere?: (messageId: string) => void;
  onAskUserSubmit?: (response: string | string[]) => void;
}