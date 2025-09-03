import { AskUserData } from "@/types/chat";
import type { ActiveToolInfo } from "@/store/chat/types";

export interface FileAttachment {
  id: string;
  file_name: string;
  original_name: string;
  file_path: string;
  file_size: number;
  mime_type?: string;
  description?: string;
  auto_description?: string;
}

export interface Message {
  id: string;
  content: string;
  role: "user" | "assistant" | "system";
  createdAt: string | Date;
  file_attachments?: FileAttachment[];
  clay_tools_used?: string[];
  tool_usages?: any[];
  ask_user?: AskUserData;
  todoWrite?: {
    todos: Array<{
      content: string;
      status: "pending" | "in_progress" | "completed";
    }>;
  };
}

export interface QueuedMessage {
  id: string;
  content: string;
  files: File[];
  timestamp: Date;
}

export interface DisplayMessage extends Message {
  isQueued?: boolean;
  queuePosition?: number;
  isEditing?: boolean;
  processing_time_ms?: number;
}

export interface MessagesProps {
  messages: Message[];
  isLoading?: boolean;
  onForgetFrom?: (messageId: string) => void;
  conversationId?: string;
  messageQueue?: QueuedMessage[];
  onEditQueued?: (messageId: string, newContent: string) => void;
  onCancelQueued?: (messageId: string) => void;
  isProcessingQueue?: boolean;
  isStreaming?: boolean;
  canStop?: boolean;
  onStop?: () => void;
  activeTools?: ActiveToolInfo[];
  onResendMessage?: (message: Message) => void;
  onNewChatFromHere?: (messageId: string) => void;
  onAskUserSubmit?: (response: string | string[]) => void;
}