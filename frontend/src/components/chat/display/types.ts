// Re-export types from lib/types for backwards compatibility
import type { 
  Message as BaseMessage,
  MessageRole,
  Conversation,
  ToolUsage,
  FileUploadResponse,
  AskUserData,
  PROJECT_ID,
  CONVERSATION_ID
} from "@/lib/types/chat";

export type { MessageRole, Conversation, ToolUsage, AskUserData, PROJECT_ID, CONVERSATION_ID };
export type { FileUploadResponse as FileAttachment };
export type Message = BaseMessage;

// Re-export the function
export { getToolNamesFromMessage } from "@/lib/types/chat";

// Additional display-specific types
export interface MessageWithUI extends Message {
  isGenerating?: boolean;
  isError?: boolean;
}

export interface AskUserOption {
  value: string;
  label: string;
  description?: string;
}

export interface DisplayMessage extends Message {
  isQueued?: boolean;
  isEditing?: boolean;
  ask_user?: {
    prompt_type: "checkbox" | "input" | "buttons";
    title: string;
    options?: AskUserOption[];
    input_type?: "text" | "password";
    placeholder: string;
    tool_use_id: string;
  };
}

export interface ToolContext {
  messageId: string;
  conversationId: string;
  category?: string;
  name?: string;
}

export interface ActiveToolInfo {
  toolName: string;
  tool_name: string;
  tool_usage_id: string;
  execution_time_ms?: number;
  started_at?: string;
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