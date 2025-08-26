import { useCallback, useEffect, useRef } from "react";
import { useSnapshot } from "valtio";
import { useQuery } from "@tanstack/react-query";
import { API_BASE_URL } from "@/lib/url";
import {
  store,
  getConversationState,
  setActiveConversation,
  updateConversationMessages,
  addMessage,
  updateLastMessage,
  setConversationLoading,
  setConversationError,
  setConversationStreaming,
  setConversationUploadedFiles,
  addConversationUploadedFile,
  setConversationAbortController,
  getConversationAbortController,
  setConversationForgotten,
  setConversationContext,
  cacheProjectContext,
  addToMessageQueue,
  removeFromMessageQueue,
  updateMessageInQueue,
  addActiveTool,
  clearActiveTools,
} from "../store";

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

/**
 * Hook for managing conversation-specific context using Valtio
 */
export function useConversationContext(conversationId: string | null) {
  // const snapshot = useSnapshot(store);

  const {
    data: context,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ["conversationContext", conversationId],
    queryFn: async () => {
      if (!conversationId) throw new Error("Conversation ID required");

      const response = await fetch(
        `${API_BASE_URL}/conversations/${conversationId}/context`,
        { credentials: "include" }
      );
      if (!response.ok) throw new Error("Failed to fetch conversation context");

      const data = await response.json();

      // Cache the context in our store
      if (conversationId) {
        setConversationContext(conversationId, data);
      }

      return data;
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
    activeTools:
      context?.available_tools.filter((t: any) => t.applicable) || [],
    dataSourceCount: context?.data_sources.length || 0,
  };
}

/**
 * Hook for managing project-wide context using Valtio
 */
export function useProjectContext(projectId: string | null) {
  // const snapshot = useSnapshot(store);

  const {
    data: projectContext,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ["projectContext", projectId],
    queryFn: async () => {
      if (!projectId) throw new Error("Project ID required");

      const response = await fetch(
        `${API_BASE_URL}/projects/${projectId}/context`,
        { credentials: "include" }
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

/**
 * Main chat hook with Valtio state management and streaming support
 */
export function useValtioChat(projectId: string, conversationId?: string) {
  const snapshot = useSnapshot(store);
  const currentConversationId = conversationId || "new";

  // Get conversation state from store
  const conversationState = snapshot.conversations[currentConversationId];
  const messages = conversationState?.messages || [];
  const isLoadingMessages = conversationState?.isLoadingMessages || false;
  const error = conversationState?.error || null;
  const isStreaming = conversationState?.isStreaming || false;
  const uploadedFiles = conversationState?.uploadedFiles || [];
  const forgottenAfterMessageId =
    conversationState?.forgottenAfterMessageId || null;
  const forgottenCount = conversationState?.forgottenCount || 0;
  const currentAbortController =
    conversationState?.currentAbortController || null;

  // Set active conversation
  useEffect(() => {
    setActiveConversation(currentConversationId);
  }, [currentConversationId]);

  // Function to load uploaded files for a conversation
  const loadUploadedFiles = useCallback(async () => {
    if (!projectId || !conversationId || conversationId === "new") {
      setConversationUploadedFiles(currentConversationId, []);
      return;
    }

    try {
      const clientId = localStorage.getItem("activeClientId");
      if (!clientId) return;

      const response = await fetch(
        `${API_BASE_URL}/uploads?client_id=${clientId}&project_id=${projectId}&conversation_id=${conversationId}`,
        { credentials: "include" }
      );

      if (response.ok) {
        const files = await response.json();
        setConversationUploadedFiles(currentConversationId, files);
      }
    } catch (err) {
      // Failed to load uploaded files - silently continue
    }
  }, [projectId, conversationId, currentConversationId]);

  // Load messages when conversation changes
  useEffect(() => {
    if (!conversationId) return;

    if (conversationId === "new") {
      // Initialize empty state for new conversation
      const state = getConversationState("new");
      // Always reset when navigating to 'new' to ensure clean slate
      state.messages = [];
      state.error = null;
      state.forgottenAfterMessageId = null;
      state.forgottenCount = 0;
      state.uploadedFiles = [];
      state.isLoading = false;
      state.isStreaming = false;
      state.messageQueue = [];
      state.activeTools = [];
      // Clear active conversation ID when navigating to new
      store.activeConversationId = null;
    } else {
      // Check if we already have messages for this conversation
      const existingState = snapshot.conversations[conversationId];
      if (existingState && existingState.messages.length > 0) {
        // We already have messages, no need to reload
        return;
      }

      // Load messages from database
      const state = getConversationState(conversationId);
      state.messages = [];
      state.error = null;
      state.isLoadingMessages = true;

      fetch(`${API_BASE_URL}/conversations/${conversationId}/messages`, {
        credentials: "include",
      })
        .then(async (res) => {
          if (!res.ok) {
            let errorMessage = "Failed to load conversation";

            switch (res.status) {
              case 404:
                errorMessage =
                  "This conversation doesn't exist or has been deleted";
                break;
              case 403:
                errorMessage =
                  "You don't have permission to access this conversation";
                break;
              case 500:
                errorMessage =
                  "Server error while loading the conversation. Please try again";
                break;
              default:
                try {
                  const errorData = await res.json();
                  errorMessage = errorData.message || errorMessage;
                } catch {
                  errorMessage = `Failed to load conversation (${res.status})`;
                }
            }

            throw new Error(errorMessage);
          }
          return res.json();
        })
        .then((data) => {
          updateConversationMessages(conversationId, [...data]);
          setConversationError(conversationId, null);
          const convState = getConversationState(conversationId);
          convState.isLoadingMessages = false;
          loadUploadedFiles();
        })
        .catch((err) => {
          setConversationError(conversationId, err.message);
          const convState = getConversationState(conversationId);
          convState.isLoadingMessages = false;
        });

      // Check forgotten status
      fetch(`${API_BASE_URL}/conversations/${conversationId}/forget-after`, {
        credentials: "include",
      })
        .then((res) => {
          if (res.ok) {
            return res.json();
          }
          return null;
        })
        .then((data) => {
          if (data && data.has_forgotten) {
            setConversationForgotten(
              conversationId,
              data.forgotten_after_message_id,
              data.forgotten_count || 0
            );
          }
        })
        .catch(() => {
          // Silently handle forgotten status errors
        });
    }
  }, [conversationId, projectId, loadUploadedFiles]);

  // Load context for the conversation or project
  const { context: conversationContext } = useConversationContext(
    conversationId && conversationId !== "new" ? conversationId : null
  );
  const { projectContext } = useProjectContext(projectId);

  // Function to restore forgotten messages
  const restoreForgottenMessages = useCallback(async () => {
    if (!currentConversationId || !forgottenAfterMessageId) return;

    try {
      const response = await fetch(
        `${API_BASE_URL}/conversations/${currentConversationId}/forget-after`,
        {
          method: "DELETE",
          credentials: "include",
        }
      );

      if (!response.ok) {
        throw new Error("Failed to restore messages");
      }

      // Clear forgotten state
      setConversationForgotten(currentConversationId, null, 0);

      // Reload all messages
      const messagesResponse = await fetch(
        `${API_BASE_URL}/conversations/${currentConversationId}/messages`,
        {
          credentials: "include",
        }
      );

      if (messagesResponse.ok) {
        const allMessages = await messagesResponse.json();
        updateConversationMessages(currentConversationId, [...allMessages]);
      }
    } catch (err) {
      setConversationError(
        currentConversationId,
        err instanceof Error ? err.message : "Failed to restore messages"
      );
    }
  }, [currentConversationId, forgottenAfterMessageId]);

  const resendMessage = useCallback(
    async (content: string) => {
      if (!projectId) {
        setConversationError(currentConversationId, "Project ID is required");
        return;
      }

      // Check if we should queue the message instead
      const state = getConversationState(currentConversationId);
      if (state.isStreaming || state.isProcessingQueue) {
        // Add to queue instead of sending immediately
        addToMessageQueue(currentConversationId, {
          id: `queue-${Date.now()}`,
          content,
          files: [],
          timestamp: new Date(),
        });
        return;
      }

      // Remove the last assistant message if it exists
      // This allows us to "resend" by regenerating the response to the existing user message
      const currentMessages = state.messages;
      if (
        currentMessages.length > 0 &&
        currentMessages[currentMessages.length - 1].role === "assistant"
      ) {
        // Remove the last assistant message
        state.messages = currentMessages.slice(0, -1);
      }

      // Create AbortController for this request
      const abortController = new AbortController();
      setConversationAbortController(currentConversationId, abortController);

      setConversationLoading(currentConversationId, true);
      setConversationError(currentConversationId, null);

      try {
        // Always use streaming endpoint
        setConversationStreaming(currentConversationId, true);
        let assistantContent = "";

        const response = await fetch(`${API_BASE_URL}/chat/stream`, {
          method: "POST",
          credentials: "include",
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
            conversation_id: currentConversationId,
          }),
          signal: abortController.signal,
        });

        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }

        const reader = response.body?.getReader();
        const decoder = new TextDecoder();

        if (!reader) {
          throw new Error("No response body");
        }

        let buffer = "";

        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          const chunk = decoder.decode(value, { stream: true });
          buffer += chunk;
          const lines = buffer.split("\n");

          buffer = lines.pop() || "";

          for (const line of lines) {
            const trimmedLine = line.trim();
            if (trimmedLine.startsWith("data:")) {
              const data = trimmedLine.slice(5).trim();
              if (data === "[DONE]") continue;
              if (!data) continue;

              try {
                const event = JSON.parse(data);

                switch (event.type) {
                  case "start":
                    // Update conversation ID when we get the real ID from backend
                    if (
                      event.conversation_id &&
                      event.conversation_id !== "new" &&
                      currentConversationId === "new"
                    ) {
                      // Transfer the state from 'new' to the real conversation ID BEFORE setting active
                      const newState = getConversationState("new");
                      const realState = getConversationState(
                        event.conversation_id
                      );
                      // Deep copy messages to avoid reference issues
                      realState.messages = [...newState.messages];
                      realState.isLoading = newState.isLoading;
                      realState.isStreaming = newState.isStreaming;
                      realState.error = newState.error;
                      realState.uploadedFiles = [...newState.uploadedFiles];

                      // Only set active conversation AFTER state is transferred
                      setActiveConversation(event.conversation_id);
                      
                      // Dispatch a custom event to notify the sidebar to refresh
                      window.dispatchEvent(new CustomEvent('conversation-created', {
                        detail: { conversationId: event.conversation_id, projectId }
                      }));
                    }
                    break;
                  case "progress":
                    try {
                      const streamJson = JSON.parse(event.content);

                      if (
                        streamJson.type === "text" ||
                        streamJson.type === "progress"
                      ) {
                        const textContent =
                          streamJson.text || streamJson.content || "";
                        if (textContent) {
                          assistantContent += textContent;
                          // Use the active conversation ID from the store, fallback to current if not set
                          const activeId =
                            store.activeConversationId || currentConversationId;
                          const currentMessages =
                            getConversationState(activeId).messages;
                          const lastMessage =
                            currentMessages[currentMessages.length - 1];
                          if (lastMessage && lastMessage.role === "assistant") {
                            updateLastMessage(activeId, {
                              content: assistantContent,
                            });
                          } else {
                            addMessage(activeId, {
                              id: `streaming-${Date.now()}`,
                              role: "assistant",
                              content: assistantContent,
                              createdAt: new Date().toISOString(),
                            });
                          }
                        }
                      }
                    } catch (parseError) {
                      // Skip non-JSON messages
                    }
                    break;
                  case "tool_use":
                    if (event.tool) {
                      const activeId =
                        store.activeConversationId || currentConversationId;
                      addActiveTool(activeId, event.tool);
                    }
                    break;
                  case "content":
                    if (event.content) {
                      const activeId =
                        store.activeConversationId || currentConversationId;
                      const currentMessages =
                        getConversationState(activeId).messages;
                      const lastMessage =
                        currentMessages[currentMessages.length - 1];
                      if (lastMessage && lastMessage.role === "assistant") {
                        updateLastMessage(activeId, { content: event.content });
                      } else if (!assistantContent) {
                        addMessage(activeId, {
                          id: `streaming-${Date.now()}`,
                          role: "assistant",
                          content: event.content,
                          createdAt: new Date().toISOString(),
                        });
                      }
                    }
                    break;
                  case "complete":
                    const activeId =
                      store.activeConversationId || currentConversationId;
                    updateLastMessage(activeId, {
                      id: event.id,
                      clay_tools_used:
                        event.tools_used.length > 0
                          ? event.tools_used
                          : undefined,
                      processing_time_ms: event.processing_time_ms,
                    });
                    // Clear active tools when response is complete
                    clearActiveTools(activeId);
                    // Clear loading state for both 'new' and actual conversation ID
                    setConversationLoading(activeId, false);
                    if (currentConversationId === "new" && activeId !== "new") {
                      setConversationLoading("new", false);
                    }
                    break;
                  case "error":
                    const errorActiveId =
                      store.activeConversationId || currentConversationId;
                    setConversationError(errorActiveId, event.error);
                    // Clear active tools on error
                    clearActiveTools(errorActiveId);
                    break;
                }
              } catch (e) {
                // Failed to parse SSE event
              }
            }
          }
        }
        setConversationStreaming(currentConversationId, false);
      } catch (err) {
        if (err instanceof DOMException && err.name === "AbortError") {
          // Request was cancelled - don't show error message
        } else {
          setConversationError(
            currentConversationId,
            err instanceof Error ? err.message : "An error occurred"
          );
        }
      } finally {
        const finalActiveId =
          store.activeConversationId || currentConversationId;
        setConversationLoading(finalActiveId, false);
        setConversationStreaming(finalActiveId, false);
        setConversationAbortController(finalActiveId, null);
        // Clear active tools when streaming ends (cleanup)
        clearActiveTools(finalActiveId);
        // Also clear for 'new' if we transitioned
        if (currentConversationId === "new" && finalActiveId !== "new") {
          setConversationLoading("new", false);
          setConversationStreaming("new", false);
          setConversationAbortController("new", null);
          clearActiveTools("new");
        }
      }
    },
    [projectId, currentConversationId]
  );

  const sendMessage = useCallback(
    async (content: string, files?: File[], isFromQueue: boolean = false) => {
      if (!projectId) {
        setConversationError(currentConversationId, "Project ID is required");
        return;
      }

      // Check if we should queue the message instead (only if not already from queue)
      const state = getConversationState(currentConversationId);
      if (!isFromQueue && (state.isStreaming || state.isProcessingQueue)) {
        // Add to queue instead of sending immediately
        addToMessageQueue(currentConversationId, {
          id: `queue-${Date.now()}`,
          content,
          files: files || [],
          timestamp: new Date(),
        });
        return;
      }

      // When sending a new message with forgotten messages, the backend will delete them permanently
      // Clear the forgotten state immediately to reflect this
      if (forgottenAfterMessageId) {
        setConversationForgotten(currentConversationId, null, 0);
      }

      // Upload files first if any
      let uploadedFilePaths: string[] = [];
      let reusedFiles: any[] = [];

      if (files && files.length > 0) {
        try {
          const clientId = localStorage.getItem("activeClientId");
          if (!clientId) {
            setConversationError(
              currentConversationId,
              "No active client found"
            );
            return;
          }

          for (const file of files) {
            const fileWithMeta = file as any;
            if (fileWithMeta.isExisting && fileWithMeta.filePath) {
              uploadedFilePaths.push(fileWithMeta.filePath);
              reusedFiles.push({
                id: fileWithMeta.fileId,
                file_path: fileWithMeta.filePath,
                original_name: fileWithMeta.name,
                description: fileWithMeta.description,
                auto_description: fileWithMeta.autoDescription,
              });
            } else {
              const formData = new FormData();
              formData.append("file", file);

              const response = await fetch(
                `${API_BASE_URL}/upload?client_id=${clientId}&project_id=${projectId}`,
                {
                  method: "POST",
                  credentials: "include",
                  body: formData,
                }
              );

              if (!response.ok) {
                throw new Error(`Failed to upload ${file.name}`);
              }

              const result = await response.json();
              uploadedFilePaths.push(result.file_path);
              addConversationUploadedFile(currentConversationId, result);
            }
          }

          if (reusedFiles.length > 0) {
            const existingFiles = getConversationState(
              currentConversationId
            ).uploadedFiles;
            const existingIds = new Set(existingFiles.map((f) => f.id));
            const newFiles = reusedFiles.filter((f) => !existingIds.has(f.id));
            newFiles.forEach((file) =>
              addConversationUploadedFile(currentConversationId, file)
            );
          }
        } catch (err) {
          setConversationError(
            currentConversationId,
            `File upload failed: ${
              err instanceof Error ? err.message : "Unknown error"
            }`
          );
          setConversationLoading(currentConversationId, false);
          return;
        }
      }

      // Create AbortController for this request
      const abortController = new AbortController();
      setConversationAbortController(currentConversationId, abortController);

      setConversationLoading(currentConversationId, true);
      setConversationError(currentConversationId, null);

      try {
        // Add user message to local state immediately
        let messageContent = content;
        if (uploadedFilePaths.length > 0) {
          messageContent += `\n\nAttached files:\n${uploadedFilePaths
            .map((f) => `- ${f}`)
            .join("\n")}`;
        }

        const userMessage = {
          id: `temp-${Date.now()}`,
          role: "user" as const,
          content: messageContent,
          createdAt: new Date().toISOString(),
        };
        addMessage(currentConversationId, userMessage);

        // Always use streaming endpoint (non-streaming endpoint doesn't exist)
        setConversationStreaming(currentConversationId, true);
        let assistantContent = "";

        const response = await fetch(`${API_BASE_URL}/chat/stream`, {
          method: "POST",
          credentials: "include",
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
            conversation_id: currentConversationId,
          }),
          signal: abortController.signal,
        });

        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }

        const reader = response.body?.getReader();
        const decoder = new TextDecoder();

        if (!reader) {
          throw new Error("No response body");
        }

        let buffer = "";

        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          const chunk = decoder.decode(value, { stream: true });
          buffer += chunk;
          const lines = buffer.split("\n");

          buffer = lines.pop() || "";

          for (const line of lines) {
            const trimmedLine = line.trim();
            if (trimmedLine.startsWith("data:")) {
              const data = trimmedLine.slice(5).trim();
              if (data === "[DONE]") continue;
              if (!data) continue;

              try {
                const event = JSON.parse(data);

                switch (event.type) {
                  case "start":
                    // Update conversation ID when we get the real ID from backend
                    if (
                      event.conversation_id &&
                      event.conversation_id !== "new" &&
                      currentConversationId === "new"
                    ) {
                      console.log(
                        "[useValtioChat] Received real conversation ID:",
                        event.conversation_id
                      );

                      // Don't use pushState, let React Router handle navigation
                      // The navigation will happen via effect in the Chat component

                      // Transfer the state from 'new' to the real conversation ID BEFORE setting active
                      const newState = getConversationState("new");
                      const realState = getConversationState(
                        event.conversation_id
                      );
                      // Deep copy messages to avoid reference issues
                      realState.messages = [...newState.messages];
                      realState.isLoading = newState.isLoading;
                      realState.isStreaming = newState.isStreaming;
                      realState.error = newState.error;
                      realState.uploadedFiles = [...newState.uploadedFiles];

                      // Only set active conversation AFTER state is transferred
                      setActiveConversation(event.conversation_id);
                      
                      // Dispatch a custom event to notify the sidebar to refresh
                      window.dispatchEvent(new CustomEvent('conversation-created', {
                        detail: { conversationId: event.conversation_id, projectId }
                      }));
                    }
                    break;
                  case "progress":
                    try {
                      const streamJson = JSON.parse(event.content);

                      if (
                        streamJson.type === "text" ||
                        streamJson.type === "progress"
                      ) {
                        const textContent =
                          streamJson.text || streamJson.content || "";
                        if (textContent) {
                          assistantContent += textContent;
                          // Use the active conversation ID from the store, fallback to current if not set
                          const activeId =
                            store.activeConversationId || currentConversationId;
                          const currentMessages =
                            getConversationState(activeId).messages;
                          const lastMessage =
                            currentMessages[currentMessages.length - 1];
                          if (lastMessage && lastMessage.role === "assistant") {
                            updateLastMessage(activeId, {
                              content: assistantContent,
                            });
                          } else {
                            addMessage(activeId, {
                              id: `streaming-${Date.now()}`,
                              role: "assistant",
                              content: assistantContent,
                              createdAt: new Date().toISOString(),
                            });
                          }
                        }
                      }
                    } catch (parseError) {
                      // Skip non-JSON messages
                    }
                    break;
                  case "tool_use":
                    if (event.tool) {
                      const activeId =
                        store.activeConversationId || currentConversationId;
                      addActiveTool(activeId, event.tool);
                    }
                    break;
                  case "content":
                    if (event.content) {
                      const activeId =
                        store.activeConversationId || currentConversationId;
                      const currentMessages =
                        getConversationState(activeId).messages;
                      const lastMessage =
                        currentMessages[currentMessages.length - 1];
                      if (lastMessage && lastMessage.role === "assistant") {
                        updateLastMessage(activeId, { content: event.content });
                      } else if (!assistantContent) {
                        addMessage(activeId, {
                          id: `streaming-${Date.now()}`,
                          role: "assistant",
                          content: event.content,
                          createdAt: new Date().toISOString(),
                        });
                      }
                    }
                    break;
                  case "complete":
                    const activeId =
                      store.activeConversationId || currentConversationId;
                    updateLastMessage(activeId, {
                      id: event.id,
                      clay_tools_used:
                        event.tools_used.length > 0
                          ? event.tools_used
                          : undefined,
                      processing_time_ms: event.processing_time_ms,
                    });
                    // Clear active tools when response is complete
                    clearActiveTools(activeId);
                    // Clear loading state for both 'new' and actual conversation ID
                    setConversationLoading(activeId, false);
                    if (currentConversationId === "new" && activeId !== "new") {
                      setConversationLoading("new", false);
                    }
                    break;
                  case "error":
                    const errorActiveId =
                      store.activeConversationId || currentConversationId;
                    setConversationError(errorActiveId, event.error);
                    // Clear active tools on error
                    clearActiveTools(errorActiveId);
                    break;
                }
              } catch (e) {
                // Failed to parse SSE event
              }
            }
          }
        }
        setConversationStreaming(currentConversationId, false);
      } catch (err) {
        if (err instanceof DOMException && err.name === "AbortError") {
          // Request was cancelled - don't show error message
        } else {
          setConversationError(
            currentConversationId,
            err instanceof Error ? err.message : "An error occurred"
          );
        }
      } finally {
        const finalActiveId =
          store.activeConversationId || currentConversationId;
        setConversationLoading(finalActiveId, false);
        setConversationStreaming(finalActiveId, false);
        setConversationAbortController(finalActiveId, null);
        // Clear active tools when streaming ends (cleanup)
        clearActiveTools(finalActiveId);
        // Also clear for 'new' if we transitioned
        if (currentConversationId === "new" && finalActiveId !== "new") {
          setConversationLoading("new", false);
          setConversationStreaming("new", false);
          setConversationAbortController("new", null);
          clearActiveTools("new");
        }
      }
    },
    [
      projectId,
      currentConversationId,
      forgottenAfterMessageId,
      restoreForgottenMessages,
    ]
  );

  // Function to stop/cancel current request
  const stopMessage = useCallback(() => {
    if (currentConversationId) {
      const controller = getConversationAbortController(currentConversationId);
      if (controller) {
        controller.abort();
      }
    }
  }, [currentConversationId]);

  // Function to forget messages after a specific message
  const forgetMessagesFrom = useCallback(
    async (messageId: string) => {
      if (!currentConversationId) return;

      // Stop any streaming if active
      const controller = getConversationAbortController(currentConversationId);
      if (controller) {
        controller.abort();
      }

      try {
        const response = await fetch(
          `${API_BASE_URL}/conversations/${currentConversationId}/forget-after`,
          {
            method: "PUT",
            credentials: "include",
            headers: {
              "Content-Type": "application/json",
            },
            body: JSON.stringify({ message_id: messageId }),
          }
        );

        if (!response.ok) {
          throw new Error("Failed to forget messages");
        }

        const result = await response.json();

        setConversationForgotten(
          currentConversationId,
          messageId,
          result.forgotten_count || 0
        );

        // Filter messages locally to only show those up to and including the forgotten point
        const messageIndex = messages.findIndex((m) => m.id === messageId);
        if (messageIndex !== -1) {
          const filteredMessages = messages
            .slice(0, messageIndex + 1)
            .map((msg) => ({
              ...msg,
              clay_tools_used: msg.clay_tools_used
                ? [...msg.clay_tools_used]
                : undefined,
              file_attachments: msg.file_attachments
                ? [...msg.file_attachments]
                : undefined,
            }));
          updateConversationMessages(currentConversationId, filteredMessages);
        }
      } catch (err) {
        setConversationError(
          currentConversationId,
          err instanceof Error ? err.message : "Failed to forget messages"
        );
      }
    },
    [currentConversationId, messages, currentAbortController]
  );

  // Get queue state early to use in dependencies
  const messageQueue = conversationState?.messageQueue || [];
  const isProcessingQueue = conversationState?.isProcessingQueue || false;

  // Queue management functions
  const editQueuedMessage = useCallback(
    (messageId: string, newContent: string) => {
      updateMessageInQueue(currentConversationId, messageId, {
        content: newContent,
      });
    },
    [currentConversationId]
  );

  const cancelQueuedMessage = useCallback(
    (messageId: string) => {
      removeFromMessageQueue(currentConversationId, messageId);
    },
    [currentConversationId]
  );

  // Store sendMessage in a ref to avoid dependency issues
  const sendMessageRef = useRef(sendMessage);
  useEffect(() => {
    sendMessageRef.current = sendMessage;
  }, [sendMessage]);

  // Auto-process queue after streaming completes
  useEffect(() => {
    const state = getConversationState(currentConversationId);
    if (
      !state.isStreaming &&
      !state.isProcessingQueue &&
      state.messageQueue.length > 0
    ) {
      // Process next message in queue
      const nextMessage = state.messageQueue[0];
      if (nextMessage) {
        // Remove from queue first
        removeFromMessageQueue(currentConversationId, nextMessage.id);

        // Send the queued message with isFromQueue flag
        const processQueuedMessage = async () => {
          await sendMessageRef.current(
            nextMessage.content,
            nextMessage.files,
            true
          );
        };

        processQueuedMessage();
      }
    }
  }, [currentConversationId, isStreaming, messageQueue.length]); // Watch queue length instead

  // Return the appropriate conversation ID and messages
  // When we're on /new, only return a different conversation ID if we've actually sent a message
  // and received a real conversation ID from the backend
  const activeConversationId = snapshot.activeConversationId;

  // Check if we should navigate away from /new
  // This happens when we've sent a message and received a real conversation ID
  const shouldNavigateFromNew =
    conversationId === "new" &&
    activeConversationId &&
    activeConversationId !== "new" &&
    activeConversationId.startsWith("conv-");

  const returnedConversationId =
    conversationId === "new"
      ? shouldNavigateFromNew
        ? activeConversationId
        : "new"
      : currentConversationId;
  // For messages, use the active conversation if we have it, otherwise use current messages
  // If we're transitioning, prefer the new conversation's messages but fall back to current if empty
  const returnedMessages =
    conversationId === "new" &&
    activeConversationId &&
    activeConversationId !== "new"
      ? snapshot.conversations[activeConversationId]?.messages?.length > 0
        ? snapshot.conversations[activeConversationId].messages
        : messages
      : messages;

  // For loading states, use the active conversation's state when we're in a new chat that has transitioned
  const effectiveConversationId =
    conversationId === "new" &&
    activeConversationId &&
    activeConversationId !== "new"
      ? activeConversationId
      : currentConversationId;
  const effectiveIsLoading =
    snapshot.conversations[effectiveConversationId]?.isLoading || false;
  const effectiveIsStreaming =
    snapshot.conversations[effectiveConversationId]?.isStreaming || false;
  const effectiveActiveTools =
    snapshot.conversations[effectiveConversationId]?.activeTools || [];

  return {
    messages: returnedMessages,
    sendMessage,
    resendMessage,
    stopMessage,
    forgetMessagesFrom,
    restoreForgottenMessages,
    isLoading: effectiveIsLoading,
    isLoadingMessages,
    isStreaming: effectiveIsStreaming,
    error,
    canStop: currentConversationId
      ? getConversationAbortController(currentConversationId) !== null
      : false,
    conversationId: returnedConversationId,
    uploadedFiles,
    forgottenAfterMessageId,
    forgottenCount,
    hasForgottenMessages: forgottenAfterMessageId !== null,
    // Queue management
    messageQueue,
    isProcessingQueue,
    editQueuedMessage,
    cancelQueuedMessage,
    // Tool usage indicators
    activeTools: effectiveActiveTools,
    // Enhanced context information
    conversationContext,
    projectContext,
    // Smart context features
    hasDataSources: (projectContext?.data_sources.length || 0) > 0,
    availableTools:
      conversationContext?.available_tools.filter((t: any) => t.applicable) ||
      projectContext?.available_tools.filter((t: any) => t.applicable) ||
      [],
    contextStrategy: conversationContext?.context_strategy,
  };
}
