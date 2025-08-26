import React, { useEffect, useRef, memo, useMemo, useState } from "react";
import { Virtuoso, VirtuosoHandle } from "react-virtuoso";
import { css } from "goober";
import {
  Bot,
  User,
  MoreVertical,
  Trash2,
  FileText,
  Download,
  ArrowDown,
} from "lucide-react";
import { cn } from "@/lib/utils";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import rehypeRaw from "rehype-raw";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { API_BASE_URL } from "@/lib/url";

interface FileAttachment {
  id: string;
  file_name: string;
  original_name: string;
  file_path: string;
  file_size: number;
  mime_type?: string;
  description?: string;
  auto_description?: string;
}

interface Message {
  id: string;
  content: string;
  role: "user" | "assistant" | "system";
  createdAt: Date;
  file_attachments?: FileAttachment[];
}

interface MessagesProps {
  messages: Message[];
  isLoading?: boolean;
  onForgetFrom?: (messageId: string) => void;
  conversationId?: string; // Add conversationId to detect conversation switches
}

// Helper function to format file size
const formatFileSize = (bytes: number) => {
  if (bytes === 0) return "0 Bytes";
  const k = 1024;
  const sizes = ["Bytes", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + " " + sizes[i];
};

// Helper function to download file
const handleDownloadFile = async (file: FileAttachment) => {
  try {
    const clientId = localStorage.getItem("activeClientId");
    const projectId = localStorage.getItem("activeProjectId");
    if (!clientId || !projectId) return;

    const fileName = file.file_path.split("/").pop();
    const downloadUrl = `${API_BASE_URL}/uploads/${clientId}/${projectId}/${fileName}`;

    const link = document.createElement("a");
    link.href = downloadUrl;
    link.download = file.original_name || fileName || "";
    link.target = "_blank";
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  } catch (error) {
    // Error downloading file
  }
};

// Memoized individual message component
const MessageItem = memo(
  ({
    message,
    onForgetFrom,
  }: {
    message: Message;
    onForgetFrom?: (messageId: string) => void;
  }) => {
    return (
      <div className="flex max-w-[45rem] mx-auto cursor-default">
        <div
          className={cn(
            "flex flex-1 relative p-2",
            css`
              &:hover .option-menu {
                opacity: 1;
              }
            `
          )}
        >
          <div
            className={cn(
              "gap-3 flex flex-1 pr-[45px]",
              message.role === "user" ? "justify-end" : "justify-start"
            )}
          >
            {message.role !== "user" && (
              <div className="flex h-8 w-8 shrink-0 select-none items-center justify-center rounded-md bg-muted">
                <Bot className="h-4 w-4" />
              </div>
            )}
            <div
              className={cn(
                "flex flex-col gap-1",
                message.role === "user" ? "max-w-[70%]" : "max-w-[70%]"
              )}
            >
              <div>
                <div
                  className={cn(
                    "rounded-lg p-3 text-sm",
                    message.role === "user"
                      ? "bg-primary text-primary-foreground"
                      : "bg-muted"
                  )}
                >
                  <div
                    className={cn(
                      "prose prose-sm max-w-none dark:prose-invert whitespace-pre-wrap break-words",
                      css`
                        .language-md {
                          font-size: 13px;
                          white-space: pre-wrap;
                        }
                      `
                    )}
                  >
                    <ReactMarkdown
                      remarkPlugins={[remarkGfm]}
                      rehypePlugins={[rehypeHighlight, rehypeRaw]}
                    >
                      {message.content}
                    </ReactMarkdown>
                  </div>
                </div>
              </div>
              {message.file_attachments &&
                message.file_attachments.length > 0 && (
                  <div className="mt-2 flex flex-wrap gap-2">
                    {message.file_attachments.map((file) => (
                      <Badge
                        key={file.id}
                        variant="outline"
                        className="flex items-center gap-1 pr-1 cursor-pointer hover:bg-accent"
                        onClick={() => handleDownloadFile(file)}
                      >
                        <FileText className="h-3 w-3" />
                        <span className="max-w-[150px] truncate text-xs">
                          {file.original_name}
                        </span>
                        <span className="text-xs text-muted-foreground">
                          ({formatFileSize(file.file_size)})
                        </span>
                        <Download className="h-3 w-3 ml-1" />
                      </Badge>
                    ))}
                  </div>
                )}
              <div className="text-xs text-muted-foreground">
                {message.createdAt.toLocaleTimeString([], {
                  hour: "2-digit",
                  minute: "2-digit",
                  hour12: false,
                })}
              </div>
            </div>
            {message.role === "user" && (
              <div className="flex h-8 w-8 shrink-0 select-none items-center justify-center rounded-md bg-primary text-primary-foreground">
                <User className="h-4 w-4" />
              </div>
            )}
          </div>
          {onForgetFrom && (
            <div
              className={cn(
                "absolute top-2 right-3 option-menu",
                "opacity-0 hover:opacity-100 transition-opacity ml-2 "
              )}
            >
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="outline" size="sm" className="h-8 w-8 p-0">
                    <MoreVertical className="h-4 w-4" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem
                    onClick={() => onForgetFrom(message.id)}
                    className="text-destructive"
                  >
                    <Trash2 className="mr-2 h-4 w-4" />
                    Forget after this
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          )}
        </div>
      </div>
    );
  }
);

export function Messages({
  messages,
  isLoading,
  onForgetFrom,
  conversationId,
}: MessagesProps) {
  const virtuosoRef = useRef<VirtuosoHandle>(null);
  const [hasScrolledOnFirstLoad, setHasScrolledOnFirstLoad] = useState(false);
  const previousConversationId = useRef(conversationId);
  const [showNewMessageAlert, setShowNewMessageAlert] = useState(false);
  const [isAtBottom, setIsAtBottom] = useState(true);
  const previousMessageCount = useRef(messages.length);
  const scrollTimeoutRef = useRef<NodeJS.Timeout | null>(null);

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

  // Cleanup scroll timeout on unmount
  useEffect(() => {
    return () => {
      if (scrollTimeoutRef.current) {
        clearTimeout(scrollTimeoutRef.current);
      }
    };
  }, []);

  // Combine messages with loading indicator
  const allItems = useMemo(() => {
    const items = [...messages];
    if (isLoading) {
      items.push({
        id: "loading",
        content: "",
        role: "assistant" as const,
        createdAt: new Date(),
      });
    }
    return items;
  }, [messages, isLoading]);

  // Utility function to scroll with debouncing
  const smoothScrollToBottom = (delay = 0) => {
    if (scrollTimeoutRef.current) {
      clearTimeout(scrollTimeoutRef.current);
    }
    
    scrollTimeoutRef.current = setTimeout(() => {
      virtuosoRef.current?.scrollToIndex({
        index: allItems.length - 1,
        behavior: "smooth",
        align: "end",
      });
      scrollTimeoutRef.current = null;
    }, delay);
  };

  // Scroll to bottom on initial load only
  useEffect(() => {
    // Reset when conversation changes
    if (conversationId !== previousConversationId.current) {
      setHasScrolledOnFirstLoad(false);
      setShowNewMessageAlert(false);
      previousConversationId.current = conversationId;
      previousMessageCount.current = messages.length;
      if (scrollTimeoutRef.current) {
        clearTimeout(scrollTimeoutRef.current);
        scrollTimeoutRef.current = null;
      }
    }

    if (allItems.length > 0 && !hasScrolledOnFirstLoad) {
      // Initial scroll to bottom - use requestAnimationFrame for better timing
      requestAnimationFrame(() => {
        virtuosoRef.current?.scrollToIndex({
          index: allItems.length - 1,
          behavior: "auto",
          align: "end",
        });
        setHasScrolledOnFirstLoad(true);
      });
    }
  }, [allItems.length, conversationId]);

  // Handle new messages scrolling behavior
  useEffect(() => {
    if (
      hasScrolledOnFirstLoad &&
      messages.length > previousMessageCount.current
    ) {
      // New message received
      const lastMessage = messages[messages.length - 1];

      // Always scroll to bottom for user messages (when user sends a message)
      if (lastMessage.role === "user") {
        smoothScrollToBottom(50);
        setShowNewMessageAlert(false);
      }
      // For assistant messages
      else if (lastMessage.role === "assistant") {
        if (isAtBottom) {
          // Auto-scroll if user is at bottom
          smoothScrollToBottom(50);
        } else {
          // Show alert if user has scrolled up
          setShowNewMessageAlert(true);
        }
      }
    }
    previousMessageCount.current = messages.length;
  }, [messages.length, isAtBottom, hasScrolledOnFirstLoad, allItems.length]);

  const handleScrollToBottom = () => {
    smoothScrollToBottom(0);
    setShowNewMessageAlert(false);
  };

  return (
    <div className="flex flex-col h-full">
      {messages.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center text-center">
          <div className="flex h-20 w-20 items-center justify-center rounded-full bg-muted">
            <Bot className="h-10 w-10" />
          </div>
          <h2 className="mt-4 text-xl font-semibold">Welcome to Clay Studio</h2>
          <p className="mt-2 text-muted-foreground">
            I'm here to help you analyze your data. What would you like to
            explore?
          </p>
        </div>
      ) : (
        <div className="flex-1 relative">
          <Virtuoso
            ref={virtuosoRef}
            data={allItems}
            initialTopMostItemIndex={
              allItems.length > 0 ? allItems.length - 1 : 0
            }
            atBottomStateChange={(atBottom) => {
              setIsAtBottom(atBottom);
              if (atBottom) {
                setShowNewMessageAlert(false);
              }
            }}
            atBottomThreshold={100}
            overscan={50}
            itemContent={(index, item) => {
              // Add top padding for first item
              const topPadding =
                index === 0 ? <div style={{ height: "100px" }} /> : null;
              // Add bottom padding for last item
              const bottomPadding =
                index === allItems.length - 1 ? (
                  <div style={{ height: "200px" }} />
                ) : null;

              // Loading indicator
              if (item.id === "loading") {
                return (
                  <>
                    {topPadding}
                    <div className="flex max-w-[45rem] mx-auto cursor-default">
                      <div className="flex flex-1 relative p-2 rounded-lg">
                        <div className="flex gap-3 justify-start flex-1 pr-[45px]">
                          <div className="flex h-8 w-8 shrink-0 select-none items-center justify-center rounded-md bg-muted">
                            <Bot className="h-4 w-4" />
                          </div>
                          <div className="flex flex-row gap-2 items-center max-w-[70%]">
                            <div className="rounded-lg p-3 text-sm bg-muted whitespace-pre-wrap break-words">
                              <div className="flex items-center space-x-2">
                                <div className="flex items-center space-x-1">
                                  <div className="h-1 w-1 animate-bounce rounded-full bg-muted-foreground [animation-delay:-0.3s]"></div>
                                  <div className="h-1 w-1 animate-bounce rounded-full bg-muted-foreground [animation-delay:-0.15s]"></div>
                                  <div className="h-1 w-1 animate-bounce rounded-full bg-muted-foreground"></div>
                                </div>
                              </div>
                            </div>
                            <span className="text-muted-foreground text-sm animate-pulse font-medium">
                              {thinkingWords[thinkingWordIndex]}...
                            </span>
                          </div>
                        </div>
                      </div>
                    </div>
                    {bottomPadding}
                  </>
                );
              }

              // Regular message
              return (
                <>
                  {topPadding}
                  <MessageItem message={item} onForgetFrom={onForgetFrom} />
                  {bottomPadding}
                </>
              );
            }}
            followOutput={false}
            className="flex-1"
          />

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
        </div>
      )}
    </div>
  );
}
