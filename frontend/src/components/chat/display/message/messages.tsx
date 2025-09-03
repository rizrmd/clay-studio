import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { ArrowDown } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useSnapshot } from "valtio";
import { messageUIStore, messageUIActions } from "@/store/message-ui-store";
import { Virtuoso, VirtuosoHandle } from "react-virtuoso";
import { LoadingIndicator } from "./loading-indicator";
import { MessageListItem } from "./message-list-item";
import { WelcomeScreen } from "../ui/welcome-screen";
import { ChatSkeleton } from "../ui/chat-skeleton";

import { MessagesProps, DisplayMessage } from "../types";
import { createPortal } from "react-dom";

export function Messages({
  messages,
  isLoading,
  onForgetFrom,
  conversationId,
  messageQueue = [],
  onEditQueued,
  onCancelQueued,
  isProcessingQueue: _isProcessingQueue = false,
  isStreaming: _isStreaming = false,
  canStop = false,
  onStop,
  activeTools = [] as any,
  onResendMessage,
  onNewChatFromHere,
  onAskUserSubmit,
}: MessagesProps) {
  const virtuosoRef = useRef<VirtuosoHandle>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const previousConversationId = useRef(conversationId);
  const messageUISnapshot = useSnapshot(messageUIStore);
  const conversationState = messageUIActions.getConversationState(conversationId || "");
  const previousMessageCount = useRef(messages.length);
  const scrollTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const scrollerRef = useRef<HTMLElement | null>(null);
  const contentStableTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const scrollHeightRef = useRef<number>(0);
  const scrollStabilityCheckRef = useRef<NodeJS.Timeout | null>(null);

  // Handlers for queue editing
  const handleStartEdit = (messageId: string) => {
    const queuedMessage = messageQueue.find((m) => m.id === messageId);
    if (queuedMessage) {
      messageUIActions.startEditingQueued(messageId, queuedMessage.content);
    }
  };

  const handleSaveEdit = () => {
    if (messageUISnapshot.editingQueuedId && onEditQueued) {
      onEditQueued(messageUISnapshot.editingQueuedId, messageUISnapshot.editingContent);
      messageUIActions.stopEditingQueued();
    }
  };

  const handleCancelEdit = () => {
    messageUIActions.stopEditingQueued();
  };

  // Fun thinking words to display while loading
  const thinkingWords = useMemo(
    () => [
      "Pondering",
      "Processing",
      "Computing",
      "Thinking",
      "Contemplating",
      "Brewing",
      "Cogitating",
      "Mulling",
      "Ruminating",
      "Musing",
      "Meditating",
      "Reflecting",
      "Deliberating",
      "Wondering",
      "Daydreaming",
      "Percolating",
      "Simmering",
      "Marinating",
      "Stewing",
      "Incubating",
      "Noodling",
      "Churning",
      "Cooking",
      "Baking",
      "Steeping",
      "Bubbling",
      "Fizzing",
      "Sparkling",
      "Tingling",
      "Whirring",
      "Humming",
      "Buzzing",
      "Ticking",
      "Clicking",
      "Whizzing",
      "Stirring",
      "Swirling",
      "Mixing",
      "Blending",
      "Whisking",
      "Kneading",
      "Folding",
      "Weaving",
      "Knitting",
      "Spinning",
      "Crystallizing",
      "Distilling",
      "Fermenting",
      "Ripening",
      "Maturing",
      "Hatching",
      "Germinating",
      "Sprouting",
      "Blooming",
      "Unfurling",
      "Awakening",
      "Emerging",
      "Materializing",
      "Manifesting",
      "Conjuring",
    ],
    []
  );

  const [thinkingWordIndex, setThinkingWordIndex] = useState(() =>
    Math.floor(Math.random() * thinkingWords.length)
  );

  // Rotate thinking word every 3-5 seconds while loading
  useEffect(() => {
    if (!isLoading) return;

    const interval = setInterval(() => {
      setThinkingWordIndex((prevIndex) => {
        // Pick a different random word
        let newIndex = Math.floor(Math.random() * thinkingWords.length);
        // Ensure we don't show the same word twice in a row
        while (newIndex === prevIndex && thinkingWords.length > 1) {
          newIndex = Math.floor(Math.random() * thinkingWords.length);
        }
        return newIndex;
      });
    }, 3000 + Math.random() * 2000); // Random interval between 3-5 seconds

    return () => clearInterval(interval);
  }, [isLoading, thinkingWords.length]);

  // Combine messages with loading indicator and queued messages
  const allItems = useMemo(() => {
    // Clear items immediately when conversation changes
    if (conversationId !== previousConversationId.current) {
      return [];
    }

    // No need to filter here since message-service already filters empty assistant messages
    const items: DisplayMessage[] = [...messages];

    // Add loading indicator if AI is responding
    if (isLoading) {
      items.push({
        id: "loading",
        content: "",
        role: "assistant" as const,
        createdAt: new Date(),
      });
    }

    // Add queued messages after loading indicator
    if (messageQueue.length > 0) {
      messageQueue.forEach((queuedMsg, index) => {
        items.push({
          id: queuedMsg.id,
          content:
            messageUISnapshot.editingQueuedId === queuedMsg.id
              ? messageUISnapshot.editingContent
              : queuedMsg.content,
          role: "user" as const,
          createdAt: queuedMsg.timestamp,
          isQueued: true,
          queuePosition: index + 1,
          isEditing: messageUISnapshot.editingQueuedId === queuedMsg.id,
          file_attachments: queuedMsg.files?.map((f) => ({
            id: `file-${f.name}`,
            file_name: f.name,
            original_name: f.name,
            file_path: "",
            file_size: f.size,
          })),
        });
      });
    }

    return items;
  }, [
    messages,
    isLoading,
    messageQueue,
    messageUISnapshot.editingQueuedId,
    messageUISnapshot.editingContent,
    activeTools.length,
  ]);

  // Find the last user message (excluding queued messages)
  const lastUserMessageId = useMemo(() => {
    for (let i = messages.length - 1; i >= 0; i--) {
      if (messages[i].role === "user") {
        return messages[i].id;
      }
    }
    return null;
  }, [messages]);

  // Find the currently streaming assistant message (if any)
  const streamingMessage = useMemo(() => {
    if (!_isStreaming) return null;

    // Find the last assistant message (which should be the streaming one)
    for (let i = messages.length - 1; i >= 0; i--) {
      if (messages[i].role === "assistant") {
        return messages[i];
      }
    }
    return null;
  }, [messages, _isStreaming]);

  // Check if messages contain charts or other async content
  const hasAsyncContent = useMemo(() => {
    // Check if any message has tool usages that might contain charts/tables
    return messages.some((msg) =>
      msg.tool_usages?.some((usage) => {
        const output = usage.output;
        if (typeof output === "string") {
          return (
            output.includes("interaction_type") &&
            (output.includes('"chart"') || output.includes('"table"'))
          );
        }
        if (typeof output === "object" && output) {
          return (
            output.interaction_type === "chart" ||
            output.interaction_type === "table"
          );
        }
        return false;
      })
    );
  }, [messages]);

  // Utility function to scroll to bottom
  const scrollToBottom = (behavior: "smooth" | "auto" = "auto") => {
    // Cancel any pending scroll operations
    if (scrollTimeoutRef.current) {
      clearTimeout(scrollTimeoutRef.current);
      scrollTimeoutRef.current = null;
    }

    if (virtuosoRef.current) {
      // Use scrollToIndex to scroll to the last item for better reliability
      const lastIndex = allItems.length - 1;
      if (lastIndex >= 0) {
        virtuosoRef.current.scrollToIndex({
          index: lastIndex,
          behavior,
          align: "end",
        });
        // Also call scrollTo to ensure we're at the absolute bottom
        setTimeout(() => {
          virtuosoRef.current?.scrollTo({
            top: Number.MAX_SAFE_INTEGER,
            behavior,
          });
        }, 50);
      }
    }
  };

  // Initialize Virtuoso when messages are available
  useEffect(() => {
    if (!messageUISnapshot.isInitialized && messages.length > 0) {
      messageUIActions.setInitialized(true);
      messageUIActions.setVirtuosoReady(false);
      // Don't show immediately - wait for Virtuoso to signal it's ready
    }
  }, [messages.length, messageUISnapshot.isInitialized]);

  // Reset ready state when allItems changes significantly (for re-initialization)
  useEffect(() => {
    if (messageUISnapshot.isInitialized && !messageUISnapshot.showVirtuoso) {
      messageUIActions.setVirtuosoReady(false);
    }
  }, [allItems.length]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (contentStableTimeoutRef.current) {
        clearTimeout(contentStableTimeoutRef.current);
      }
      if (scrollTimeoutRef.current) {
        clearTimeout(scrollTimeoutRef.current);
      }
      if (scrollStabilityCheckRef.current) {
        clearTimeout(scrollStabilityCheckRef.current);
      }
    };
  }, []);

  // Show Virtuoso when it's ready
  useEffect(() => {
    if (messageUISnapshot.isVirtuosoReady && messageUISnapshot.isInitialized && !messageUISnapshot.showVirtuoso) {
      // Show immediately for instant experience
      messageUIActions.setShowVirtuoso(true);
      // Scroll after next frame
      requestAnimationFrame(() => {
        scrollToBottom("auto");
      });
    }
  }, [messageUISnapshot.isVirtuosoReady, messageUISnapshot.isInitialized, messageUISnapshot.showVirtuoso]);

  // Hide Virtuoso immediately when conversation changes (before messages update)
  useEffect(() => {
    // Reset immediately when conversation ID changes
    if (conversationId !== previousConversationId.current) {
      // Hide Virtuoso immediately to prevent flash
      messageUIActions.setShowVirtuoso(false);
      messageUIActions.setVirtuosoReady(false);
      messageUIActions.setInitialized(false);

      // Clear any pending timeouts
      if (contentStableTimeoutRef.current) {
        clearTimeout(contentStableTimeoutRef.current);
        contentStableTimeoutRef.current = null;
      }
      if (scrollStabilityCheckRef.current) {
        clearTimeout(scrollStabilityCheckRef.current);
        scrollStabilityCheckRef.current = null;
      }

      messageUIActions.setShowNewMessageAlert(false, conversationId);
      scrollHeightRef.current = 0; // Reset scroll height tracking
      previousConversationId.current = conversationId;
      previousMessageCount.current = 0; // Reset to 0 so we can detect when new messages arrive
    }
  }, [conversationId]); // Only depend on conversationId, not messages

  // Initialize when messages arrive for the current conversation
  useEffect(() => {
    if (
      conversationId === previousConversationId.current &&
      messages.length > 0 &&
      !messageUISnapshot.isInitialized
    ) {
      messageUIActions.setInitialized(true);
      previousMessageCount.current = messages.length;
      // Wait for Virtuoso to signal it's ready
    }
  }, [messages.length, conversationId, messageUISnapshot.isInitialized]);

  // Handle new messages and content changes scrolling behavior
  useEffect(() => {
    const hasNewMessage = messages.length > previousMessageCount.current;

    if (hasNewMessage) {
      // New message received
      const lastMessage = messages[messages.length - 1];

      // Always scroll to bottom for user messages (when user sends a message)
      if (lastMessage.role === "user") {
        // Scroll immediately for instant feedback
        requestAnimationFrame(() => {
          scrollToBottom("smooth");
        });
        messageUIActions.setShowNewMessageAlert(false, conversationId);
      }
      // For assistant messages
      else if (lastMessage.role === "assistant") {
        // Scroll immediately if at bottom
        if (conversationState.isAtBottom) {
          requestAnimationFrame(() => {
            scrollToBottom("smooth");
          });
        } else {
          // Show alert if user has scrolled up
          messageUIActions.setShowNewMessageAlert(true, conversationId);
        }
      }
      previousMessageCount.current = messages.length;
    } else if (conversationState.isAtBottom && messages.length > 0) {
      // No new message but content might have changed (tool results, streaming, etc.)
      // Auto-scroll to bottom if user is already at bottom to maintain position
      scrollToBottom("auto");
    }
  }, [messages, conversationState.isAtBottom]);

  // Handle loading state changes (when loading indicator appears/disappears)
  useEffect(() => {
    if (isLoading && conversationState.isAtBottom) {
      requestAnimationFrame(() => {
        scrollToBottom("auto");
      });
    }
  }, [isLoading, conversationState.isAtBottom]);

  const handleScrollToBottom = () => {
    scrollToBottom("smooth");
    messageUIActions.setShowNewMessageAlert(false, conversationId);
  };

  const portal = document.getElementById("portal-body");
  return (
    <div className={cn("flex flex-col h-full")}>
      {portal &&
        createPortal(
          <div
            className={cn(
              !messageUISnapshot.showVirtuoso
                ? "opacity-100 transition-opacity duration-200"
                : "opacity-0 pointer-events-none",
              "hidden md:fixed right-0 bottom-0 top-0 w-[50px]"
            )}
          ></div>,
          portal
        )}
      <div
        className={cn("flex-1 relative")}
        ref={containerRef}
        style={{
          transform: "translateZ(0)",
          backfaceVisibility: "hidden",
        }}
      >
        {/* Always keep Virtuoso mounted, just control visibility */}
        <div
          className={cn(
            "absolute inset-0 transition-all",
            messageUISnapshot.showVirtuoso ? "opacity-100" : "opacity-0"
          )}
        >
          <Virtuoso
            ref={virtuosoRef}
            data={allItems}
            initialTopMostItemIndex={
              allItems.length > 0 ? Math.max(0, allItems.length - 1) : undefined
            }
            defaultItemHeight={100}
            fixedItemHeight={undefined}
            scrollerRef={(ref) => {
              scrollerRef.current = ref as HTMLElement;
              if (ref && !messageUISnapshot.isVirtuosoReady) {
                // Clear any existing timeouts
                if (scrollStabilityCheckRef.current) {
                  clearTimeout(scrollStabilityCheckRef.current);
                }

                // Simplified stability check - much faster
                const checkScrollStability = () => {
                  let _ref = ref as HTMLElement;
                  if (!_ref) return;

                  const currentHeight = _ref.scrollHeight;

                  // If we have any height, we're ready
                  if (currentHeight > 0) {
                    // Quick check for async content
                    const waitTime = hasAsyncContent ? 100 : 20;
                    scrollStabilityCheckRef.current = setTimeout(() => {
                      messageUIActions.setVirtuosoReady(true);
                    }, waitTime);
                  } else {
                    // No height yet, check again
                    requestAnimationFrame(checkScrollStability);
                  }
                };

                // Start monitoring after a few frames to let Virtuoso initialize
                let initFrames = 0;
                const waitForInit = () => {
                  initFrames++;
                  if (initFrames < 3) {
                    requestAnimationFrame(waitForInit);
                  } else {
                    scrollHeightRef.current = (ref as HTMLElement).scrollHeight;
                    requestAnimationFrame(checkScrollStability);
                  }
                };
                requestAnimationFrame(waitForInit);
              }
            }}
            atBottomStateChange={(atBottom) => {
              messageUIActions.setIsAtBottom(atBottom, conversationId);
              if (atBottom) {
                messageUIActions.setShowNewMessageAlert(false, conversationId);
              }
            }}
            atBottomThreshold={50}
            overscan={{ main: 500, reverse: 500 }}
            increaseViewportBy={{ top: 200, bottom: 200 }}
            scrollSeekConfiguration={{
              enter: (velocity) => Math.abs(velocity) > 1000,
              exit: (velocity) => Math.abs(velocity) < 300,
              change: (_velocity) => {
                return (
                  <div
                    className="flex max-w-[45rem] mx-auto p-2"
                    style={{ minHeight: "80px" }}
                  >
                    <div className="h-20 w-full bg-muted/30 rounded-lg animate-pulse" />
                  </div>
                );
              },
            }}
            itemContent={(index, item) => {
              // Add top padding for first item
              const topPadding =
                index === 0 ? (
                  <div style={{ height: "100px", minHeight: "100px" }} />
                ) : null;
              // Add bottom padding for last item
              const bottomPadding =
                index === allItems.length - 1 ? (
                  <div style={{ height: "100px", minHeight: "100px" }} />
                ) : null;

              // Loading indicator
              if (item.id === "loading") {
                return (
                  <div style={{ minHeight: "60px" }}>
                    {topPadding}
                    <LoadingIndicator
                      activeTools={activeTools}
                      canStop={canStop}
                      onStop={onStop}
                      thinkingWord={thinkingWords[thinkingWordIndex]}
                      streamingMessageTools={streamingMessage?.tool_usages}
                      messages={messages}
                    />
                    {bottomPadding}
                  </div>
                );
              }

              // Regular message
              return (
                <div style={{ minHeight: "60px" }}>
                  {topPadding}
                  <MessageListItem
                    message={item}
                    onForgetFrom={onForgetFrom}
                    onStartEdit={handleStartEdit}
                    onSaveEdit={handleSaveEdit}
                    onCancelEdit={handleCancelEdit}
                    onCancelQueued={onCancelQueued}
                    editingContent={messageUISnapshot.editingContent}
                    setEditingContent={messageUIActions.updateEditingContent}
                    onResendMessage={onResendMessage}
                    isLastUserMessage={item.id === lastUserMessageId}
                    onNewChatFromHere={onNewChatFromHere}
                    onAskUserSubmit={onAskUserSubmit}
                    isStreaming={
                      _isStreaming &&
                      item.role === "assistant" &&
                      // Check if this is the last non-loading, non-queued message
                      index === messages.length - 1
                    }
                    isLoading={isLoading}
                    allMessages={allItems}
                    messageIndex={index}
                    onScroll={() => scrollToBottom("smooth")}
                  />
                  {bottomPadding}
                </div>
              );
            }}
            followOutput={false}
            className="flex-1"
            style={{
              height: "100%",
              minHeight: "200px",
              overscrollBehavior: "contain",
              WebkitOverflowScrolling: "touch",
              overflow: "auto",
            }}
          />
        </div>

        {/* Show loading or welcome screen as overlay when Virtuoso is hidden or has no data */}
        {(allItems.length === 0 || !messageUISnapshot.showVirtuoso) && (
          <div className="absolute inset-0 bg-background flex justify-center items-center">
            {messages.length === 0 && !isLoading ? (
              <WelcomeScreen />
            ) : (
              <>
                <ChatSkeleton />
              </>
            )}
          </div>
        )}

        {/* New message indicator */}
        {conversationState.showNewMessageAlert && (
          <div className="absolute bottom-[60px] left-1/2 transform -translate-x-1/2 z-20">
            <Button
              onClick={handleScrollToBottom}
              size="sm"
              className="rounded-full shadow-lg animate-in fade-in slide-in-from-bottom-2 duration-300"
            >
              <ArrowDown className="h-4 w-4 mr-2" />
              New message
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
