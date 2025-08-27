import { useState, useCallback } from "react";
import { logger } from "@/lib/logger";
import { API_BASE_URL } from "@/lib/url";
import { ToolUsage } from "@/types/chat";

interface UseToolUsageReturn {
  fetchToolUsage: (messageId: string, toolName: string) => Promise<ToolUsage | null>;
  fetchAllToolUsages: (messageId: string) => Promise<ToolUsage[]>;
  loading: boolean;
  error: string | null;
}

export function useToolUsage(): UseToolUsageReturn {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchToolUsage = useCallback(async (messageId: string, toolName: string): Promise<ToolUsage | null> => {
    setLoading(true);
    setError(null);
    
    try {
      const response = await fetch(
        `${API_BASE_URL}/messages/${messageId}/tool-usage/${encodeURIComponent(toolName)}`,
        {
          credentials: "include",
        }
      );

      if (!response.ok) {
        if (response.status === 404) {
          // Tool usage not found - this is ok, might not be captured yet
          return null;
        }
        throw new Error(`Failed to fetch tool usage: ${response.status}`);
      }

      const data = await response.json();
      return data as ToolUsage;
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to fetch tool usage";
      setError(message);
      logger.error("ToolUsage: Error fetching tool usage:", err);
      return null;
    } finally {
      setLoading(false);
    }
  }, []);

  const fetchAllToolUsages = useCallback(async (messageId: string): Promise<ToolUsage[]> => {
    setLoading(true);
    setError(null);
    
    try {
      const response = await fetch(
        `${API_BASE_URL}/messages/${messageId}/tool-usages`,
        {
          credentials: "include",
        }
      );

      if (!response.ok) {
        throw new Error(`Failed to fetch tool usages: ${response.status}`);
      }

      const data = await response.json();
      return data as ToolUsage[];
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to fetch tool usages";
      setError(message);
      logger.error("ToolUsage: Error fetching tool usages:", err);
      return [];
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    fetchToolUsage,
    fetchAllToolUsages,
    loading,
    error,
  };
}