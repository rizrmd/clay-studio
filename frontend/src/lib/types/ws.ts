import type { Message, ToolUsage } from './chat'

// Server message types (received from backend)
export type ServerMessage =
  | {
      type: "connected";
      user_id: string;
      authenticated: boolean;
      client_id?: string;
      role?: string;
    }
  | { type: "authentication_required" }
  | { type: "subscribed"; project_id: string; conversation_id?: string }
  | {
      type: "conversation_redirect";
      old_conversation_id: string;
      new_conversation_id: string;
    }
  | { type: "pong" }
  | { type: "start"; id: string; conversation_id: string }
  | { 
      type: "progress"; 
      content: {
        // Initial progress message with system info
        apiKeySource?: string;
        mcp_servers?: Array<{name: string; status: string}>;
        model?: string;
        output_style?: string;
        permissionMode?: string;
        session_id?: string;
        slash_commands?: string[];
        subtype?: "init" | "success" | string;
        tools?: string[];
        content_type?: string;
        uuid?: string;
        // Message progress
        message?: {
          content?: Array<{text: string; type: string}>;
          id?: string;
          model?: string;
          role?: string;
          stop_reason?: string | null;
          stop_sequence?: string | null;
          type?: string;
          usage?: any;
        };
        parent_tool_use_id?: string | null;
        // Result progress
        duration_api_ms?: number;
        duration_ms?: number;
        is_error?: boolean;
        num_turns?: number;
        permission_denials?: any[];
        result?: string;
        total_cost_usd?: number;
        usage?: any;
      }; 
      conversation_id: string 
    }
  | {
      type: "tool_use";
      tool: string;
      tool_usage_id: string;
      conversation_id: string;
    }
  | {
      type: "tool_complete";
      tool: string;
      tool_usage_id: string;
      execution_time_ms: number;
      output?: any;
      conversation_id: string;
    }
  | { type: "content"; content: string; conversation_id: string }
  | {
      type: "complete";
      id: string;
      conversation_id: string;
      processing_time_ms: number;
      tool_usages?: ToolUsage[];
    }
  | { type: "error"; error: string; conversation_id: string }
  | {
      type: "conversation_activity";
      conversation_id: string;
      user_id: string;
      user_name: string;
      activity_type: string;
      timestamp: string;
      message_preview?: string;
    }
  | { type: "conversation_list"; conversations: import('./chat').Conversation[] }
  | { type: "conversation_created"; conversation: import('./chat').Conversation }
  | { type: "conversation_details"; conversation: import('./chat').Conversation }
  | { type: "conversation_updated"; conversation: import('./chat').Conversation }
  | { type: "conversation_deleted"; conversation_id: string }
  | {
      type: "conversations_bulk_deleted";
      conversation_ids: string[];
      failed_ids: string[];
    }
  | {
      type: "conversation_messages";
      conversation_id: string;
      messages: Message[];
    };

// Client message types (sent to backend)
export type ClientMessage =
  | { type: "subscribe"; project_id: string; conversation_id?: string }
  | { type: "unsubscribe" }
  | { type: "ping" }
  | {
      type: "ask_user_response";
      conversation_id: string;
      interaction_id: string;
      response: any;
    }
  | { type: "stop_streaming"; conversation_id: string }
  | {
      type: "send_message";
      project_id: string;
      conversation_id: string;
      content: string;
      file_ids?: string[];
    }
  | { type: "create_conversation"; project_id: string; title?: string; first_message?: string; file_ids?: string[] }
  | { type: "list_conversations"; project_id: string }
  | { type: "get_conversation"; conversation_id: string }
  | {
      type: "update_conversation";
      conversation_id: string;
      title?: string;
    }
  | { type: "delete_conversation"; conversation_id: string }
  | { type: "bulk_delete_conversations"; conversation_ids: string[] }
  | { type: "get_conversation_messages"; conversation_id: string }
  | { type: "retry_last_message"; project_id: string; conversation_id: string };

export interface StreamingState {
  messageId: string;
  partialContent: string;
  activeTools: Array<{ tool: string; toolUsageId: string; startTime: number }>;
  isComplete: boolean;
}
