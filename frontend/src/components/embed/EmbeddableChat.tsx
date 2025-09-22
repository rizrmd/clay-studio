import React, { useEffect, useState, useRef } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Send, Bot, User } from "lucide-react";
import ReactMarkdown from "react-markdown";

interface Message {
  id: string;
  content: string;
  role: "user" | "assistant";
  timestamp: string;
}

interface EmbeddableChatProps {
  shareToken: string;
  theme?: "light" | "dark" | "auto";
  readOnly?: boolean;
  className?: string;
  onMessage?: (message: Message) => void;
  onError?: (error: string) => void;
}

export function EmbeddableChat({
  shareToken,
  theme = "light",
  readOnly = false,
  className = "",
  onMessage,
  onError,
}: EmbeddableChatProps) {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [_sessionToken, setSessionToken] = useState<string | null>(null);
  const [shareData, setShareData] = useState<any>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  // Load shared data on mount
  useEffect(() => {
    const loadShareData = async () => {
      try {
        const response = await fetch(`/api/shares/${shareToken}/data`);
        if (!response.ok) {
          throw new Error("Failed to load shared data");
        }
        const data = await response.json();
        setShareData(data);

        // If not read-only, create a session
        if (!readOnly) {
          const sessionResponse = await fetch(`/api/shares/${shareToken}/session`, {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
            },
          });
          if (sessionResponse.ok) {
            const sessionData = await sessionResponse.json();
            setSessionToken(sessionData.session_token);
          }
        }

        // Load initial conversations if available
        if (data.conversations && data.conversations.length > 0) {
          // const _firstConv = data.conversations[0];
          // This would normally load messages from the conversation
          // For now, we'll just show a welcome message
          setMessages([
            {
              id: "welcome",
              content: "Hello! This is a shared conversation. How can I help you?",
              role: "assistant",
              timestamp: new Date().toISOString(),
            },
          ]);
        }
      } catch (error) {
        console.error("Failed to load share data:", error);
        onError?.("Failed to load chat data");
      }
    };

    if (shareToken) {
      loadShareData();
    }
  }, [shareToken, readOnly, onError]);

  const handleSendMessage = async () => {
    if (!input.trim() || isLoading || readOnly) return;

    const userMessage: Message = {
      id: Date.now().toString(),
      content: input,
      role: "user",
      timestamp: new Date().toISOString(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setInput("");
    setIsLoading(true);
    onMessage?.(userMessage);

    try {
      // In a real implementation, this would send the message to the backend
      // For now, we'll simulate an AI response
      await new Promise(resolve => setTimeout(resolve, 1000));

      const assistantMessage: Message = {
        id: (Date.now() + 1).toString(),
        content: "Thank you for your message! This is a demo response from the embedded chat.",
        role: "assistant",
        timestamp: new Date().toISOString(),
      };

      setMessages((prev) => [...prev, assistantMessage]);
      onMessage?.(assistantMessage);
    } catch (error) {
      console.error("Failed to send message:", error);
      onError?.("Failed to send message");
    } finally {
      setIsLoading(false);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  };

  const themeClass = theme === "dark" ? "dark" : "";

  return (
    <div className={`${themeClass} ${className}`}>
      <div className="flex flex-col h-full bg-background border border-border rounded-lg overflow-hidden">
        {/* Header */}
        <div className="border-b border-border px-4 py-3 bg-muted/10">
          <h3 className="text-sm font-medium text-foreground">
            {shareData?.project_name || "Shared Chat"}
          </h3>
          {shareData?.share?.settings?.description && (
            <p className="text-xs text-muted-foreground mt-1">
              {shareData.share.settings.description}
            </p>
          )}
        </div>

        {/* Messages */}
        <ScrollArea className="flex-1 p-4">
          <div className="space-y-4">
            {messages.map((message) => (
              <div key={message.id} className="flex gap-3">
                <Avatar className="h-8 w-8 mt-1">
                  <AvatarFallback className="text-xs">
                    {message.role === "user" ? (
                      <User className="h-4 w-4" />
                    ) : (
                      <Bot className="h-4 w-4" />
                    )}
                  </AvatarFallback>
                </Avatar>
                <div className="flex-1 space-y-1">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium">
                      {message.role === "user" ? "You" : "Assistant"}
                    </span>
                    <span className="text-xs text-muted-foreground">
                      {new Date(message.timestamp).toLocaleTimeString()}
                    </span>
                  </div>
                  <div className="text-sm text-foreground prose prose-sm max-w-none">
                    <ReactMarkdown>{message.content}</ReactMarkdown>
                  </div>
                </div>
              </div>
            ))}
            {isLoading && (
              <div className="flex gap-3">
                <Avatar className="h-8 w-8 mt-1">
                  <AvatarFallback className="text-xs">
                    <Bot className="h-4 w-4" />
                  </AvatarFallback>
                </Avatar>
                <div className="flex-1">
                  <div className="text-sm font-medium">Assistant</div>
                  <div className="text-sm text-muted-foreground">
                    <div className="flex items-center gap-1">
                      <div className="animate-pulse">●</div>
                      <div className="animate-pulse animation-delay-200">●</div>
                      <div className="animate-pulse animation-delay-400">●</div>
                      <span className="ml-2">Thinking...</span>
                    </div>
                  </div>
                </div>
              </div>
            )}
            <div ref={messagesEndRef} />
          </div>
        </ScrollArea>

        {/* Input */}
        {!readOnly && (
          <div className="border-t border-border p-4">
            <div className="flex gap-2">
              <Input
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyPress={handleKeyPress}
                placeholder="Type a message..."
                disabled={isLoading}
                className="flex-1"
              />
              <Button
                onClick={handleSendMessage}
                disabled={!input.trim() || isLoading}
                size="sm"
              >
                <Send className="h-4 w-4" />
              </Button>
            </div>
          </div>
        )}

        {/* Footer */}
        {shareData?.share?.settings?.show_branding !== false && (
          <div className="border-t border-border px-4 py-2 text-center">
            <span className="text-xs text-muted-foreground">
              Powered by{" "}
              <a
                href="https://clay.studio"
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary hover:underline"
              >
                Clay Studio
              </a>
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

// Export a version that can be used as a web component
export function initEmbeddableChat(container: HTMLElement, _props: EmbeddableChatProps) {
  const root = document.createElement('div');
  root.style.width = '100%';
  root.style.height = '100%';
  container.appendChild(root);

  // This would normally use React.render, but for simplicity we'll return a placeholder
  return {
    destroy: () => {
      container.removeChild(root);
    },
    updateProps: (newProps: Partial<EmbeddableChatProps>) => {
      // Update the component props
      console.log('Updating props:', newProps);
    }
  };
}