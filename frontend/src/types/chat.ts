// Types for chat functionality

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
  createdAt?: string;
  clay_tools_used?: string[];
  processing_time_ms?: number;
  file_attachments?: FileAttachment[];
}

export interface ConversationContext {
  conversation_id: string;
  project_id: string;
  messages: Message[];
  summary?: ConversationSummary;
  data_sources: DataSourceContext[];
  available_tools: ToolContext[];
  project_settings: ProjectSettings;
  total_messages: number;
  context_strategy: "FullHistory" | "SummaryWithRecent" | "OnlyRecent";
}

export interface ConversationSummary {
  id: string;
  summary_text: string;
  message_count: number;
  summary_type: string;
  created_at: string;
}

export interface DataSourceContext {
  id: string;
  name: string;
  source_type: string;
  connection_config: any;
  schema_info?: any;
  preview_data?: any;
  table_list?: string[];
  last_tested_at?: string;
  is_active: boolean;
}

export interface ToolContext {
  name: string;
  category: string;
  description: string;
  parameters: any;
  applicable: boolean;
  usage_examples: string[];
}

export interface ProjectSettings {
  project_id: string;
  name: string;
  settings: any;
  organization_settings: any;
  default_analysis_preferences: AnalysisPreferences;
}

export interface AnalysisPreferences {
  auto_suggest_visualizations: boolean;
  preferred_chart_types: string[];
  default_aggregation_functions: string[];
  enable_statistical_insights: boolean;
  context_length_preference: string;
}

export interface ProjectContextResponse {
  project_id: string;
  project_settings: ProjectSettings;
  data_sources: DataSourceContext[];
  available_tools: ToolContext[];
  total_conversations: number;
  recent_activity: RecentActivity[];
}

export interface RecentActivity {
  activity_type: string;
  description: string;
  timestamp: string;
  conversation_id?: string;
}