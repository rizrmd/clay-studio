/**
 * This file re-exports the new redesigned chat hook implementation.
 * The new architecture uses:
 * 
 * - ConversationManager for atomic state updates
 * - Event-driven communication to prevent race conditions
 * - Centralized abort controller management
 * - Proper conversation isolation to prevent message bleeding
 */

// Use the new implementation by default
export { useChat as useValtioChat } from "./use-chat";

// Old implementation removed with SSE streaming

// Keep context hooks as they are
export { useConversationContext, useProjectContext } from "./chat/use-context";

// Re-export types
export type {
  FileAttachment,
  Message,
  ConversationContext,
  ConversationSummary,
  DataSourceContext,
  ToolContext,
  ProjectSettings,
  AnalysisPreferences,
  ProjectContextResponse,
  RecentActivity,
} from "../types/chat";