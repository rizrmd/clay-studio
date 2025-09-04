import { memo, useMemo, lazy, Suspense } from "react";
import { FileAttachments } from "./file-attachments";
import { getToolNamesFromMessage } from "@/lib/types/chat";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { ToolCallIndicator } from "../tool/tool-call-indicator";
import { AskUser } from "../interaction/ask-user";
import {
  hasInteraction,
} from "../interaction/interaction-renderer";

// Lazy load heavy components for better code splitting
const InteractionRenderer = lazy(() => 
  import("../interaction/interaction-renderer").then(m => ({ default: m.InteractionRenderer }))
);
const MarkdownRenderer = lazy(() => 
  import("./markdown-renderer").then(m => ({ default: m.MarkdownRenderer }))
);
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { css } from "goober";
import {
  Copy,
  MoreVertical,
  Send,
  Trash2,
  User,
  Bot,
} from "lucide-react";

import { DisplayMessage, Message, ToolUsage } from "../types";

interface MessageListItemProps {
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
  onAskUserSubmit?: (response: string | string[]) => void;
  isStreaming?: boolean;
  isLoading?: boolean;
  allMessages?: DisplayMessage[];
  messageIndex?: number;
  onScroll?: () => void;
}

export const MessageListItem = memo(
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
    onAskUserSubmit,
    isStreaming,
    isLoading,
    allMessages,
    messageIndex,
    onScroll,
  }: MessageListItemProps) => {
    const isQueued = message.isQueued;
    const isEditing = message.isEditing;

    // Determine if content should use full width based on content characteristics
    const shouldUseFullWidth = useMemo(() => {
      if (message.role === "user") return false; // User messages always stay at 70%

      // Ensure content is always a string
      const content =
        typeof message.content === "string" ? message.content : "";
      const contentLength = content.length;

      // Check for indicators of large/complex content
      const hasCodeBlocks = content.includes("```");
      const hasLongLines = content.split("\n").some((line: string) => line.length > 80);
      const hasTables = content.includes("|") && content.includes("---");
      const hasMultipleTools = (message.tool_usages?.length || 0) > 3;
      const isLongContent = contentLength > 1500;
      const hasLists = (content.match(/^\s*[\d•\-\*]\s+/gm) || []).length > 5;
      const hasIndentedContent =
        content.includes("    ") || content.includes("\t");
      const hasMultipleAddresses =
        (content.match(/Address:|Alamat:/gi) || []).length > 2;

      // Use full width if any of these conditions are met
      return (
        hasCodeBlocks ||
        hasLongLines ||
        hasTables ||
        hasMultipleTools ||
        isLongContent ||
        hasLists ||
        hasIndentedContent ||
        hasMultipleAddresses
      );
    }, [message.content, message.role, message.tool_usages]);

    // Hide system messages that are interaction responses
    if (
      message.role === "system" &&
      message.content?.includes("User response to interaction")
    ) {
      return null;
    }

    // Find if there's a response to this interaction anywhere in the subsequent messages
    const findInteractionResponse = () => {
      if (
        !message.tool_usages?.some(
          (u: ToolUsage) => u.tool_name === "mcp__interaction__ask_user"
        )
      ) {
        return { hasResponse: false, response: undefined };
      }

      if (!allMessages || messageIndex === undefined) {
        return { hasResponse: false, response: undefined };
      }

      // Get the interaction ID from this message
      const interactionUsage = message.tool_usages.find(
        (u: ToolUsage) => u.tool_name === "mcp__interaction__ask_user"
      );
      if (!interactionUsage?.output) {
        return { hasResponse: false, response: undefined };
      }

      let interactionId = null;
      try {
        const output = Array.isArray(interactionUsage.output)
          ? interactionUsage.output[0]
          : interactionUsage.output;
        const text =
          typeof output === "object" && output.text ? output.text : output;
        const match = text.match(/"interaction_id":\s*"([^"]+)"/);
        interactionId = match?.[1];
      } catch (e) {
        return { hasResponse: false, response: undefined };
      }

      if (!interactionId) {
        return { hasResponse: false, response: undefined };
      }

      // Look for a subsequent assistant message containing response to this interaction
      for (let i = messageIndex + 1; i < allMessages.length; i++) {
        const futureMessage = allMessages[i];

        if (
          futureMessage.role === "system" &&
          futureMessage.content?.includes(
            `User response to interaction ${interactionId}:`
          )
        ) {
          const responseMatch = futureMessage.content.match(
            /User response to interaction [^:]+:\s*\n?"([^"]+)"/
          );
          return { hasResponse: true, response: responseMatch?.[1] };
        }
      }

      return { hasResponse: false, response: undefined };
    };

    const { hasResponse: hasUserResponse, response: extractedResponse } =
      findInteractionResponse();

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
                message.role === "user"
                  ? cn("max-w-[70%]")
                  : shouldUseFullWidth
                  ? css`
                      width: calc(100% - 30px) !important;
                      max-width: 570px;
                    `
                  : "max-w-[70%]"
              )}
            >
              {/* Only show message box if there's actual content or it's a user message */}
              {(message.content?.trim() || message.role === "user") && (
                <div
                  className={cn(
                    "max-w-[350px] lg:max-w-[450px] rounded-lg px-0 py-3 text-sm overflow-hidden",
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
                      <Suspense fallback={<div className="animate-pulse bg-gray-100 h-4 rounded w-full" />}>
                        <MarkdownRenderer
                          content={message.content}
                        />
                      </Suspense>
                    </div>
                  )}
                </div>
              )}
              <FileAttachments attachments={message.file_attachments || []} />
              {/* Show TodoList if this message has TodoWrite data AND */}
              {/* {message.todoWrite && (
                <div className="mt-3">
                  <TodoList todos={message.todoWrite.todos} />
                </div>
              )} */}
              {/* Interaction Tool Rendering */}
              {(() => {
                const interactionUsages = message.tool_usages?.filter(
                  (usage: ToolUsage) =>
                    usage.tool_name === "mcp__interaction__ask_user" ||
                    usage.tool_name === "mcp__interaction__show_table" ||
                    usage.tool_name === "mcp__interaction__show_chart"
                );
                return interactionUsages?.some((usage: ToolUsage) =>
                  hasInteraction(usage.output)
                );
              })() && (
                <div
                  className={cn(
                    (message.content?.trim() || message.role === "user") &&
                      "mt-2",
                    "space-y-2"
                  )}
                >
                  {(message.tool_usages || [])
                    .filter(
                      (usage: ToolUsage) =>
                        usage.tool_name === "mcp__interaction__ask_user" ||
                        usage.tool_name === "mcp__interaction__show_table" ||
                        usage.tool_name === "mcp__interaction__show_chart"
                    )
                    .map((usage: ToolUsage, index: number) => (
                      <Suspense key={`${usage.id}-${index}`} fallback={<div className="h-12 bg-gray-100 animate-pulse rounded" />}>
                        <InteractionRenderer
                          toolOutput={usage.output}
                          onAskUserSubmit={onAskUserSubmit}
                          isDisabled={isStreaming || isLoading}
                          hasResponse={hasUserResponse}
                          selectedResponse={extractedResponse}
                          onScroll={onScroll}
                        />
                      </Suspense>
                    ))}
                </div>
              )}

              {/* Legacy AskUser component for backward compatibility */}
              {message.ask_user &&
                message.role === "assistant" &&
                !message.tool_usages?.some(
                  (usage: ToolUsage) =>
                    usage.tool_name === "mcp__interaction__ask_user" &&
                    hasInteraction(usage.output)
                ) && (
                  <div className="mt-3">
                    <AskUser
                      promptType={message.ask_user.prompt_type}
                      title={message.ask_user.title}
                      options={message.ask_user.options}
                      inputType={message.ask_user.input_type}
                      placeholder={message.ask_user.placeholder}
                      toolUseId={message.ask_user.tool_use_id}
                      onSubmit={(response) => {
                        if (onAskUserSubmit) {
                          onAskUserSubmit(response);
                        }
                      }}
                      isDisabled={isStreaming || isLoading}
                      hasResponse={hasUserResponse}
                      selectedResponse={extractedResponse}
                      onScroll={onScroll}
                    />
                  </div>
                )}
               {/* Show tools used for messages (both streaming and completed) */}
               {getToolNamesFromMessage(message as any).length > 0 &&
                 !message.isQueued && (
                   <div className=" flex items-center gap-2">
                     <div className="border px-1 py-[2px] rounded-sm">
                       <ToolCallIndicator
                         tools={getToolNamesFromMessage(message as any)}
                         variant="compact"
                         isCompleted={!isStreaming} // Show as in-progress while streaming
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
                       {message.role === "assistant" && message.processing_time_ms && (
                         <span className="ml-2 text-muted-foreground">
                           • {Math.round(message.processing_time_ms / 1000)}s
                         </span>
                       )}
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
                  <>
                    {(message.createdAt instanceof Date
                      ? message.createdAt
                      : new Date(message.createdAt)
                    ).toLocaleTimeString([], {
                      hour: "2-digit",
                      minute: "2-digit",
                      hour12: false,
                    })}
                    {message.role === "assistant" && message.processing_time_ms && (
                      <span className="ml-2 text-muted-foreground">
                        • {Math.round(message.processing_time_ms / 1000)}s
                      </span>
                    )}
                  </>
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
                  "md:opacity-0 hover:opacity-100 transition-opacity ml-2 "
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
