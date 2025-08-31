import { useState, useEffect, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { logger } from "@/lib/logger";
import { Messages, ChatSkeleton } from "../display";
import { MultimodalInput } from "../input/multimodal-input";
import { ContextIndicator } from "../display";
import { useValtioChat } from "@/hooks/use-valtio-chat";
import { useInputState } from "@/hooks/use-input-state";
import { useViewportHeight } from "@/hooks/use-viewport-height";
import { updateConversationMessages } from "@/store/chat-store";
import { api } from "@/lib/api";
import {
  AlertTriangle,
  FileText,
  PanelLeftOpen,
  PanelLeftClose,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import type { Message } from "@/types/chat";
import { cn } from "@/lib/utils";

interface ChatProps {
  projectId?: string;
  conversationId?: string;
  onToggleSidebar?: () => void;
  isSidebarCollapsed?: boolean;
}

export function Chat({
  projectId,
  conversationId: propConversationId,
  onToggleSidebar,
  isSidebarCollapsed,
}: ChatProps) {
  const [isDraggingOver, setIsDraggingOver] = useState(false);
  const [shouldFocusInput, setShouldFocusInput] = useState(false);
  const dragCounter = useRef(0);
  const navigate = useNavigate();
  const [previousId, setPreviousId] = useState("");
  const { viewportHeight } = useViewportHeight();

  useEffect(() => {
    // Reset when conversation changes
    if (propConversationId !== previousConversationId.current) {
      setPreviousId(propConversationId || "");
    }
  }, [propConversationId]);

  // Use the new Valtio-based chat hook
  const {
    messages,
    sendMessage,
    stopMessage,
    forgetMessagesFrom,
    restoreForgottenMessages,
    isLoading,
    isLoadingMessages,
    isStreaming,
    error,
    canStop,
    conversationId: hookConversationId,
    uploadedFiles,
    hasForgottenMessages,
    forgottenCount,
    messageQueue,
    isProcessingQueue,
    editQueuedMessage,
    cancelQueuedMessage,
    activeTools,
    contextUsage,
  } = useValtioChat(projectId || "", propConversationId);

  // Track current conversation ID similar to messages.tsx
  const currentConversationId =
    hookConversationId || propConversationId || "new";
  const previousConversationId = useRef(currentConversationId);

  // Use the input state hook to persist input across conversation switches
  const {
    draftMessage: input,
    setDraftMessage: setInput,
    attachments: pendingFiles,
    setAttachments: setPendingFiles,
  } = useInputState(propConversationId || "new");

  // Handle navigation when a new conversation is created
  // Navigate when we receive a real conversation ID from the backend
  useEffect(() => {
    if (
      propConversationId === "new" &&
      hookConversationId &&
      hookConversationId !== "new"
    ) {
      logger.info(
        "Chat: REDIRECTING to hookConversationId:",
        hookConversationId
      );
      // Navigate to the real conversation URL
      navigate(`/chat/${projectId}/${hookConversationId}`, { replace: true });
    }
  }, [propConversationId, hookConversationId, projectId, navigate]);

  // Focus input when conversation changes (including navigating to /new)
  useEffect(() => {
    if (projectId && propConversationId) {
      // Small delay to ensure components are rendered
      const timer = setTimeout(() => {
        setShouldFocusInput(true);
        // Reset after focusing
        setTimeout(() => setShouldFocusInput(false), 100);
      }, 100);

      return () => clearTimeout(timer);
    }
  }, [projectId, propConversationId]);

  const handleSubmit = async (e: React.FormEvent, files?: File[]) => {
    e.preventDefault();
    if (!input.trim() || !projectId) return;

    const messageContent = input.trim();
    setInput("");
    // Include any pending files from drag-drop along with form files
    const allFiles = [...(files || []), ...pendingFiles];
    setPendingFiles([]);
    await sendMessage(
      messageContent,
      allFiles.length > 0 ? allFiles : undefined
    );
  };

  const handleResendMessage = async (message: any) => {
    if (!projectId) return;

    // Extract the original content, removing any file attachment mentions
    let content = message.content;

    // Remove the "Attached files:" section if present
    const attachedFilesIndex = content.indexOf("\n\nAttached files:");
    if (attachedFilesIndex > -1) {
      content = content.substring(0, attachedFilesIndex);
    }

    // Remove the current message from the messages array
    const messageIndex = messages.findIndex((m) => m.id === message.id);
    if (messageIndex !== -1) {
      // Remove the current message and any subsequent assistant response
      const messagesToKeep = messages.slice(0, messageIndex);

      // Update the conversation state with the filtered messages
      const currentConversationId =
        hookConversationId || propConversationId || "new";
      updateConversationMessages(
        currentConversationId,
        messagesToKeep as Message[]
      );
    }

    // Send the message as a new message (not resend)
    await sendMessage(content.trim());
  };

  const handleNewChatFromHere = async (messageId: string) => {
    if (!projectId) return;

    // Find the index of the message to clone up to (inclusive)
    const messageIndex = messages.findIndex((m) => m.id === messageId);
    if (messageIndex === -1) return;

    // Get messages to clone (from start to the selected message, inclusive)
    const messagesToClone = messages.slice(0, messageIndex + 1);

    // Call API to create new chat with cloned messages
    try {
      const response = await api.fetchStream(
        `/conversations/new-from-message`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            project_id: projectId,
            source_conversation_id: hookConversationId || propConversationId,
            message_id: messageId,
            messages: messagesToClone,
          }),
        }
      );

      if (!response.ok) {
        throw new Error("Failed to create new chat");
      }

      const data = await response.json();

      // Navigate to the new conversation
      if (data.conversation_id) {
        navigate(`/chat/${projectId}/${data.conversation_id}`);
      }
    } catch (error) {
      logger.error("Chat: Failed to create new chat from message:", error);
    }
  };

  const handleAskUserSubmit = (response: string | string[]) => {
    // Format the response for sending as a message
    let responseText: string;

    if (Array.isArray(response)) {
      // For checkbox selections, send as comma-separated list
      responseText = response.join(", ");
    } else {
      // For buttons and input, send as is
      responseText = response;
    }

    // Send the response as a new user message
    if (responseText) {
      sendMessage(responseText, []);
    }
  };

  // Container-level drag handlers for larger drop area
  const handleContainerDragEnter = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounter.current++;
    if (e.dataTransfer.items && e.dataTransfer.items.length > 0) {
      setIsDraggingOver(true);
    }
  };

  const handleContainerDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounter.current--;
    if (dragCounter.current === 0) {
      setIsDraggingOver(false);
    }
  };

  const handleContainerDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  };

  const handleContainerDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDraggingOver(false);
    dragCounter.current = 0;

    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      const droppedFiles = Array.from(e.dataTransfer.files);

      setPendingFiles([...pendingFiles, ...droppedFiles]);
      e.dataTransfer.clearData();
    }
  };

  return (
    <>
      <div
        className="group w-full overflow-auto pl-0 relative flex flex-col"
        style={{ height: viewportHeight ? `${viewportHeight}px` : "100vh" }}
        onDragEnter={handleContainerDragEnter}
        onDragLeave={handleContainerDragLeave}
        onDragOver={handleContainerDragOver}
        onDrop={handleContainerDrop}
      >
        {/* Floating sidebar toggle button - hidden on mobile since mobile has its own toggle */}
        {onToggleSidebar && (
          <Button
            variant="outline"
            size="sm"
            onClick={onToggleSidebar}
            className="fixed top-4 left-4 z-30 h-10 w-10 p-0 rounded-full shadow-lg bg-background border hidden md:flex"
          >
            {isSidebarCollapsed ? (
              <PanelLeftOpen className="h-5 w-5" />
            ) : (
              <PanelLeftClose className="h-5 w-5" />
            )}
          </Button>
        )}
        {hasForgottenMessages && (
          <div className="absolute left-0 right-0 bg-white pt-5 top-0 w-full z-10 px-4">
            <div className="flex max-w-[44rem] mx-auto mb-4 rounded-lg border border-amber-200 bg-amber-50 p-3 items-center justify-between">
              <p className="text-sm text-amber-800">
                {forgottenCount} message{forgottenCount !== 1 ? "s" : ""}{" "}
                forgotten.
              </p>
              <button
                onClick={restoreForgottenMessages}
                className="text-sm px-3 py-1 bg-amber-600 text-white rounded hover:bg-amber-700 transition-colors"
              >
                Restore All
              </button>
            </div>
          </div>
        )}
        {/* Full-screen drop zone overlay */}
        {isDraggingOver && (
          <div className="fixed inset-0 z-40 bg-primary/10 backdrop-blur-sm flex items-center justify-center pointer-events-none">
            <div className="bg-white rounded-lg shadow-xl p-8 text-center animate-in fade-in zoom-in duration-200">
              <FileText className="h-16 w-16 mx-auto mb-4 text-primary animate-bounce" />
              <h3 className="text-xl font-semibold mb-2">
                Drop your files here
              </h3>
              <p className="text-muted-foreground">
                Files will be uploaded when you send your message
              </p>
            </div>
          </div>
        )}

        <div className="flex-1 overflow-hidden flex flex-col">
          <div id="portal-body"></div>
          <div className="flex-1 overflow-y-auto">
            {/* Error display */}
            {error && (
              <div className="mb-4 bg-red-600 p-4 text-white">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-sm">
                    <AlertTriangle />
                    <div>{error}</div>
                  </div>
                  {(error.includes("doesn't exist") ||
                    error.includes("deleted") ||
                    error.includes("permission")) &&
                    projectId && (
                      <button
                        onClick={() => {
                          navigate(`/chat/${projectId}/new`, { replace: true });
                        }}
                        className="text-sm px-3 py-1 bg-red-600 text-white rounded hover:bg-red-700 transition-colors"
                      >
                        Start New Chat
                      </button>
                    )}
                </div>
              </div>
            )}

            {/* Show message when no project is selected */}
            {!projectId ? (
              <div className="text-center py-12">
                <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">
                  Select or Create a Project
                </h3>
                <p className="text-sm text-gray-500 dark:text-gray-400">
                  Choose a project from the selector above to start chatting
                  with Claude.
                </p>
              </div>
            ) : isLoadingMessages ? (
              <ChatSkeleton />
            ) : (
              <Messages
                messages={messages.map((msg) => ({
                  ...msg,
                  createdAt: msg.createdAt
                    ? new Date(msg.createdAt)
                    : new Date(),
                  tool_usages: msg.tool_usages
                    ? [...msg.tool_usages]
                    : undefined,
                  file_attachments: msg.file_attachments
                    ? [...msg.file_attachments]
                    : undefined,
                }))}
                isLoading={isLoading}
                onForgetFrom={forgetMessagesFrom}
                conversationId={hookConversationId}
                messageQueue={messageQueue.map((q) => ({
                  ...q,
                  files: [...q.files],
                }))}
                isProcessingQueue={isProcessingQueue}
                onEditQueued={editQueuedMessage}
                onCancelQueued={cancelQueuedMessage}
                isStreaming={isStreaming}
                canStop={canStop}
                onStop={stopMessage}
                activeTools={[...activeTools]}
                onResendMessage={handleResendMessage}
                onNewChatFromHere={handleNewChatFromHere}
                onAskUserSubmit={handleAskUserSubmit}
              />
            )}
          </div>
        </div>
        <div
          className="bg-background border-t"
          style={{
            bottom: "0",
            transition: "bottom 0.3s ease-in-out",
          }}
        >
          <div className="mx-auto max-w-2xl sm:px-4">
            {/* Context usage indicator */}
            {contextUsage && propConversationId !== "new" && (
              <div className="flex justify-end mb-2 px-2">
                <ContextIndicator contextUsage={contextUsage} />
              </div>
            )}
            <MultimodalInput
              input={input}
              setInput={setInput}
              handleSubmit={handleSubmit}
              isLoading={isLoading}
              isStreaming={isStreaming}
              projectId={projectId}
              uploadedFiles={uploadedFiles ? [...uploadedFiles] : []}
              externalFiles={pendingFiles ? [...pendingFiles] : []}
              onExternalFilesChange={setPendingFiles}
              shouldFocus={shouldFocusInput}
              className={cn(
                previousId === "new" && propConversationId !== "new"
                  ? "opacity-0"
                  : "",
                "lg:ml-[-10px] lg:mr-[40px]"
              )}
            />
          </div>
        </div>
      </div>
    </>
  );
}
