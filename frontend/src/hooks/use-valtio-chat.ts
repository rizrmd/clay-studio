import { useCallback, useEffect } from "react";
import { useSnapshot } from 'valtio';
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
  setConversationStarted,
  setConversationContext,
  cacheProjectContext
} from '../store';

// Re-export types from the original hook
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
  RecentActivity
} from './use-clay-chat';

// Additional types for our Valtio implementation
export interface QueuedMessage {
  id: string;
  content: string;
  timestamp: Date;
  status: 'pending' | 'processing' | 'completed' | 'failed';
  isEditable?: boolean;
  files?: File[];
}

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
    activeTools: context?.available_tools.filter((t: any) => t.applicable) || [],
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
      projectContext?.data_sources.reduce((acc: Record<string, number>, ds: any) => {
        acc[ds.source_type] = (acc[ds.source_type] || 0) + 1;
        return acc;
      }, {} as Record<string, number>) || {},
    toolsByCategory:
      projectContext?.available_tools.reduce((acc: Record<string, any[]>, tool: any) => {
        if (!acc[tool.category]) acc[tool.category] = [];
        acc[tool.category].push(tool);
        return acc;
      }, {} as Record<string, any[]>) || {},
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
  const currentConversationId = conversationId || 'new';
  
  // Get conversation state from store
  const conversationState = snapshot.conversations[currentConversationId];
  const messages = conversationState?.messages || [];
  const isLoading = conversationState?.isLoading || false;
  const isLoadingMessages = conversationState?.isLoadingMessages || false;
  const error = conversationState?.error || null;
  const isStreaming = conversationState?.isStreaming || false;
  const uploadedFiles = conversationState?.uploadedFiles || [];
  const forgottenAfterMessageId = conversationState?.forgottenAfterMessageId || null;
  const forgottenCount = conversationState?.forgottenCount || 0;
  const hasStartedNewConversation = conversationState?.hasStartedNewConversation || false;
  const currentAbortController = conversationState?.currentAbortController || null;

  // Set active conversation
  useEffect(() => {
    setActiveConversation(currentConversationId);
  }, [currentConversationId]);

  // Function to load uploaded files for a conversation
  const loadUploadedFiles = useCallback(async () => {
    if (!projectId || !conversationId || conversationId === 'new') {
      setConversationUploadedFiles(currentConversationId, []);
      return;
    }

    try {
      const clientId = localStorage.getItem('activeClientId');
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
    
    if (conversationId === 'new') {
      // Initialize empty state for new conversation
      const state = getConversationState('new');
      state.messages = [];
      state.error = null;
      state.forgottenAfterMessageId = null;
      state.forgottenCount = 0;
      state.uploadedFiles = [];
      state.hasStartedNewConversation = false;
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
        .then(async res => {
          if (!res.ok) {
            let errorMessage = "Failed to load conversation";
            
            switch (res.status) {
              case 404:
                errorMessage = "This conversation doesn't exist or has been deleted";
                break;
              case 403:
                errorMessage = "You don't have permission to access this conversation";
                break;
              case 500:
                errorMessage = "Server error while loading the conversation. Please try again";
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
        .then(data => {
          updateConversationMessages(conversationId, [...data]);
          setConversationError(conversationId, null);
          const convState = getConversationState(conversationId);
          convState.isLoadingMessages = false;
          loadUploadedFiles();
        })
        .catch(err => {
          setConversationError(conversationId, err.message);
          const convState = getConversationState(conversationId);
          convState.isLoadingMessages = false;
        });
      
      // Check forgotten status
      fetch(`${API_BASE_URL}/conversations/${conversationId}/forget-after`, {
        credentials: "include",
      })
        .then(res => {
          if (res.ok) {
            return res.json();
          }
          return null;
        })
        .then(data => {
          if (data && data.has_forgotten) {
            setConversationForgotten(conversationId, data.forgotten_after_message_id, data.forgotten_count || 0);
          }
        })
        .catch(() => {
          // Silently handle forgotten status errors
        });
    }
  }, [conversationId, projectId, loadUploadedFiles, snapshot.conversations]);

  // Load context for the conversation or project
  const { context: conversationContext } = useConversationContext(
    conversationId && conversationId !== 'new' ? conversationId : null
  );
  const { projectContext } = useProjectContext(projectId);

  // Function to restore forgotten messages
  const restoreForgottenMessages = useCallback(
    async () => {
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
        setConversationError(currentConversationId, err instanceof Error ? err.message : "Failed to restore messages");
      }
    },
    [currentConversationId, forgottenAfterMessageId]
  );

  const sendMessage = useCallback(
    async (content: string, useStreaming = true, files?: File[]) => {
      if (!projectId) {
        setConversationError(currentConversationId, "Project ID is required");
        return;
      }

      // If there are forgotten messages, restore them first
      if (forgottenAfterMessageId) {
        await restoreForgottenMessages();
      }

      // Upload files first if any
      let uploadedFilePaths: string[] = [];
      let reusedFiles: any[] = [];
      
      if (files && files.length > 0) {
        try {
          const clientId = localStorage.getItem('activeClientId');
          if (!clientId) {
            setConversationError(currentConversationId, "No active client found");
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
              formData.append('file', file);

              const response = await fetch(`${API_BASE_URL}/upload?client_id=${clientId}&project_id=${projectId}`, {
                method: 'POST',
                credentials: 'include',
                body: formData,
              });

              if (!response.ok) {
                throw new Error(`Failed to upload ${file.name}`);
              }

              const result = await response.json();
              uploadedFilePaths.push(result.file_path);
              addConversationUploadedFile(currentConversationId, result);
            }
          }
          
          if (reusedFiles.length > 0) {
            const existingFiles = getConversationState(currentConversationId).uploadedFiles;
            const existingIds = new Set(existingFiles.map(f => f.id));
            const newFiles = reusedFiles.filter(f => !existingIds.has(f.id));
            newFiles.forEach(file => addConversationUploadedFile(currentConversationId, file));
          }
        } catch (err) {
          setConversationError(currentConversationId, `File upload failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
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
          messageContent += `\n\nAttached files:\n${uploadedFilePaths.map(f => `- ${f}`).join('\n')}`;
        }

        const userMessage = {
          id: `temp-${Date.now()}`,
          role: "user" as const,
          content: messageContent,
          createdAt: new Date().toISOString(),
        };
        addMessage(currentConversationId, userMessage);

        // If we're sending a message from /new, mark that we're starting a new conversation
        if (currentConversationId === 'new') {
          setConversationStarted(currentConversationId, true);
        }

        if (useStreaming) {
          // Use streaming endpoint
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

          let buffer = '';
          
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;

            const chunk = decoder.decode(value, { stream: true });
            buffer += chunk;
            const lines = buffer.split("\n");
            
            buffer = lines.pop() || '';

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
                      if (event.conversation_id && event.conversation_id !== 'new' && (!currentConversationId || currentConversationId === 'new')) {
                        // Handle conversation ID update if needed
                      }
                      break;
                    case "progress":
                      try {
                        const streamJson = JSON.parse(event.content);
                        
                        if (streamJson.type === 'text' || streamJson.type === 'progress') {
                          const textContent = streamJson.text || streamJson.content || '';
                          if (textContent) {
                            assistantContent += textContent;
                            const currentMessages = getConversationState(currentConversationId).messages;
                            const lastMessage = currentMessages[currentMessages.length - 1];
                            if (lastMessage && lastMessage.role === "assistant") {
                              updateLastMessage(currentConversationId, { content: assistantContent });
                            } else {
                              addMessage(currentConversationId, {
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
                    case "content":
                      if (event.content) {
                        const currentMessages = getConversationState(currentConversationId).messages;
                        const lastMessage = currentMessages[currentMessages.length - 1];
                        if (lastMessage && lastMessage.role === "assistant") {
                          updateLastMessage(currentConversationId, { content: event.content });
                        } else if (!assistantContent) {
                          addMessage(currentConversationId, {
                            id: `streaming-${Date.now()}`,
                            role: "assistant",
                            content: event.content,
                            createdAt: new Date().toISOString(),
                          });
                        }
                      }
                      break;
                    case "complete":
                      updateLastMessage(currentConversationId, {
                        id: event.id,
                        clay_tools_used: event.tools_used.length > 0 ? event.tools_used : undefined,
                        processing_time_ms: event.processing_time_ms,
                      });
                      break;
                    case "error":
                      setConversationError(currentConversationId, event.error);
                      break;
                  }
                } catch (e) {
                  // Failed to parse SSE event
                }
              }
            }
          }
          setConversationStreaming(currentConversationId, false);
        } else {
          // Non-streaming fallback (same logic as original)
          const response = await fetch(`${API_BASE_URL}/chat/one-shot`, {
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

          const assistantResponse = await response.json();

          const assistantMessage = {
            id: assistantResponse.id,
            role: "assistant" as const,
            content: assistantResponse.content,
            createdAt: assistantResponse.createdAt,
            clay_tools_used: assistantResponse.clay_tools_used,
            processing_time_ms: assistantResponse.processing_time_ms,
          };
          addMessage(currentConversationId, assistantMessage);
        }
      } catch (err) {
        if (err instanceof DOMException && err.name === 'AbortError') {
          // Request was cancelled - don't show error message
        } else {
          setConversationError(currentConversationId, err instanceof Error ? err.message : "An error occurred");
        }
      } finally {
        setConversationLoading(currentConversationId, false);
        setConversationStreaming(currentConversationId, false);
        setConversationAbortController(currentConversationId, null);
      }
    },
    [projectId, currentConversationId, forgottenAfterMessageId, restoreForgottenMessages]
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
        
        setConversationForgotten(currentConversationId, messageId, result.forgotten_count || 0);
        
        // Filter messages locally to only show those up to and including the forgotten point
        const messageIndex = messages.findIndex(m => m.id === messageId);
        if (messageIndex !== -1) {
          updateConversationMessages(currentConversationId, [...messages.slice(0, messageIndex + 1)]);
        }
      } catch (err) {
        setConversationError(currentConversationId, err instanceof Error ? err.message : "Failed to forget messages");
      }
    },
    [currentConversationId, messages, currentAbortController]
  );

  // Return the appropriate conversation ID and messages
  const returnedConversationId = conversationId === 'new' 
    ? (hasStartedNewConversation && currentConversationId && currentConversationId !== 'new' && currentConversationId.startsWith('conv-')
       ? currentConversationId
       : 'new')
    : currentConversationId;
  const returnedMessages = conversationId === 'new' ? [] : messages;

  return {
    messages: returnedMessages,
    sendMessage,
    stopMessage,
    forgetMessagesFrom,
    restoreForgottenMessages,
    isLoading,
    isLoadingMessages,
    isStreaming,
    error,
    canStop: currentConversationId ? getConversationAbortController(currentConversationId) !== null : false,
    conversationId: returnedConversationId,
    uploadedFiles,
    forgottenAfterMessageId,
    forgottenCount,
    hasForgottenMessages: forgottenAfterMessageId !== null,
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