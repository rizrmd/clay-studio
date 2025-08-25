import { useEffect, useRef, memo, useMemo, useState } from "react";
import {
  Bot,
  User,
  MoreVertical,
  Trash2,
  FileText,
  Download,
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
      <div
        className={cn(
          "flex gap-3 max-w-2xl mb-6 relative group",
          message.role === "user" ? "ml-auto flex-row-reverse" : "mr-auto"
        )}
      >
        <div className="flex h-8 w-8 shrink-0 select-none items-center justify-center rounded-md bg-muted">
          {message.role === "user" ? (
            <User className="h-4 w-4" />
          ) : (
            <Bot className="h-4 w-4" />
          )}
        </div>
        <div className="flex flex-col gap-1 flex-1">
          <div>
            <div
              className={cn(
                "rounded-lg p-3 text-sm",
                message.role === "user"
                  ? "bg-primary text-primary-foreground"
                  : "bg-muted"
              )}
            >
              <div className="prose prose-sm max-w-none dark:prose-invert">
                <ReactMarkdown
                  remarkPlugins={[remarkGfm]}
                  rehypePlugins={[rehypeHighlight, rehypeRaw]}
                >
                  {message.content}
                </ReactMarkdown>
              </div>
            </div>
          </div>
          {message.file_attachments && message.file_attachments.length > 0 && (
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
        {onForgetFrom && (
          <div className="absolute -right-10 top-0 opacity-0 group-hover:opacity-100 transition-opacity">
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
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
    );
  }
);

export function Messages({ messages, isLoading, onForgetFrom }: MessagesProps) {
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const [_visibleRange, _setVisibleRange] = useState({
    start: 0,
    end: messages.length,
  });
  const [hasScrolledOnFirstLoad, setHasScrolledOnFirstLoad] = useState(false);

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

  const [thinkingWordIndex, setThinkingWordIndex] = useState(
    () => Math.floor(Math.random() * thinkingWords.length)
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

  // For performance with large message counts, only render recent messages
  const visibleMessages = useMemo(() => {
    if (messages.length <= 20) {
      return messages; // Render all messages if count is reasonable
    }

    // For large lists, show last 15 messages + some buffer for scrolling
    const startIndex = Math.max(0, messages.length - 20);
    return messages.slice(startIndex);
  }, [messages]);

  useEffect(() => {
    if (messages.length > 0 && !hasScrolledOnFirstLoad) {
      // Instant scroll on first load
      messagesEndRef.current?.scrollIntoView({ behavior: "instant" });
      setHasScrolledOnFirstLoad(true);
    } else if (hasScrolledOnFirstLoad) {
      // Smooth scroll for subsequent updates
      messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages, isLoading, hasScrolledOnFirstLoad]);

  return (
    <div className="flex flex-col flex-1">
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
        <div className="flex flex-col gap-6 p-4">
          {/* Show indicator if messages are truncated */}
          {messages.length > 20 && (
            <div className="text-center py-2 text-sm text-muted-foreground border-b border-muted">
              Showing recent {visibleMessages.length} of {messages.length}{" "}
              messages
            </div>
          )}

          {visibleMessages.map((message) => (
            <MessageItem
              key={message.id}
              message={message}
              onForgetFrom={onForgetFrom}
            />
          ))}

          {isLoading && (
            <div className="flex gap-3 max-w-2xl mr-auto">
              <div className="flex h-8 w-8 shrink-0 select-none items-center justify-center rounded-md bg-muted">
                <Bot className="h-4 w-4" />
              </div>
              <div className="flex flex-row gap-2 flex-1 items-center">
                <div className="rounded-lg p-3 text-sm bg-muted">
                  <div className="flex items-center space-x-2">
                    <div className="flex items-center space-x-1">
                      <div className="h-1 w-1 animate-bounce rounded-full bg-muted-foreground [animation-delay:-0.3s]"></div>
                      <div className="h-1 w-1 animate-bounce rounded-full bg-muted-foreground [animation-delay:-0.15s]"></div>
                      <div className="h-1 w-1 animate-bounce rounded-full bg-muted-foreground"></div>
                    </div>
                  </div>
                </div>
                <span className="text-muted-foreground text-sm animate-pulse font-medium">
                  {thinkingWords[thinkingWordIndex]}
                </span>
              </div>
            </div>
          )}
        </div>
      )}
      <div ref={messagesEndRef} />
    </div>
  );
}
