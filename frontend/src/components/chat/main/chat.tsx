import { useState, useEffect, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { Messages } from "../display";
import { MultimodalInput } from "../input/multimodal-input";
import { useValtioChat } from "@/hooks/use-valtio-chat";
import { useInputState } from "@/hooks/use-input-state";
import { updateConversationMessages } from "@/store";
import { FileText } from "lucide-react";
import type { Message } from "@/types/chat";

interface ChatProps {
  projectId?: string;
  conversationId?: string;
}

export function Chat({
  projectId,
  conversationId: propConversationId,
}: ChatProps) {
  const [isDraggingOver, setIsDraggingOver] = useState(false);
  const [shouldFocusInput, setShouldFocusInput] = useState(false);
  const dragCounter = useRef(0);
  const navigate = useNavigate();
  const previousPropConversationId = useRef(propConversationId);

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
  } = useValtioChat(projectId || "", propConversationId);

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
    if (propConversationId === 'new' && hookConversationId && hookConversationId !== 'new') {
      // Navigate to the real conversation URL
      navigate(`/chat/${projectId}/${hookConversationId}`, { replace: true });
    }
    previousPropConversationId.current = propConversationId;
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
        className="group w-full overflow-auto pl-0 relative h-full"
        onDragEnter={handleContainerDragEnter}
        onDragLeave={handleContainerDragLeave}
        onDragOver={handleContainerDragOver}
        onDrop={handleContainerDrop}
      >
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

        <div className="h-full">
          <div className="h-full flex flex-col">
            {/* Error display */}
            {error && (
              <div className="mb-4 rounded-lg border border-red-200 bg-red-50 p-4 text-red-800">
                <div className="flex items-center justify-between">
                  <p className="text-sm">{error}</p>
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
              <div className="text-center py-12 mt-20">
                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto mb-4"></div>
                <p className="text-sm text-muted-foreground">
                  Loading conversation...
                </p>
              </div>
            ) : (
              <div className="flex-1 overflow-hidden">
                <Messages
                  messages={messages.map((msg) => ({
                    ...msg,
                    createdAt: msg.createdAt
                      ? new Date(msg.createdAt)
                      : new Date(),
                    clay_tools_used: msg.clay_tools_used
                      ? [...msg.clay_tools_used]
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
                />
              </div>
            )}
          </div>
        </div>
        <div className="w-full absolute bottom-0 right-0">
          <div className="mx-auto max-w-2xl px-2 sm:px-4">
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
              className={""}
            />
          </div>
        </div>
      </div>
    </>
  );
}
