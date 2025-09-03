import { useCallback, useEffect, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { useSnapshot } from "valtio";
import { api } from "@/lib/utils/api";
import { logger } from "@/lib/utils/logger";
import { sidebarStore, sidebarActions } from "@/store/sidebar-store";
import { useAuth } from "@/hooks/use-auth";

interface Conversation {
  id: string;
  project_id: string;
  title: string;
  message_count: number;
  created_at: string;
  updated_at: string;
  is_title_manually_set?: boolean;
}

export function useConversations(projectId?: string, currentConversationId?: string) {
  const sidebarSnapshot = useSnapshot(sidebarStore);
  const { isAuthenticated, isSetupComplete } = useAuth();
  const navigate = useNavigate();
  const fetchTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const lastFetchTimeRef = useRef<number>(0);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (fetchTimeoutRef.current) {
        clearTimeout(fetchTimeoutRef.current);
      }
    };
  }, []);

  // Shared function to fetch conversations with debouncing
  const fetchConversations = useCallback(async (showLoadingState = true, forceImmediate = false) => {
    if (!projectId || !isAuthenticated || !isSetupComplete) return;

    // Implement debouncing for background refreshes (not for initial load)
    if (!forceImmediate && !showLoadingState) {
      const now = Date.now();
      const timeSinceLastFetch = now - lastFetchTimeRef.current;

      // If we fetched less than 1 second ago, debounce
      if (timeSinceLastFetch < 1000) {
        // Clear any existing timeout
        if (fetchTimeoutRef.current) {
          clearTimeout(fetchTimeoutRef.current);
        }

        // Set a new timeout to fetch after the debounce period
        fetchTimeoutRef.current = setTimeout(() => {
          fetchConversations(false, true); // Force immediate after debounce
        }, 1000 - timeSinceLastFetch);

        return;
      }
    }

    // Record the fetch time
    lastFetchTimeRef.current = Date.now();

    // Only show sidebarSnapshot.loading state if requested and we don't have conversations yet
    if (showLoadingState && sidebarSnapshot.conversations.length === 0) {
      sidebarActions.setLoading(true);
    }
    sidebarActions.setError(null);
    try {
      const url = `/conversations?project_id=${encodeURIComponent(
        projectId
      )}`;

      const response = await api.fetchStream(url);

      if (!response.ok) {
        throw new Error(
          `Failed to fetch conversations: ${response.status} - ${response.statusText}`
        );
      }

      const data = await response.json();
      sidebarActions.setConversations(data);

      // Validate current conversation ID against fetched conversations
      // Only redirect if we have an invalid conversation ID and there are existing conversations
      // OR if there are no conversations at all (empty state should go to /new)
      if (
        currentConversationId &&
        !data.find((conv: Conversation) => conv.id === currentConversationId)
      ) {
        // Current conversation ID doesn't exist in the list, redirect to new
        navigate("/projects", { replace: true });
      }
    } catch (err) {
      if (showLoadingState) {
        sidebarActions.setError(
          err instanceof Error ? err.message : "Failed to load conversations"
        );
      } else {
        logger.error(
          "ConversationSidebar: Failed to refresh conversations:",
          err
        );
      }
    } finally {
      if (showLoadingState) {
        sidebarActions.setLoading(false);
      }
    }
  }, [projectId, isAuthenticated, isSetupComplete, currentConversationId, navigate, sidebarSnapshot.conversations.length]);

  // Initial fetch when component mounts or auth changes
  useEffect(() => {
    if (!projectId || !isAuthenticated || !isSetupComplete) return;
    fetchConversations(true, true); // true = show sidebarSnapshot.loading, true = force immediate
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, isAuthenticated, isSetupComplete]); // Removed currentConversationId from deps to prevent refetch on navigation

  return {
    fetchConversations,
  };
}