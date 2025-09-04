import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { ArrowDown } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useSnapshot } from "valtio";
import {
  messageUIActions,
  messageUIStore,
} from "@/lib/store/chat/message-ui-store";
import { Virtuoso } from "react-virtuoso";
import { LoadingIndicator } from "./loading-indicator";
import { MessageListItem } from "./message-list-item";

import {
  MessagesProps,
  DisplayMessage,
  ActiveToolInfo,
  Message,
  ToolUsage,
} from "../types";

export function Messages({
  messages,
  isLoading,
  onForgetFrom,
  conversationId,
  isStreaming = false,
  canStop = false,
  onStop,
  activeTools = [],
  onResendMessage,
  onNewChatFromHere,
  onAskUserSubmit,
}: MessagesProps) {
  const messageUISnapshot = useSnapshot(messageUIStore);
  const conversationState =
    messageUIActions.getConversationState(conversationId);

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
      setThinkingWordIndex((prevIndex: number) => {
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

  // Combine messages with loading indicator
  let allItems = useMemo(() => {
    // Clear items immediately when conversation changes
    if (conversationId !== conversationState.previousConversationId) {
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
        createdAt: new Date().toISOString(),
      } as DisplayMessage);
    }

    return items;
  }, [messages, isLoading, activeTools.length]);

  if (messages.length > 0 && allItems.length === 0) {
    allItems = messages;
  }

  // Find the last user message (excluding queued messages)
  const lastUserMessageId = useMemo(() => {
    const typedMessages = messages as Message[];
    for (let i = typedMessages.length - 1; i >= 0; i--) {
      if (typedMessages[i].role === "user") {
        return typedMessages[i].id;
      }
    }
    return null;
  }, [messages]);

  // Find the currently streaming assistant message (if any)
  const streamingMessage = useMemo(() => {
    if (!isStreaming) return null;

    // Find the last assistant message (which should be the streaming one)
    const typedMessages = messages as Message[];
    for (let i = typedMessages.length - 1; i >= 0; i--) {
      if (typedMessages[i].role === "assistant") {
        return typedMessages[i];
      }
    }
    return null;
  }, [messages, isStreaming]);

  // Check if messages contain charts or other async content
  const hasAsyncContent = useMemo(() => {
    // Check if any message has tool usages that might contain charts/tables
    const typedMessages = messages as Message[];
    return typedMessages.some((msg) =>
      msg.tool_usages?.some((usage: ToolUsage) => {
        const output = usage.output;
        if (typeof output === "string") {
          return (
            output.includes("interaction_type") &&
            (output.includes('"chart"') || output.includes('"table"'))
          );
        }
        if (typeof output === "object" && output) {
          const outputObj = output as { interaction_type?: string };
          return (
            outputObj.interaction_type === "chart" ||
            outputObj.interaction_type === "table"
          );
        }
        return false;
      })
    );
  }, [messages]);

  // Utility function to scroll to bottom
  const scrollToBottom = (behavior: "smooth" | "auto" = "auto") => {
    messageUIActions.scrollToBottom(behavior, allItems.length);
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
    if (messageUISnapshot.isInitialized) {
      messageUIActions.setVirtuosoReady(false);
    }
  }, [allItems.length]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      messageUIActions.clearTimeouts();
    };
  }, []);

  // Show Virtuoso when it's ready
  useEffect(() => {
    if (messageUISnapshot.isVirtuosoReady && messageUISnapshot.isInitialized) {
      // Show immediately for instant experience
      // Scroll after next frame
      requestAnimationFrame(() => {
        scrollToBottom("auto");
      });
    }
  }, [messageUISnapshot.isVirtuosoReady, messageUISnapshot.isInitialized]);

  // Hide Virtuoso immediately when conversation changes (before messages update)
  useEffect(() => {
    // Reset immediately when conversation ID changes
    if (conversationId !== conversationState.previousConversationId) {
      // Hide Virtuoso immediately to prevent flash
      messageUIActions.setVirtuosoReady(false);
      messageUIActions.setInitialized(false);

      // Clear any pending timeouts
      messageUIActions.clearTimeouts();

      messageUIActions.setShowNewMessageAlert(false, conversationId);
      messageUIActions.setScrollHeight(conversationId || "", 0); // Reset scroll height tracking
      messageUIActions.setPreviousConversationId(
        conversationId || "",
        conversationId
      );
      messageUIActions.setPreviousMessageCount(conversationId || "", 0); // Reset to 0 so we can detect when new messages arrive
    }
  }, [conversationId, conversationState.previousConversationId]);

  // Initialize when messages arrive for the current conversation
  useEffect(() => {
    if (
      conversationId === conversationState.previousConversationId &&
      messages.length > 0 &&
      !messageUISnapshot.isInitialized
    ) {
      messageUIActions.setInitialized(true);
      messageUIActions.setPreviousMessageCount(
        conversationId || "",
        messages.length
      );
      // Wait for Virtuoso to signal it's ready
    }
  }, [
    messages.length,
    conversationId,
    messageUISnapshot.isInitialized,
    conversationState.previousConversationId,
  ]);

  // Handle new messages and content changes scrolling behavior
  useEffect(() => {
    const hasNewMessage =
      messages.length > conversationState.previousMessageCount;

    if (hasNewMessage) {
      // New message received
      const typedMessages = messages as Message[];
      const lastMessage = typedMessages[typedMessages.length - 1];

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
      messageUIActions.setPreviousMessageCount(
        conversationId || "",
        messages.length
      );
    } else if (conversationState.isAtBottom && messages.length > 0) {
      // No new message but content might have changed (tool results, streaming, etc.)
      // Auto-scroll to bottom if user is already at bottom to maintain position
      scrollToBottom("auto");
    }
  }, [
    messages,
    conversationState.isAtBottom,
    conversationState.previousMessageCount,
    conversationId,
  ]);

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

  return (
    <>
      <div
        className={cn(
          "flex flex-col h-full transition-all",
          messageUISnapshot.isVirtuosoReady ? "opacity-100" : "opacity-0"
        )}
      >
        <Virtuoso
          ref={(ref) => messageUIActions.setVirtuosoRef(ref)}
          data={allItems}
          initialTopMostItemIndex={
            allItems.length > 0 ? Math.max(0, allItems.length - 1) : undefined
          }
          defaultItemHeight={100}
          fixedItemHeight={undefined}
          scrollerRef={(ref) => {
            if (ref && !messageUISnapshot.isVirtuosoReady) {
              // Clear any existing timeouts
              messageUIActions.setScrollStabilityCheckTimeout(null);

              // Simplified stability check - much faster
              const checkScrollStability = () => {
                let _ref = ref as HTMLElement;
                if (!_ref) return;

                const currentHeight = _ref.scrollHeight;

                // If we have any height, we're ready
                if (currentHeight > 0) {
                  // Quick check for async content
                  const waitTime = hasAsyncContent ? 100 : 20;
                  const timeout = setTimeout(() => {
                    messageUIActions.setVirtuosoReady(true);
                  }, waitTime);
                  messageUIActions.setScrollStabilityCheckTimeout(timeout);
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
                  if (conversationId) {
                    messageUIActions.setScrollHeight(
                      conversationId,
                      (ref as HTMLElement).scrollHeight
                    );
                  }
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
            const typedItem = item as Message;
            if (typedItem.id === "loading") {
              return (
                <div style={{ minHeight: "60px" }}>
                  {topPadding}
                  <LoadingIndicator
                    activeTools={activeTools as ActiveToolInfo[]}
                    canStop={canStop}
                    onStop={onStop}
                    thinkingWord={thinkingWords[thinkingWordIndex]}
                    streamingMessageTools={streamingMessage?.tool_usages}
                    messages={messages as Message[]}
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
                  onResendMessage={onResendMessage}
                  isLastUserMessage={typedItem.id === lastUserMessageId}
                  onNewChatFromHere={onNewChatFromHere}
                  onAskUserSubmit={onAskUserSubmit}
                  isStreaming={
                    isStreaming &&
                    typedItem.role === "assistant" &&
                    // Check if this is the last non-loading message
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
    </>
  );
}
