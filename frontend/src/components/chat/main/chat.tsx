import { useEffect, useRef } from "react";
import { useSnapshot } from "valtio";
import { ChatSkeleton, MessageList } from "../display";
import { MultimodalInput } from "../input";
import { uiStore, uiActions } from "@/lib/store/chat/ui-store";
import { chatInputStore, chatInputActions } from "@/lib/store/chat-input-store";
import { useChat } from "@/lib/hooks/use-chat";
import { FileText, PanelLeftOpen, PanelLeftClose } from "lucide-react";
import { Button } from "@/components/ui/button";

export function Chat() {
  // UI state and navigation
  const uiSnapshot = useSnapshot(uiStore, { sync: true });
  const chatInputSnapshot = useSnapshot(chatInputStore);

  // Current conversation info
  const projectId = uiSnapshot.currentProject;
  const conversationId = uiSnapshot.currentConversation;

  // Refs for tracking state changes
  const dragCounter = useRef(0);

  // Chat state and actions
  const {
    currentMessages,
    conversationMap,
    sendMessage,
    stopStreaming: _stopStreaming,
    isConnected,
    isStreaming,
    // testToolEvents: _testToolEvents, // Keep for potential future debug use
  } = useChat();

  // Debug logging for conversation data
  useEffect(() => {
  }, [conversationId, conversationMap, currentMessages.length]);

  // Update UI WebSocket status
  useEffect(() => {
    uiActions.setWsSubscribed(isConnected);
  }, [isConnected]);

  const handleSubmit = async (
    e: React.FormEvent,
    message: string,
    files?: File[]
  ) => {
    e.preventDefault();
    if (!message.trim()) return;

    chatInputActions.clearInput();
    const allFiles = [...(files || []), ...chatInputSnapshot.pendingFiles];
    chatInputActions.clearPendingFiles();

    // Convert File objects to file paths/names for the WebSocket API
    const fileNames = allFiles.map((f) => f.name);
    sendMessage(message, fileNames.length > 0 ? fileNames : undefined);
  };

  // Drag and drop handlers
  const handleDragEnter = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounter.current++;
    if (e.dataTransfer.items?.length > 0) {
      uiActions.setDragging(true);
    }
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounter.current--;
    if (dragCounter.current === 0) {
      uiActions.setDragging(false);
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    uiActions.setDragging(false);
    dragCounter.current = 0;

    if (e.dataTransfer.files?.length > 0) {
      const droppedFiles = Array.from(e.dataTransfer.files);
      chatInputActions.addPendingFiles(droppedFiles);
      e.dataTransfer.clearData();
    }
  };

  // Early return if no conversation ID
  if (!conversationId) {
    return null;
  }

  return (
    <>
      <div
        className="group w-full overflow-auto pl-0 relative flex flex-col h-full"
        onDragEnter={handleDragEnter}
        onDragLeave={handleDragLeave}
        onDragOver={handleDragOver}
        onDrop={handleDrop}
      >
        {/* Floating sidebar toggle button */}
        <Button
          variant="outline"
          size="sm"
          onClick={() => uiActions.toggleSidebar()}
          className="fixed top-4 left-4 z-30 h-10 w-10 p-0 rounded-full shadow-lg bg-background border hidden md:flex"
        >
          {uiSnapshot.isSidebarCollapsed ? (
            <PanelLeftOpen className="h-5 w-5" />
          ) : (
            <PanelLeftClose className="h-5 w-5" />
          )}
        </Button>
        {/* Full-screen drop zone overlay */}
        {uiSnapshot.isDraggingOver && (
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
          <div className="flex-1 flex flex-col">
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
            ) : conversationId && !conversationMap[conversationId] ? (
              // Show loading while conversation is being loaded
              <>
                <ChatSkeleton />
              </>
            ) : currentMessages.length === 0 ? (
              // Conversation is loaded but has no messages - this is a valid empty state
              <div className="flex flex-col relative flex-1">
                <MessageList />
              </div>
            ) : (
              <div className="flex flex-col relative flex-1">
                <MessageList />
              </div>
            )}
          </div>
        </div>

        <div className="bg-background border-t">
          <div className="mx-auto max-w-2xl sm:px-4">
            <MultimodalInput
              input={chatInputSnapshot.input}
              setInput={chatInputActions.setInput}
              handleSubmit={handleSubmit}
              isLoading={false}
              isStreaming={isStreaming}
              projectId={projectId || undefined}
              uploadedFiles={[]}
              externalFiles={[...chatInputSnapshot.pendingFiles]}
              onExternalFilesChange={chatInputActions.setPendingFiles}
              shouldFocus={uiSnapshot.shouldFocusInput}
              isSubscribed={uiSnapshot.isWsSubscribed}
              conversationId={conversationId || undefined}
            />
          </div>
        </div>
      </div>
    </>
  );
}
