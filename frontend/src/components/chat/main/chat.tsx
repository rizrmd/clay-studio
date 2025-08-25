import { useState, useEffect, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { Messages } from "../display";
import { MultimodalInput } from "../input/multimodal-input";
import { useClayChat } from "@/hooks/use-clay-chat";
import { FileText } from "lucide-react";

interface ChatProps {
  projectId?: string;
  conversationId?: string;
}

export function Chat({
  projectId,
  conversationId: propConversationId,
}: ChatProps) {
  const [input, setInput] = useState("");
  const [isDraggingOver, setIsDraggingOver] = useState(false);
  const [pendingFiles, setPendingFiles] = useState<File[]>([]);
  const [shouldFocusInput, setShouldFocusInput] = useState(false);
  const dragCounter = useRef(0);
  const navigate = useNavigate();
  const previousPropConversationId = useRef(propConversationId);

  // Use conversation ID from props
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
  } = useClayChat(projectId || "", propConversationId);

  // Handle navigation when a new conversation is created
  useEffect(() => {
    // Update the previous value for next render
    previousPropConversationId.current = propConversationId;

    // Only navigate away from /new when we receive a real conversation ID from the backend
    // This happens after sending the first message
    if (
      propConversationId === "new" &&
      hookConversationId &&
      hookConversationId !== "new" &&
      hookConversationId.startsWith("conv-")
    ) {
      const newUrl = `/chat/${projectId}/${hookConversationId}`;

      // Navigate with replace to avoid adding to history
      navigate(newUrl, { replace: true });
    }
  }, [hookConversationId, propConversationId, projectId, navigate]);

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
    if (!input.trim() || isLoading || !projectId) return;

    const messageContent = input.trim();
    setInput("");
    // Include any pending files from drag-drop along with form files
    const allFiles = [...(files || []), ...pendingFiles];
    setPendingFiles([]);
    await sendMessage(
      messageContent,
      true,
      allFiles.length > 0 ? allFiles : undefined
    );
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

      setPendingFiles((prev) => [...prev, ...droppedFiles]);
      e.dataTransfer.clearData();
    }
  };

  const stop = () => {
    stopMessage();
  };

  return (
    <>
      {/* Forgotten messages indicator */}
      {hasForgottenMessages && (
        <div className="fixed top-5 w-[800px] z-10 mb-4 rounded-lg border border-amber-200 bg-amber-50 p-3 flex items-center justify-between">
          <p className="text-sm text-amber-800">
            {forgottenCount} message{forgottenCount !== 1 ? "s" : ""} hidden
          </p>
          <button
            onClick={restoreForgottenMessages}
            className="text-sm px-3 py-1 bg-amber-600 text-white rounded hover:bg-amber-700 transition-colors"
          >
            Restore All
          </button>
        </div>
      )}
      <div
        className="group w-full overflow-auto pl-0 peer-[[data-state=open]]:lg:pl-[250px] peer-[[data-state=open]]:xl:pl-[300px] relative"
        onDragEnter={handleContainerDragEnter}
        onDragLeave={handleContainerDragLeave}
        onDragOver={handleContainerDragOver}
        onDrop={handleContainerDrop}
      >
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

        <div className="pb-[200px] pt-4 md:pt-10">
          <div className="mx-auto max-w-2xl px-4">
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
              <div className="text-center py-12">
                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto mb-4"></div>
                <p className="text-sm text-muted-foreground">
                  Loading conversation...
                </p>
              </div>
            ) : (
              <Messages
                messages={messages.map((msg) => ({
                  ...msg,
                  createdAt: msg.createdAt
                    ? new Date(msg.createdAt)
                    : new Date(),
                }))}
                isLoading={isLoading}
                onForgetFrom={forgetMessagesFrom}
              />
            )}
          </div>
        </div>
        <div className="fixed inset-x-0 bottom-0 w-full bg-gradient-to-b from-muted/30 from-0% to-muted/30 to-50% duration-300 ease-in-out animate-in dark:from-background/10 dark:from-10% dark:to-background/80 peer-[[data-state=open]]:group-[]:lg:pl-[250px] peer-[[data-state=open]]:group-[]:xl:pl-[300px]">
          <div className="mx-auto max-w-2xl px-4">
            <MultimodalInput
              input={input}
              setInput={setInput}
              handleSubmit={handleSubmit}
              isLoading={isLoading}
              isStreaming={isStreaming}
              canStop={canStop}
              stop={stop}
              projectId={projectId}
              uploadedFiles={uploadedFiles}
              externalFiles={pendingFiles}
              onExternalFilesChange={setPendingFiles}
              shouldFocus={shouldFocusInput}
            />
          </div>
        </div>
      </div>
    </>
  );
}
