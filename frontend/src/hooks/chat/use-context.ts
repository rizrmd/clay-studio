import { useNavigate, useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/utils/api";
import { cacheProjectContext } from "@/store/chat-store";

/**
 * Hook for managing conversation-specific context using Valtio
 */
export function useConversationContext(conversationId: string | null) {
  const navigate = useNavigate();
  const { projectId } = useParams<{ projectId: string }>();

  const {
    data: context,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ["conversationContext", conversationId],
    queryFn: async () => {
      if (!conversationId || conversationId === 'new') {
        // Return empty context for new conversations
        return {
          conversation_id: 'new',
          project_id: projectId || '',
          messages: [],
          summary: null,
          data_sources: [],
          available_tools: [],
          project_settings: null,
          total_messages: 0,
          context_strategy: 'full_history',
        };
      }

      const response = await api.fetchStream(
        `/conversations/${conversationId}/context`
      );

      if (!response.ok) {
        // If conversation doesn't exist (404), redirect to /new
        if (response.status === 404 && projectId) {
          navigate(`/chat/${projectId}/new`, { replace: true });
        }
        throw new Error(`Failed to fetch conversation context: ${response.status}`);
      }

      return response.json();
    },
    enabled: !!conversationId,
  });

  return {
    context,
    isLoading,
    error,
    refresh: refetch,
    // Derived convenience properties
    hasLongHistory: context ? context.total_messages > 20 : false,
    contextStrategy: context?.context_strategy,
    activeTools:
      context?.available_tools.filter((t: any) => t.applicable) || [],
    dataSourceCount: context?.data_sources.length || 0,
  };
}

/**
 * Hook for managing project-wide context using Valtio
 */
export function useProjectContext(projectId: string | null) {
  const {
    data: projectContext,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ["projectContext", projectId],
    queryFn: async () => {
      if (!projectId) throw new Error("Project ID required");

      const response = await api.fetchStream(
        `/projects/${projectId}/context`
      );
      if (!response.ok) throw new Error("Failed to fetch project context");

      const data = await response.json();

      // Cache the context in our store
      cacheProjectContext(projectId, data);

      return data;
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
      projectContext?.data_sources.reduce(
        (acc: Record<string, number>, ds: any) => {
          acc[ds.source_type] = (acc[ds.source_type] || 0) + 1;
          return acc;
        },
        {} as Record<string, number>
      ) || {},
    toolsByCategory:
      projectContext?.available_tools.reduce(
        (acc: Record<string, any[]>, tool: any) => {
          if (!acc[tool.category]) acc[tool.category] = [];
          acc[tool.category].push(tool);
          return acc;
        },
        {} as Record<string, any[]>
      ) || {},
    recentConversations:
      projectContext?.recent_activity.filter(
        (a: any) => a.activity_type === "message" && a.conversation_id
      ) || [],
  };
}