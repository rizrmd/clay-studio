import { Badge } from "@/components/ui/badge";
import { getToolNamesFromMessage, type AskUserData } from "@/types/chat";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { ToolCallIndicator } from "./tool-call-indicator";
import { TodoList } from "./todo-list";
import { AskUser } from "./ask-user";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { API_BASE_URL } from "@/lib/url";
import { cn } from "@/lib/utils";
import { css } from "goober";
import "github-markdown-css/github-markdown-light.css";
import {
  ArrowDown,
  Bot,
  Clock,
  Copy,
  Download,
  FileText,
  MoreVertical,
  Send,
  Square,
  Trash2,
  User,
} from "lucide-react";
import { memo, useEffect, useMemo, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import { Virtuoso, VirtuosoHandle } from "react-virtuoso";
import rehypeHighlight from "rehype-highlight";
import rehypeRaw from "rehype-raw";
import remarkGfm from "remark-gfm";

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
  createdAt: string | Date;
  file_attachments?: FileAttachment[];
  clay_tools_used?: string[];
  tool_usages?: any[]; // Added for compatibility
  ask_user?: AskUserData;
}

interface QueuedMessage {
  id: string;
  content: string;
  files: File[];
  timestamp: Date;
}

interface MessagesProps {
  messages: Message[];
  isLoading?: boolean;
  onForgetFrom?: (messageId: string) => void;
  conversationId?: string; // Add conversationId to detect conversation switches
  messageQueue?: QueuedMessage[];
  onEditQueued?: (messageId: string, newContent: string) => void;
  onCancelQueued?: (messageId: string) => void;
  isProcessingQueue?: boolean;
  isStreaming?: boolean;
  canStop?: boolean;
  onStop?: () => void;
  activeTools?: string[]; // Active tools being used
  onResendMessage?: (message: Message) => void; // Add resend callback
  onNewChatFromHere?: (messageId: string) => void; // Add new chat from here callback
  onAskUserSubmit?: (response: string | string[]) => void; // Add ask user submission callback
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

// Extended message type for display
interface DisplayMessage extends Message {
  isQueued?: boolean;
  queuePosition?: number;
  isEditing?: boolean;
}

// Memoized individual message component
const MessageItem = memo(
  ({
    message,
    onForgetFrom,
    onStartEdit,
    onSaveEdit,
    onCancelEdit,
    onCancelQueued,
    editingContent,
    setEditingContent,
    onResendMessage,
    isLastUserMessage,
    onNewChatFromHere,
    latestTodoWrite,
    onAskUserSubmit,
    isStreaming,
    isLoading,
  }: {
    message: DisplayMessage;
    onForgetFrom?: (messageId: string) => void;
    onStartEdit?: (messageId: string) => void;
    onSaveEdit?: () => void;
    onCancelEdit?: () => void;
    onCancelQueued?: (messageId: string) => void;
    editingContent?: string;
    setEditingContent?: (content: string) => void;
    onResendMessage?: (message: Message) => void;
    isLastUserMessage?: boolean;
    onNewChatFromHere?: (messageId: string) => void;
    latestTodoWrite?: { messageId: string; todos: any[] } | null;
    onAskUserSubmit?: (response: string | string[]) => void;
    isStreaming?: boolean;
    isLoading?: boolean;
  }) => {
    const isQueued = message.isQueued;
    const isEditing = message.isEditing;
    return (
      <div
        className={cn(
          "flex max-w-[45rem] mx-auto cursor-default",
          isQueued && "opacity-70"
        )}
      >
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
              "gap-3 flex flex-1 pr-[45px] min-w-0",
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
                "flex flex-col gap-1 min-w-0",
                message.role === "user" ? "max-w-[70%]" : "max-w-[70%]"
              )}
            >
              {/* Queue indicator badge */}
              {isQueued && (
                <div className="flex items-center gap-2 mb-1">
                  <Badge variant="outline" className="text-xs">
                    <Clock className="h-3 w-3 mr-1" />
                    Queued #{message.queuePosition}
                  </Badge>
                </div>
              )}
              <div className="min-w-0">
                <div
                  className={cn(
                    "rounded-lg px-0 py-3 text-sm overflow-hidden",
                    message.role === "user" && !isQueued
                      ? "bg-primary text-background"
                      : message.role === "user" && isQueued
                      ? "bg-primary/20 text-foreground border-2 border-dashed border-primary/30"
                      : "bg-muted"
                  )}
                >
                  {isEditing ? (
                    <div className="space-y-2">
                      <Textarea
                        value={editingContent || message.content}
                        onChange={(e) => setEditingContent?.(e.target.value)}
                        className="min-h-[80px] text-sm bg-white border-0 min-w-[400px]"
                        autoFocus
                        onKeyDown={(e) => {
                          if (e.key === "Escape") {
                            onCancelEdit?.();
                          } else if (
                            e.key === "Enter" &&
                            (e.metaKey || e.ctrlKey)
                          ) {
                            onSaveEdit?.();
                          }
                        }}
                      />
                      <div className="flex gap-2">
                        <Button size="sm" onClick={onSaveEdit}>
                          Save
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={onCancelEdit}
                        >
                          Cancel
                        </Button>
                      </div>
                    </div>
                  ) : (
                    <div
                      className={cn(
                        "overflow-hidden break-words",
                        message.role === "user" && !isQueued
                          ? "markdown-body-dark"
                          : "markdown-body",
                        css`
                          * {
                            font-size: 0.85rem !important;
                            line-height: 1.6;
                          }

                          &.markdown-body-dark pre {
                            background-color: rgba(0, 0, 0, 0.2) !important;
                          }

                          &.markdown-body-dark code {
                            background-color: rgba(0, 0, 0, 0.2) !important;
                            color: inherit !important;
                          }

                          &.markdown-body-light {
                            color: inherit !important;
                          }

                          * {
                            max-width: 100%;
                          }

                          &.markdown-body,
                          &.markdown-body-dark {
                            & > * {
                              margin-left: 10px;
                              margin-right: 10px;
                            }

                            color: inherit !important;
                            h1 {
                              font-size: 1.2rem !important;
                            }

                            h2 {
                              font-size: 1.1rem !important;
                            }

                            h3 {
                              font-size: 1.05rem !important;
                            }

                            h4 {
                              font-size: 1rem !important;
                            }

                            h5 {
                              font-size: 0.95rem !important;
                            }
                            h1,
                            h2,
                            h3,
                            h4,
                            h5 {
                              margin-bottom: 5px;
                              border-bottom: 1px solid #d1d9e0b3;
                            }

                            hr {
                              margin: -5px 0px !important;
                              height: 10px;
                              background: white;
                              position: relative;
                            }

                            pre {
                              max-width: 450px;
                              overflow-x: auto;
                              word-break: break-word;
                              overflow-wrap: anywhere;
                            }

                            code {
                              word-break: break-all;
                              overflow-wrap: anywhere;
                            }

                            table {
                              display: block;
                              overflow-x: auto;
                              max-width: 100%;
                              width: max-content;
                              max-width: calc(100% - 20px);
                            }

                            a {
                              word-break: break-all;
                              overflow-wrap: anywhere;
                            }
                            ol > li {
                              list-style-type: square;
                            }
                            ul > li {
                              list-style-type: decimal;
                            }
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
                  )}
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
              {/* Show TodoList only for the latest TodoWrite message */}
              {latestTodoWrite && latestTodoWrite.messageId === message.id && (
                <div className="mt-3">
                  <TodoList todos={latestTodoWrite.todos} />
                </div>
              )}
              {/* Show AskUser component if present */}
              {message.ask_user && message.role === "assistant" && (
                <div className="mt-3">
                  <AskUser
                    promptType={message.ask_user.prompt_type}
                    title={message.ask_user.title}
                    options={message.ask_user.options}
                    inputType={message.ask_user.input_type}
                    placeholder={message.ask_user.placeholder}
                    toolUseId={message.ask_user.tool_use_id}
                    onSubmit={(response) => {
                      // Handle the submission
                      if (onAskUserSubmit) {
                        onAskUserSubmit(response);
                      }
                    }}
                    isDisabled={isStreaming || isLoading}
                  />
                </div>
              )}
              {/* Show tools used for completed messages */}
              {getToolNamesFromMessage(message as any).length > 0 &&
                !message.isQueued && (
                  <div className=" flex items-center gap-2">
                    <div className="border px-1 py-[2px] rounded-sm">
                      <ToolCallIndicator
                        tools={getToolNamesFromMessage(message as any)}
                        variant="compact"
                        isCompleted={true}
                        messageId={message.id}
                        toolUsages={message.tool_usages}
                      />
                    </div>
                    <div className="text-xs">
                      {(message.createdAt instanceof Date
                        ? message.createdAt
                        : new Date(message.createdAt)
                      ).toLocaleTimeString([], {
                        hour: "2-digit",
                        minute: "2-digit",
                        hour12: false,
                      })}
                    </div>
                  </div>
                )}
              <div className="text-xs text-muted-foreground">
                {isQueued && !isEditing ? (
                  <div className="flex items-center gap-2">
                    <span>Waiting to send...</span>
                    {onStartEdit && (
                      <button
                        onClick={() => onStartEdit(message.id)}
                        className="text-primary hover:underline"
                      >
                        Edit
                      </button>
                    )}
                    {onCancelQueued && (
                      <button
                        onClick={() => onCancelQueued(message.id)}
                        className="text-destructive hover:underline"
                      >
                        Cancel
                      </button>
                    )}
                  </div>
                ) : !isQueued &&
                  !(getToolNamesFromMessage(message as any).length > 0) ? (
                  (message.createdAt instanceof Date
                    ? message.createdAt
                    : new Date(message.createdAt)
                  ).toLocaleTimeString([], {
                    hour: "2-digit",
                    minute: "2-digit",
                    hour12: false,
                  })
                ) : null}
              </div>
            </div>
            {message.role === "user" && (
              <div
                className={cn(
                  "flex h-8 w-8 shrink-0 select-none items-center justify-center rounded-md",
                  isQueued
                    ? "bg-primary/20 text-primary border-2 border-dashed border-primary/30"
                    : "bg-primary text-primary-foreground"
                )}
              >
                <User className="h-4 w-4" />
              </div>
            )}
          </div>
          {(onForgetFrom ||
            onNewChatFromHere ||
            (onResendMessage &&
              isLastUserMessage &&
              message.role === "user")) &&
            !isQueued && (
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
                    {onResendMessage &&
                      isLastUserMessage &&
                      message.role === "user" && (
                        <DropdownMenuItem
                          onClick={() => onResendMessage(message)}
                        >
                          <Send className="mr-2 h-4 w-4" />
                          Resend
                        </DropdownMenuItem>
                      )}
                    {onForgetFrom && (
                      <DropdownMenuItem
                        onClick={() => onForgetFrom(message.id)}
                        className="text-destructive"
                      >
                        <Trash2 className="mr-2 h-4 w-4" />
                        Forget after this
                      </DropdownMenuItem>
                    )}
                    {onNewChatFromHere && (
                      <DropdownMenuItem
                        onClick={() => onNewChatFromHere(message.id)}
                      >
                        <Copy className="mr-2 h-4 w-4" />
                        New Chat From Here
                      </DropdownMenuItem>
                    )}
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
    // Cancel any pending scroll operations
    if (scrollTimeoutRef.current) {
      clearTimeout(scrollTimeoutRef.current);
      scrollTimeoutRef.current = null;
    }

    virtuosoRef.current?.scrollToIndex({
      index: allItems.length - 1,
      behavior,
      align: "end",
    });
  };

  // Initialize Virtuoso after a small delay to prevent initial flicker
  useEffect(() => {
    if (!isInitialized && messages.length > 0) {
      // Use requestAnimationFrame to ensure DOM is ready
      requestAnimationFrame(() => {
        setIsInitialized(true);
      });
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
      // Re-initialize after conversation change
      requestAnimationFrame(() => {
        setIsInitialized(true);
      });
    }
  }, [allItems.length, conversationId]);

  // Handle new messages scrolling behavior
  useEffect(() => {
    if (messages.length > previousMessageCount.current) {
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
    }
    previousMessageCount.current = messages.length;
  }, [messages.length, isAtBottom]);

  const handleScrollToBottom = () => {
    scrollToBottom("smooth");
    setShowNewMessageAlert(false);
  };

  return (
    <div className={cn("flex flex-col h-full")}>
      {messages.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-start md:justify-center text-center pt-12 md:pt-0">
          <div className="flex h-20 w-20 items-center justify-center rounded-full bg-muted">
            <Bot className="h-10 w-10" />
          </div>
          <h2 className="mt-4 text-xl font-semibold">Welcome to Clay Studio</h2>
          <p className="mt-2 text-muted-foreground px-4">
            I'm here to help you analyze your data. What would you like to
            explore?
          </p>
        </div>
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
              {!isInitialized && messages.length > 0 && (
                <div className="flex-1 overflow-hidden">
                  <div className="flex flex-col gap-4 p-4 animate-pulse">
                    {messages.slice(-3).map((_, idx) => (
                      <div
                        key={idx}
                        className="flex max-w-[45rem] mx-auto w-full"
                      >
                        <div className="flex gap-3 w-full">
                          <div className="h-8 w-8 rounded-md bg-muted" />
                          <div className="flex-1 space-y-2">
                            <div className="h-4 bg-muted rounded w-3/4" />
                            <div className="h-4 bg-muted rounded w-1/2" />
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
              {previousConversationId.current === conversationId &&
                isInitialized && (
                  <Virtuoso
                    ref={virtuosoRef}
                    data={allItems}
                    initialTopMostItemIndex={
                      allItems.length > 0
                        ? Math.max(0, allItems.length - 1)
                        : undefined
                    }
                    defaultItemHeight={100}
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
                          <div className="flex max-w-[45rem] mx-auto p-2">
                            <div className="h-20 w-full bg-muted/30 rounded-lg animate-pulse" />
                          </div>
                        );
                      },
                    }}
                    itemContent={(index, item) => {
                      // Add top padding for first item
                      const topPadding =
                        index === 0 ? (
                          <div style={{ height: "100px" }} />
                        ) : null;
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
                                  <div className="flex flex-col gap-2 max-w-[70%]">
                                    <div className="rounded-lg p-3 text-sm bg-muted">
                                      <div className="flex items-center space-x-2">
                                        {activeTools.length === 0 ? (
                                          <div className="flex items-center space-x-2 flex-1">
                                            <div className="flex items-center space-x-1">
                                              <div className="h-1 w-1 animate-bounce rounded-full bg-muted-foreground [animation-delay:-0.3s]"></div>
                                              <div className="h-1 w-1 animate-bounce rounded-full bg-muted-foreground [animation-delay:-0.15s]"></div>
                                              <div className="h-1 w-1 animate-bounce rounded-full bg-muted-foreground"></div>
                                            </div>

                                            <span className="text-muted-foreground text-sm animate-pulse font-medium">
                                              {thinkingWords[thinkingWordIndex]}
                                              ...
                                            </span>
                                          </div>
                                        ) : (
                                          <>
                                            <span
                                              className={cn(
                                                "text-muted-foreground text-sm animate-pulse font-medium",
                                                (!canStop || !onStop) &&
                                                  "flex items-center justify-center flex-1"
                                              )}
                                            >
                                              {thinkingWords[thinkingWordIndex]}{" "}
                                              {activeTools.length > 1
                                                ? "tools"
                                                : "tool"}
                                            </span>
                                          </>
                                        )}
                                        {canStop && onStop && (
                                          <div className="pl-6">
                                            <Button
                                              variant="outline"
                                              size="sm"
                                              onClick={onStop}
                                              className="h-7 px-2"
                                            >
                                              <Square className="h-3 w-3 mr-1" />
                                              Stop
                                            </Button>
                                          </div>
                                        )}
                                      </div>
                                    </div>
                                    {/* Show active tools if any are being used */}
                                    {activeTools.length > 0 && (
                                      <ToolCallIndicator
                                        key={`active-tools-${activeTools.join(
                                          "-"
                                        )}`}
                                        tools={activeTools}
                                        variant="full"
                                        isCompleted={false}
                                        className="ml-3"
                                      />
                                    )}
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
                          <MessageItem
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
                          />
                          {bottomPadding}
                        </>
                      );
                    }}
                    followOutput={false}
                    className="flex-1"
                    style={{
                      height: "100%",
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
