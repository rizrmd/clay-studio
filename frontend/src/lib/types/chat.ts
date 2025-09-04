export type PROJECT_ID = string;
export type CONVERSATION_ID = string;
export type MessageRole = "user" | "assistant" | "system";
export type FileUploadResponse = {
  id: string;
  file_name: string;
  original_name: string;
  file_path: string;
  file_size: number;
  mime_type?: string;
  description?: string;
  auto_description?: string;
  created_at: string;
  is_text_file: boolean;
  preview?: string;
};

export type ToolUsage = {
  id: string;
  message_id: string;
  tool_name: string;
  tool_use_id?: string;
  parameters?: Record<string, unknown>;
  output?: unknown;
  execution_time_ms?: number;
  createdAt?: string;
};

export type Message = {
  id: string;
  content: string;
  role: MessageRole;
  createdAt?: string;
  processing_time_ms?: number;
  file_attachments?: FileUploadResponse[];
  tool_usages?: ToolUsage[];
  todoWrite?: unknown; // Legacy field for TodoWrite functionality
};

export type Conversation = {
  id: CONVERSATION_ID;
  project_id: string;
  title?: string;
  created_at: string;
  updated_at: string;
  message_count: number;
  is_title_manually_set?: boolean;
  messages: Message[];
};

export interface AskUserData {
  interaction_id: string;
  interaction_type: string;
  title: string;
  data: any;
  options?: any;
}

export interface ToolContext {
  messageId: string;
  conversationId: string;
}

export interface McpServer {
  name: string;
  status: "connected" | "disconnected" | "error";
}

export interface ProgressMessageContent {
  type: "progress";
  content: {
    apiKeySource: string;
    mcp_servers: McpServer[];
    model: string;
    output_style: string;
    permissionMode: string;
    session_id: string;
    slash_commands: string[];
    subtype: string;
    tools: string[];
    uuid: string;
  };
  conversation_id: string;
}

// Utility function to extract tool names from a message
export function getToolNamesFromMessage(message: Message): string[] {
  if (!message.tool_usages || message.tool_usages.length === 0) {
    return [];
  }
  return message.tool_usages.map(usage => usage.tool_name);
}
