import { useState, useCallback, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { API_BASE_URL } from "@/lib/url";

// Types matching the backend
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
        `${API_BASE_URL}/conversations/${conversationId}/context`,
        { credentials: "include" }
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
        `${API_BASE_URL}/projects/${projectId}/context`,
        { credentials: "include" }
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
 * Main chat hook with backend integration (with streaming support)
 */
export function useClayChat(projectId: string, conversationId?: string) {
  const [messages, setMessages] = useState<Message[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isLoadingMessages, setIsLoadingMessages] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isStreaming, setIsStreaming] = useState(false);
  const [currentAbortController, setCurrentAbortController] = useState<AbortController | null>(null);
  // Track the current conversation ID internally
  const [currentConversationId, setCurrentConversationId] = useState<string | undefined>(conversationId);
  const [uploadedFiles, setUploadedFiles] = useState<any[]>([]);
  const [forgottenAfterMessageId, setForgottenAfterMessageId] = useState<string | null>(null);
  const [forgottenCount, setForgottenCount] = useState<number>(0);
  const [hasStartedNewConversation, setHasStartedNewConversation] = useState(false);

  // Function to load uploaded files for a conversation
  const loadUploadedFiles = useCallback(async () => {
    if (!projectId || !conversationId || conversationId === 'new') {
      setUploadedFiles([]);
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
        setUploadedFiles(files);
      }
    } catch (err) {
      // Failed to load uploaded files
    }
  }, [projectId, conversationId]);

  // Load messages when conversation changes
  useEffect(() => {
    if (!conversationId) return;
    
    if (conversationId === 'new') {
      setCurrentConversationId('new');
      setMessages([]);
      setError(null);
      setForgottenAfterMessageId(null);
      setForgottenCount(0);
      setUploadedFiles([]);
      setHasStartedNewConversation(false);
    } else {
      setCurrentConversationId(conversationId);
      // Clear previous messages immediately when switching conversations
      setMessages([]);
      setError(null);
      setIsLoadingMessages(true);
      
      // Load messages from database
      fetch(`${API_BASE_URL}/conversations/${conversationId}/messages`, {
        credentials: "include",
      })
        .then(async res => {
          if (!res.ok) {
            // Provide specific error messages based on status code
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
          setMessages(data);
          setIsLoadingMessages(false);
          // Load uploaded files for this conversation
          loadUploadedFiles();
        })
        .catch(err => {
          setError(err.message);
          setIsLoadingMessages(false);
        });
      
      // Check forgotten status (don't show error for this, as it's not critical)
      fetch(`${API_BASE_URL}/conversations/${conversationId}/forget-after`, {
        credentials: "include",
      })
        .then(res => {
          if (res.ok) {
            return res.json();
          }
          // Don't throw error for forgotten status - just skip it
          return null;
        })
        .then(data => {
          if (data && data.has_forgotten) {
            setForgottenAfterMessageId(data.forgotten_after_message_id);
            setForgottenCount(data.forgotten_count || 0);
          }
        })
        .catch(() => {
          // Silently handle forgotten status errors
        });
    }
  }, [conversationId, projectId, loadUploadedFiles]);

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
        setForgottenAfterMessageId(null);
        setForgottenCount(0);
        
        // Reload all messages
        const messagesResponse = await fetch(
          `${API_BASE_URL}/conversations/${currentConversationId}/messages`,
          {
            credentials: "include",
          }
        );
        
        if (messagesResponse.ok) {
          const allMessages = await messagesResponse.json();
          setMessages(allMessages);
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to restore messages");
      }
    },
    [currentConversationId, forgottenAfterMessageId]
  );

  const sendMessage = useCallback(
    async (content: string, useStreaming = true, files?: File[]) => {
      if (!projectId) {
        setError("Project ID is required");
        return;
      }

      // If there are forgotten messages, restore them first before sending new message
      if (forgottenAfterMessageId) {
        await restoreForgottenMessages();
      }

      // Upload files first if any
      let uploadedFiles: string[] = [];
      let reusedFiles: any[] = [];
      
      if (files && files.length > 0) {
        try {
          // Get client ID from localStorage or API
          const clientId = localStorage.getItem('activeClientId');
          if (!clientId) {
            setError("No active client found");
            return;
          }

          for (const file of files) {
            // Check if this is an existing file being reused
            const fileWithMeta = file as any;
            if (fileWithMeta.isExisting && fileWithMeta.filePath) {
              // This is an existing file, just add its path
              uploadedFiles.push(fileWithMeta.filePath);
              reusedFiles.push({
                id: fileWithMeta.fileId,
                file_path: fileWithMeta.filePath,
                original_name: fileWithMeta.name,
                description: fileWithMeta.description,
                auto_description: fileWithMeta.autoDescription,
              });
            } else {
              // This is a new file, upload it
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
              uploadedFiles.push(result.file_path);
              // Store the uploaded file info for display
              setUploadedFiles(prev => [...prev, result]);
            }
          }
          
          // Add reused files to the uploadedFiles display
          if (reusedFiles.length > 0) {
            setUploadedFiles(prev => {
              // Filter out duplicates
              const existingIds = new Set(prev.map(f => f.id));
              const newFiles = reusedFiles.filter(f => !existingIds.has(f.id));
              return [...prev, ...newFiles];
            });
          }
        } catch (err) {
          setError(`File upload failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
          setIsLoading(false);
          return;
        }
      }

      // Create AbortController for this request
      const abortController = new AbortController();
      setCurrentAbortController(abortController);

      setIsLoading(true);
      setError(null);

      try {
        // Add user message to local state immediately
        let messageContent = content;
        if (uploadedFiles.length > 0) {
          messageContent += `\n\nAttached files:\n${uploadedFiles.map(f => `- ${f}`).join('\n')}`;
        }

        const userMessage: Message = {
          id: `temp-${Date.now()}`,
          role: "user",
          content: messageContent,
          createdAt: new Date().toISOString(),
        };
        setMessages((prev) => [...prev, userMessage]);

        // If we're sending a message from /new, mark that we're starting a new conversation
        if (currentConversationId === 'new') {
          setHasStartedNewConversation(true);
        }

        if (useStreaming) {
          // Use streaming endpoint
          setIsStreaming(true);
          let assistantContent = "";
          let toolsUsed: string[] = [];
          let processingTime: number | undefined;

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

          // Check if conversation ID is in response headers
          const responseConversationId = response.headers.get('X-Conversation-ID') || response.headers.get('conversation-id');
          if (responseConversationId && !currentConversationId) {
            setCurrentConversationId(responseConversationId);
          }

          const reader = response.body?.getReader();
          const decoder = new TextDecoder();

          if (!reader) {
            throw new Error("No response body");
          }

          // Don't add placeholder message yet - wait for actual content

          let buffer = '';
          
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;

            const chunk = decoder.decode(value, { stream: true });
            buffer += chunk;
            const lines = buffer.split("\n");
            
            // Keep the last incomplete line in the buffer
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
                      // Message ID received, streaming started
                      // Set conversation ID if this is a new conversation
                      if (event.conversation_id && event.conversation_id !== 'new' && (!currentConversationId || currentConversationId === 'new')) {
                        setCurrentConversationId(event.conversation_id);
                      } else if (!currentConversationId || currentConversationId === 'new') {
                        // Check for alternative property names
                        const altConvId = event.conversationId || event.conversation || event.conv_id;
                        if (altConvId) {
                          setCurrentConversationId(altConvId);
                        }
                      }
                      break;
                    case "progress":
                      // Set conversation ID if this is a new conversation
                      if (event.conversation_id && (!currentConversationId || currentConversationId === 'new')) {
                        setCurrentConversationId(event.conversation_id);
                      }
                      
                      // Parse stream-json format from Claude SDK
                      try {
                        const streamJson = JSON.parse(event.content);
                        
                        // Handle different stream-json message types
                        if (streamJson.type === 'text' || streamJson.type === 'progress') {
                          const textContent = streamJson.text || streamJson.content || '';
                          if (textContent) {
                            assistantContent += textContent;
                            setMessages((prev) => {
                              const newMessages = [...prev];
                              const lastMessage = newMessages[newMessages.length - 1];
                              if (lastMessage && lastMessage.role === "assistant") {
                                // Update existing assistant message
                                lastMessage.content = assistantContent;
                              } else {
                                // Create new assistant message with content
                                newMessages.push({
                                  id: `streaming-${Date.now()}`,
                                  role: "assistant",
                                  content: assistantContent,
                                  createdAt: new Date().toISOString(),
                                });
                              }
                              return newMessages;
                            });
                          }
                        } else if (streamJson.type === 'assistant' && streamJson.message) {
                          // Handle assistant messages with content blocks
                          if (streamJson.message.content && Array.isArray(streamJson.message.content)) {
                            for (const block of streamJson.message.content) {
                              if (block.type === 'text' && block.text) {
                                assistantContent += block.text;
                                setMessages((prev) => {
                                  const newMessages = [...prev];
                                  const lastMessage = newMessages[newMessages.length - 1];
                                  if (lastMessage && lastMessage.role === "assistant") {
                                    lastMessage.content = assistantContent;
                                  } else {
                                    // Create new assistant message with content
                                    newMessages.push({
                                      id: `streaming-${Date.now()}`,
                                      role: "assistant",
                                      content: assistantContent,
                                      createdAt: new Date().toISOString(),
                                    });
                                  }
                                  return newMessages;
                                });
                              }
                            }
                          }
                        } else if (streamJson.role === 'assistant' && streamJson.content) {
                          // Alternative assistant format
                          if (Array.isArray(streamJson.content)) {
                            for (const block of streamJson.content) {
                              if (block.type === 'text' && block.text) {
                                assistantContent += block.text;
                              }
                            }
                          } else if (typeof streamJson.content === 'string') {
                            assistantContent += streamJson.content;
                          }
                          
                          if (assistantContent) {
                            setMessages((prev) => {
                              const newMessages = [...prev];
                              const lastMessage = newMessages[newMessages.length - 1];
                              if (lastMessage && lastMessage.role === "assistant") {
                                lastMessage.content = assistantContent;
                              } else {
                                // Create new assistant message with content
                                newMessages.push({
                                  id: `streaming-${Date.now()}`,
                                  role: "assistant",
                                  content: assistantContent,
                                  createdAt: new Date().toISOString(),
                                });
                              }
                              return newMessages;
                            });
                          }
                        }
                      } catch (parseError) {
                        // Skip non-JSON messages (like system init messages)
                      }
                      break;
                    case "content":
                      // Final content from Result message - only update if we have an assistant message
                      // Set conversation ID if this is a new conversation
                      if (event.conversation_id && (!currentConversationId || currentConversationId === 'new')) {
                        setCurrentConversationId(event.conversation_id);
                      }
                      
                      if (event.content) {
                        setMessages((prev) => {
                          const newMessages = [...prev];
                          const lastMessage = newMessages[newMessages.length - 1];
                          if (lastMessage && lastMessage.role === "assistant") {
                            // Update with final content
                            lastMessage.content = event.content;
                          } else if (!assistantContent) {
                            // Only create if we haven't received progressive content
                            newMessages.push({
                              id: `streaming-${Date.now()}`,
                              role: "assistant",
                              content: event.content,
                              createdAt: new Date().toISOString(),
                            });
                          }
                          return newMessages;
                        });
                      }
                      break;
                    case "tool_use":
                      toolsUsed.push(event.tool);
                      break;
                    case "complete":
                      processingTime = event.processing_time_ms;
                      
                      // Update final message with complete data
                      setMessages((prev) => {
                        const newMessages = [...prev];
                        const lastMessage = newMessages[newMessages.length - 1];
                        if (lastMessage && lastMessage.role === "assistant") {
                          lastMessage.id = event.id;
                          lastMessage.clay_tools_used = event.tools_used.length > 0 ? event.tools_used : undefined;
                          lastMessage.processing_time_ms = processingTime;
                        }
                        return newMessages;
                      });
                      
                      // Set conversation ID if this is a new conversation
                      // Check if we got a real conversation ID (not 'new')
                      if (event.conversation_id && event.conversation_id !== 'new' && (!currentConversationId || currentConversationId === 'new')) {
                        setCurrentConversationId(event.conversation_id);
                      } else if (!currentConversationId || currentConversationId === 'new') {
                        // Check for alternative property names
                        const altConvId = event.conversationId || event.conversation || event.conv_id;
                        if (altConvId && altConvId !== 'new') {
                          setCurrentConversationId(altConvId);
                        }
                      }
                      break;
                    case "error":
                      setError(event.error);
                      break;
                  }
                } catch (e) {
                  // Failed to parse SSE event
                }
              }
            }
          }
          setIsStreaming(false);
        } else {
          // Use non-streaming endpoint (backward compatibility)
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

          // Set conversation ID if this is a new conversation
          if (assistantResponse.conversation_id && !currentConversationId) {
            setCurrentConversationId(assistantResponse.conversation_id);
          } else if (!currentConversationId) {
            // Check for alternative property names
            const altConvId = assistantResponse.conversationId || assistantResponse.conversation || assistantResponse.conv_id;
            if (altConvId) {
              setCurrentConversationId(altConvId);
            }
          }

          // Add assistant response to messages
          const assistantMessage: Message = {
            id: assistantResponse.id,
            role: "assistant",
            content: assistantResponse.content,
            createdAt: assistantResponse.createdAt,
            clay_tools_used: assistantResponse.clay_tools_used,
            processing_time_ms: assistantResponse.processing_time_ms,
          };
          setMessages((prev) => [...prev, assistantMessage]);
        }
      } catch (err) {
        if (err instanceof DOMException && err.name === 'AbortError') {
          // Request was cancelled - don't show error message
        } else {
          setError(err instanceof Error ? err.message : "An error occurred");
        }
      } finally {
        setIsLoading(false);
        setIsStreaming(false);
        setCurrentAbortController(null);
      }
    },
    [projectId, currentConversationId, forgottenAfterMessageId, restoreForgottenMessages]
  );


  // Function to stop/cancel current request
  const stopMessage = useCallback(() => {
    if (currentAbortController) {
      currentAbortController.abort();
    }
  }, [currentAbortController]);

  // Function to send a one-shot (non-streaming) message
  const sendOneShot = useCallback(
    async (content: string) => {
      if (!projectId) {
        setError("Project ID is required");
        return null;
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
          clay_tools_used: assistantResponse.clay_tools_used,
          processing_time_ms: assistantResponse.processing_time_ms,
        };
        setMessages((prev) => [...prev, assistantMessage]);
        
        return assistantMessage;
      } catch (err) {
        setError(err instanceof Error ? err.message : "An error occurred");
        return null;
      } finally {
        setIsLoading(false);
      }
    },
    [projectId, currentConversationId]
  );

  // Function to forget messages after a specific message
  const forgetMessagesFrom = useCallback(
    async (messageId: string) => {
      if (!currentConversationId) return;

      // Stop any streaming if active
      if (currentAbortController) {
        currentAbortController.abort();
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
        
        // Update local state
        setForgottenAfterMessageId(messageId);
        setForgottenCount(result.forgotten_count || 0);
        
        // Filter messages locally to only show those up to and including the forgotten point
        const messageIndex = messages.findIndex(m => m.id === messageId);
        if (messageIndex !== -1) {
          setMessages(messages.slice(0, messageIndex + 1));
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to forget messages");
      }
    },
    [currentConversationId, messages, currentAbortController]
  );

  // Debug the conversationId being returned
  // When on /new, only return the real conversation ID if we've actually started a new conversation by sending a message
  const returnedConversationId = conversationId === 'new' 
    ? (hasStartedNewConversation && currentConversationId && currentConversationId !== 'new' && currentConversationId.startsWith('conv-')
       ? currentConversationId
       : 'new')
    : currentConversationId;
  // When on /new, always return empty messages to ensure clean slate
  const returnedMessages = conversationId === 'new' ? [] : messages;

  return {
    messages: returnedMessages,
    sendMessage,
    sendOneShot,
    stopMessage,
    forgetMessagesFrom,
    restoreForgottenMessages,
    isLoading,
    isLoadingMessages,
    isStreaming,
    error,
    canStop: currentAbortController !== null,
    // Return the prop directly when it's 'new' to avoid stale state issues
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
      conversationContext?.available_tools.filter(t => t.applicable) ||
      projectContext?.available_tools.filter(t => t.applicable) ||
      [],
    contextStrategy: conversationContext?.context_strategy,
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
