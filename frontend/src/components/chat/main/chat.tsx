import { useEffect, useRef } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { useSnapshot } from "valtio";
import { logger } from "@/lib/utils/logger";
import { Messages, ChatSkeleton, WelcomeScreen } from "../display";
import type { Message } from "../display/types";
import { MultimodalInput } from "../input";
import { ContextIndicator } from "../display";
import { useInputState } from "@/hooks/use-input-state";
import { useViewportHeight } from "@/hooks/use-viewport-height";
import { api } from "@/lib/utils/api";
import { setConversationError } from "@/store/chat-store";
import {
  AlertTriangle,
  FileText,
  PanelLeftOpen,
  PanelLeftClose,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { WebSocketService } from "@/lib/services/chat/websocket-service";
import { uiStore, uiActions } from "@/store/ui-store";
import { useChat } from "@/hooks/use-chat";

export function Chat() {
  const uiSnapshot = useSnapshot(uiStore, { sync: true });
  const dragCounter = useRef(0);
  const navigate = useNavigate();
  const location = useLocation();
  const { viewportHeight } = useViewportHeight();

  // Get routing data from valtio store
  const projectId = uiSnapshot.currentProjectId;
  const conversationId = uiSnapshot.currentConversationId;
  const previousConversationIdRef = useRef(conversationId);

  // Use the new Valtio-based chat hook - must be called before any early returns
  const {
    messages,
    sendMessage,
    triggerResponse,
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
  } = useChat(projectId || "", conversationId || "");

  // Current conversation ID from hook or props

  // Use the input state hook to persist input across conversation switches
  const {
    draftMessage: input,
    setDraftMessage: setInput,
    attachments: pendingFiles,
    setAttachments: setPendingFiles,
  } = useInputState(conversationId || "");

  // Get WebSocket subscription status
  const wsService = WebSocketService.getInstance();

  // Reset when conversation changes
  useEffect(() => {
    if (conversationId !== previousConversationIdRef.current) {
      previousConversationIdRef.current = conversationId;
      // Reset any chat-specific UI state
      uiActions.setFocusInput(false);
      uiActions.setDragging(false);
    }
  }, [conversationId]);

  // Monitor WebSocket subscription status (but only for real conversations)
  useEffect(() => {
    const checkSubscription = () => {
      const isSubscribed = wsService.subscribed;
      uiActions.setWsSubscribed(isSubscribed);

      // Try to reconnect if not subscribed
      if (!isSubscribed && projectId && conversationId) {
        console.log("WebSocket not subscribed, attempting to reconnect...");
        wsService.connect().catch((error) => {
          console.error("Failed to reconnect WebSocket:", error);
        });
      }
    };

    // Check every 2 seconds (reduced frequency)
    const interval = setInterval(checkSubscription, 2000);
    checkSubscription();

    return () => clearInterval(interval);
  }, [projectId, conversationId, wsService]);

  // Handle initial message from navigation state
  useEffect(() => {
    const state = location.state as {
      initialMessage?: string;
      initialFiles?: File[];
      fromNewChat?: boolean;
      existingMessages?: Message[];
    } | null;

    if (state?.initialMessage && projectId && conversationId) {
      // If transitioning from new chat, use existing messages to prevent loading skeleton
      if (state.fromNewChat && state.existingMessages) {
        // Set existing messages immediately to prevent loading state
        // This will be handled in useChat hook
      }

      // Send the initial message
      sendMessage(state.initialMessage, state.initialFiles);
      // Clear the navigation state
      window.history.replaceState(null, "", location.pathname);
    }
  }, [projectId, conversationId, location.state, sendMessage]);

  // Focus input when conversation changes (including navigating to /new)
  useEffect(() => {
    if (projectId && conversationId) {
      // Small delay to ensure components are rendered
      const timer = setTimeout(() => {
        uiActions.setFocusInput(true);
        // Reset after focusing
        setTimeout(() => uiActions.setFocusInput(false), 100);
      }, 100);

      return () => clearTimeout(timer);
    }
  }, [projectId, conversationId]);

  const handleSubmit = async (
    e: React.FormEvent,
    message: string,
    files?: File[]
  ) => {
    e.preventDefault();
    console.log(
      "handleSubmit called with projectId:",
      projectId,
      "conversationId:",
      conversationId
    );
    console.log("WebSocket subscribed:", wsService.subscribed);
    console.log("Message:", message);
    console.log("sendMessage function:", typeof sendMessage);
    console.log("WebSocket service methods:", Object.keys(wsService));
    if (!message.trim() || !projectId) return;

    // If there are forgotten messages, restore them first before sending
    if (hasForgottenMessages) {
      await restoreForgottenMessages();
    }

    // Clear the input field
    setInput("");
    // Include any pending files from drag-drop along with form files
    const allFiles = [...(files || []), ...pendingFiles];
    setPendingFiles([]);

    try {
      // For 'new' conversations, no need to check WebSocket - it will be handled in sendMessage
      if (!wsService.subscribed) {
        console.log("WebSocket not connected, attempting to connect...");
        await wsService.connect();
      }

      console.log("Calling sendMessage with:", {
        message,
        fileCount: allFiles.length,
      });

      const result = await sendMessage(
        message,
        allFiles.length > 0 ? allFiles : undefined
      );
      console.log("sendMessage completed:", result);
    } catch (error) {
      console.error("Failed to send message:", error);
      // Set error state to show to user
      setConversationError(
        conversationId || "",
        "Failed to send message. Please try again."
      );
    }
  };

  const handleResendMessage = async (message: any) => {
    if (!projectId) return;

    // If there are forgotten messages, restore them first before resending
    if (hasForgottenMessages) {
      await restoreForgottenMessages();
    }

    // Extract the original content, removing any file attachment mentions
    let content = message.content;

    // Remove the "Attached files:" section if present
    const attachedFilesIndex = content.indexOf("\n\nAttached files:");
    if (attachedFilesIndex > -1) {
      content = content.substring(0, attachedFilesIndex);
    }

    // Use triggerResponse to avoid duplicating the user message
    await triggerResponse(content.trim());
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
            source_conversation_id: hookConversationId || conversationId,
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
        navigate(`/p/${projectId}/c/${data.conversation_id}`);
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
      uiActions.setDragging(true);
    }
  };

  const handleContainerDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounter.current--;
    if (dragCounter.current === 0) {
      uiActions.setDragging(false);
    }
  };

  const handleContainerDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  };

  const handleContainerDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    uiActions.setDragging(false);
    dragCounter.current = 0;

    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      const droppedFiles = Array.from(e.dataTransfer.files);

      setPendingFiles([...pendingFiles, ...droppedFiles]);
      e.dataTransfer.clearData();
    }
  };

  // Early return if no conversation ID - must be after all hooks and handlers
  if (!conversationId) {
    return null;
  }

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
        <Button
          variant="outline"
          size="sm"
          onClick={() => uiActions.toggleSidebar()}
          className="fixed top-4 left-4 z-30 h-10 w-10 p-0 rounded-full shadow-lg bg-background border hidden md:flex"
        >
          {uiSnapshot.isMobile ? (
            true
          ) : uiSnapshot.isSidebarCollapsed ? (
            <PanelLeftOpen className="h-5 w-5" />
          ) : (
            <PanelLeftClose className="h-5 w-5" />
          )}
        </Button>
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
                          navigate("/projects", { replace: true });
                        }}
                        className="text-sm px-3 py-1 bg-red-600 text-white rounded hover:bg-red-700 transition-colors"
                      >
                        Go to Projects
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
            ) : isLoadingMessages &&
              messages.length === 0 &&
              !isStreaming &&
              !uiSnapshot.isTransitioningFromNew ? (
              // Only show skeleton when initially loading a conversation with no messages yet
              // Don't show if we're streaming or transitioning from new chat
              <ChatSkeleton />
            ) : messages.length === 0 ? (
              <WelcomeScreen />
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
            {contextUsage && (
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
              shouldFocus={uiSnapshot.shouldFocusInput}
              isSubscribed={uiSnapshot.isWsSubscribed}
              conversationId={conversationId}
              className="lg:ml-[-10px] lg:mr-[40px]"
            />
          </div>
        </div>
      </div>
    </>
  );
}
