import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { ArrowDown } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { Virtuoso, VirtuosoHandle } from "react-virtuoso";
import { LoadingIndicator } from "./loading-indicator";
import { MessageListItem } from "./message-list-item";
import { WelcomeScreen } from "../ui/welcome-screen";

import { MessagesProps, DisplayMessage } from "../types";


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
  activeTools = [],
  onResendMessage,
  onNewChatFromHere,
  onAskUserSubmit,
}: MessagesProps) {
  const virtuosoRef = useRef<VirtuosoHandle>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const previousConversationId = useRef(conversationId);
  const [showNewMessageAlert, setShowNewMessageAlert] = useState(false);
  const [editingQueuedId, setEditingQueuedId] = useState<string | null>(null);
  const [editingContent, setEditingContent] = useState("");
  const [isAtBottom, setIsAtBottom] = useState(true);
  const previousMessageCount = useRef(messages.length);
  const scrollTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const [isInitialized, setIsInitialized] = useState(false);

  // Handlers for queue editing
  const handleStartEdit = (messageId: string) => {
    const queuedMessage = messageQueue.find((m) => m.id === messageId);
    if (queuedMessage) {
      setEditingQueuedId(messageId);
      setEditingContent(queuedMessage.content);
    }
  };

  const handleSaveEdit = () => {
    if (editingQueuedId && onEditQueued) {
      onEditQueued(editingQueuedId, editingContent);
      setEditingQueuedId(null);
      setEditingContent("");
    }
  };

  const handleCancelEdit = () => {
    setEditingQueuedId(null);
    setEditingContent("");
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
            editingQueuedId === queuedMsg.id
              ? editingContent
              : queuedMsg.content,
          role: "user" as const,
          createdAt: queuedMsg.timestamp,
          isQueued: true,
          queuePosition: index + 1,
          isEditing: editingQueuedId === queuedMsg.id,
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
    editingQueuedId,
    editingContent,
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

  // Find the latest TodoWrite tool usage across all messages
  const latestTodoWrite = useMemo(() => {
    for (let i = messages.length - 1; i >= 0; i--) {
      // Get all TodoWrite usages in this message
      const todoUsages =
        messages[i].tool_usages?.filter((tu) => tu.tool_name === "TodoWrite") ||
        [];

      // If this message has TodoWrite usages, return the last one
      if (todoUsages.length > 0) {
        const lastTodoUsage = todoUsages[todoUsages.length - 1];
        return {
          messageId: messages[i].id,
          todos: (() => {
            const params = lastTodoUsage.parameters;
            // Handle both direct object and stringified JSON
            if (typeof params === "string") {
              try {
                const parsed = JSON.parse(params);
                return parsed.todos || [];
              } catch (e) {
                console.error("Failed to parse TodoWrite parameters:", e);
                return [];
              }
            }
            return params?.todos || [];
          })(),
        };
      }
    }
    return null;
  }, [messages]);

  // Utility function to scroll to bottom
  const scrollToBottom = (behavior: "smooth" | "auto" = "auto") => {
    console.log("Messages: scrollToBottom called", { behavior, itemsLength: allItems.length, virtuosoRef: !!virtuosoRef.current });
    // Cancel any pending scroll operations
    if (scrollTimeoutRef.current) {
      clearTimeout(scrollTimeoutRef.current);
      scrollTimeoutRef.current = null;
    }

    if (virtuosoRef.current) {
      // Use scrollTo for more reliable scrolling to bottom
      virtuosoRef.current.scrollTo({
        top: 999999999, // Large number to ensure we scroll to bottom
        behavior,
      });
    }
  };

  // Initialize Virtuoso immediately when messages are available
  useEffect(() => {
    if (!isInitialized && messages.length > 0) {
      setIsInitialized(true);
    }
  }, [messages.length, isInitialized]);

  // Scroll to bottom on initial load only
  useEffect(() => {
    // Reset when conversation changes
    if (conversationId !== previousConversationId.current) {
      setShowNewMessageAlert(false);
      setIsInitialized(false);
      previousConversationId.current = conversationId;
      previousMessageCount.current = messages.length;
      // Re-initialize immediately after conversation change
      setIsInitialized(true);
    }
  }, [allItems.length, conversationId]);

  // Handle new messages and content changes scrolling behavior
  useEffect(() => {
    const hasNewMessage = messages.length > previousMessageCount.current;
    
    if (hasNewMessage) {
      // New message received
      const lastMessage = messages[messages.length - 1];

      // Always scroll to bottom for user messages (when user sends a message)
      if (lastMessage.role === "user") {
        // Add a small delay to ensure the message is rendered
        setTimeout(() => {
          scrollToBottom("smooth");
        }, 300);
        setShowNewMessageAlert(false);
      }
      // For assistant messages
      else if (lastMessage.role === "assistant") {
        if (isAtBottom) {
          // Auto-scroll if user is at bottom - use auto for streaming messages
          scrollToBottom("auto");
        } else {
          // Show alert if user has scrolled up
          setShowNewMessageAlert(true);
        }
      }
      previousMessageCount.current = messages.length;
    } else if (isAtBottom && messages.length > 0) {
      // No new message but content might have changed (tool results, streaming, etc.)
      // Auto-scroll to bottom if user is already at bottom to maintain position
      scrollToBottom("auto");
    }
  }, [messages, isAtBottom]);

  const handleScrollToBottom = () => {
    scrollToBottom("smooth");
    setShowNewMessageAlert(false);
  };

  return (
    <div className={cn("flex flex-col h-full")}>
      {messages.length === 0 ? (
        <WelcomeScreen />
      ) : (
        <div
          className={cn("flex-1 relative")}
          ref={containerRef}
          style={{
            transform: "translateZ(0)",
            backfaceVisibility: "hidden",
          }}
        >
          {previousConversationId.current === "new" ? (
            <div className="flex absolute items-center justify-center inset-0">
              <Button
                size="sm"
                disabled
                className="rounded-full shadow-lg animate-in fade-in slide-in-from-bottom-2 duration-300"
              >
                Creating new chat...
              </Button>
            </div>
          ) : (
            <>
              {previousConversationId.current === conversationId && (
                  <Virtuoso
                    ref={virtuosoRef}
                    data={allItems}
                    initialTopMostItemIndex={
                      allItems.length > 0
                        ? Math.max(0, allItems.length - 1)
                        : undefined
                    }
                    defaultItemHeight={100}
                    fixedItemHeight={undefined}
                    atBottomStateChange={(atBottom) => {
                      setIsAtBottom(atBottom);
                      if (atBottom) {
                        setShowNewMessageAlert(false);
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
                          <div className="flex max-w-[45rem] mx-auto p-2" style={{ minHeight: "80px" }}>
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
                          <div style={{ height: "200px", minHeight: "200px" }} />
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
                            editingContent={editingContent}
                            setEditingContent={setEditingContent}
                            onResendMessage={onResendMessage}
                            isLastUserMessage={item.id === lastUserMessageId}
                            onNewChatFromHere={onNewChatFromHere}
                            latestTodoWrite={latestTodoWrite}
                            onAskUserSubmit={onAskUserSubmit}
                            isStreaming={_isStreaming}
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
                    }}
                  />
                )}

              {/* New message indicator */}
              {showNewMessageAlert && (
                <div className="absolute bottom-[60px] left-1/2 transform -translate-x-1/2 z-10">
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
          )}
        </div>
      )}
    </div>
  );
}
