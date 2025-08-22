import { useState, useCallback } from "react";
import { useQuery } from "@tanstack/react-query";
import { API_BASE_URL } from "@/lib/url";

// Types matching the backend
export interface Message {
  id: string;
  content: string;
  role: "user" | "assistant" | "system";
  createdAt?: string;
  clay_tools_used?: string[];
  processing_time_ms?: number;
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

/**
 * Hook for managing conversation-specific context
 */
export function useConversationContext(conversationId: string | null) {
  const {
    data: context,
    isLoading,
    error,
    refetch,
  } = useQuery<ConversationContext>({
    queryKey: ["conversationContext", conversationId],
    queryFn: async () => {
      if (!conversationId) throw new Error("Conversation ID required");

      const response = await fetch(
        `${API_BASE_URL}/conversations/${conversationId}/context`
      );
      if (!response.ok) throw new Error("Failed to fetch conversation context");

      return response.json();
    },
    enabled: !!conversationId,
    staleTime: 1000 * 60 * 5, // Context is fresh for 5 minutes
    gcTime: 1000 * 60 * 30, // Keep in cache for 30 minutes
  });

  return {
    context,
    isLoading,
    error,
    refresh: refetch,
    // Derived convenience properties
    hasLongHistory: context ? context.total_messages > 20 : false,
    contextStrategy: context?.context_strategy,
    activeTools: context?.available_tools.filter((t) => t.applicable) || [],
    dataSourceCount: context?.data_sources.length || 0,
  };
}

/**
 * Hook for managing project-wide context
 */
export function useProjectContext(projectId: string | null) {
  const {
    data: projectContext,
    isLoading,
    error,
    refetch,
  } = useQuery<ProjectContextResponse>({
    queryKey: ["projectContext", projectId],
    queryFn: async () => {
      if (!projectId) throw new Error("Project ID required");

      const response = await fetch(
        `${API_BASE_URL}/projects/${projectId}/context`
      );
      if (!response.ok) throw new Error("Failed to fetch project context");

      return response.json();
    },
    enabled: !!projectId,
    staleTime: 1000 * 60 * 10, // Project context fresh for 10 minutes
    gcTime: 1000 * 60 * 60, // Keep in cache for 1 hour
  });

  return {
    projectContext,
    isLoading,
    error,
    refresh: refetch,
    // Derived properties
    dataSourcesByType:
      projectContext?.data_sources.reduce((acc, ds) => {
        acc[ds.source_type] = (acc[ds.source_type] || 0) + 1;
        return acc;
      }, {} as Record<string, number>) || {},
    toolsByCategory:
      projectContext?.available_tools.reduce((acc, tool) => {
        if (!acc[tool.category]) acc[tool.category] = [];
        acc[tool.category].push(tool);
        return acc;
      }, {} as Record<string, ToolContext[]>) || {},
    recentConversations:
      projectContext?.recent_activity.filter(
        (a) => a.activity_type === "message" && a.conversation_id
      ) || [],
  };
}

/**
 * Main chat hook with backend integration
 */
export function useClayChat(projectId: string, conversationId?: string) {
  const [messages, setMessages] = useState<Message[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load context for the conversation or project
  const { context: conversationContext } = useConversationContext(
    conversationId || null
  );
  const { projectContext } = useProjectContext(projectId);

  const sendMessage = useCallback(
    async (content: string) => {
      if (!projectId) {
        setError("Project ID is required");
        return;
      }

      setIsLoading(true);
      setError(null);

      try {
        // Add user message to local state immediately
        const userMessage: Message = {
          id: `temp-${Date.now()}`,
          role: "user",
          content,
          createdAt: new Date().toISOString(),
        };
        setMessages((prev) => [...prev, userMessage]);

        const response = await fetch(`${API_BASE_URL}/chat`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            messages: [
              {
                id: `msg-${Date.now()}`,
                role: "user",
                content,
              },
            ],
            project_id: projectId,
            conversation_id: conversationId,
          }),
        });

        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }

        const assistantResponse = await response.json();

        // Add assistant response to messages
        const assistantMessage: Message = {
          id: assistantResponse.id,
          role: "assistant",
          content: assistantResponse.content,
          createdAt: assistantResponse.createdAt,
        };
        setMessages((prev) => [...prev, assistantMessage]);
      } catch (err) {
        setError(err instanceof Error ? err.message : "An error occurred");
        console.error("Chat error:", err);
      } finally {
        setIsLoading(false);
      }
    },
    [projectId, conversationId]
  );

  return {
    messages,
    sendMessage,
    isLoading,
    error,
    // Enhanced context information
    conversationContext,
    projectContext,
    // Smart context features
    hasDataSources: (projectContext?.data_sources.length || 0) > 0,
    availableTools:
      conversationContext?.available_tools ||
      projectContext?.available_tools ||
      [],
    contextStrategy: conversationContext?.context_strategy,
    canUseAdvancedAnalysis:
      (projectContext?.data_sources.length || 0) > 0 &&
      (conversationContext?.available_tools.some((t) => t.applicable) || false),
  };
}

/**
 * Hook for tool recommendations based on current context
 */
export function useToolRecommendations(
  conversationContext?: ConversationContext,
  userQuery?: string
) {
  const [recommendations, setRecommendations] = useState<ToolContext[]>([]);

  // Simple recommendation logic based on user query and available tools
  const generateRecommendations = useCallback(() => {
    if (!conversationContext || !userQuery) {
      setRecommendations([]);
      return;
    }

    const queryLower = userQuery.toLowerCase();
    const applicable = conversationContext.available_tools.filter(
      (tool) => tool.applicable
    );

    const scored = applicable
      .map((tool) => ({
        tool,
        score: calculateToolRelevance(tool, queryLower, conversationContext),
      }))
      .filter((item) => item.score > 0)
      .sort((a, b) => b.score - a.score)
      .slice(0, 5)
      .map((item) => item.tool);

    setRecommendations(scored);
  }, [conversationContext, userQuery]);

  // Update recommendations when context or query changes
  useState(() => {
    generateRecommendations();
  });

  return recommendations;
}

// Helper function to calculate tool relevance
function calculateToolRelevance(
  tool: ToolContext,
  queryLower: string,
  context: ConversationContext
): number {
  let score = 0;

  // Base score for applicable tools
  if (tool.applicable) score += 1;

  // Boost score based on category matches
  const categoryKeywords = {
    time_series: ["time", "trend", "forecast", "seasonal", "over time"],
    statistics: [
      "average",
      "mean",
      "correlation",
      "distribution",
      "statistics",
    ],
    data_quality: ["quality", "missing", "clean", "duplicate", "validate"],
    sql: ["query", "select", "join", "table", "database"],
    data_exploration: ["explore", "show", "describe", "summary", "overview"],
  };

  const keywords =
    categoryKeywords[tool.category as keyof typeof categoryKeywords];
  if (keywords) {
    const matches = keywords.filter((keyword) => queryLower.includes(keyword));
    score += matches.length * 2;
  }

  // Boost based on data source types
  const hasTimeSeriesData = context.data_sources.some(
    (ds) => ds.schema_info?.has_time_column
  );
  const hasNumericalData = context.data_sources.some(
    (ds) => ds.schema_info?.numerical_columns?.length > 0
  );

  if (tool.category === "time_series" && hasTimeSeriesData) score += 3;
  if (tool.category === "statistics" && hasNumericalData) score += 3;

  return score;
}
