import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { MoreVertical, Copy, Trash2, Send, ArrowDown } from "lucide-react";
import { Message } from "../types";

interface MessageActionsProps {
  message: Message;
  onCopy: () => void;
  onForgetFrom?: (messageId: string) => void;
  onResendMessage?: (message: Message) => void;
  onNewChatFromHere?: (messageId: string) => void;
  isLastUserMessage?: boolean;
}

export function MessageActions({
  message,
  onCopy,
  onForgetFrom,
  onResendMessage,
  onNewChatFromHere,
  isLastUserMessage,
}: MessageActionsProps) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="sm" className="h-6 w-6 p-0">
          <MoreVertical className="h-3 w-3" />
          <span className="sr-only">Open menu</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onClick={onCopy}>
          <Copy className="mr-2 h-4 w-4" />
          Copy
        </DropdownMenuItem>
        {message.role === "user" && isLastUserMessage && onResendMessage && (
          <DropdownMenuItem onClick={() => onResendMessage(message)}>
            <Send className="mr-2 h-4 w-4" />
            Resend Message
          </DropdownMenuItem>
        )}
        {onNewChatFromHere && (
          <DropdownMenuItem onClick={() => onNewChatFromHere(message.id)}>
            <ArrowDown className="mr-2 h-4 w-4" />
            New Chat from Here
          </DropdownMenuItem>
        )}
        {onForgetFrom && (
          <DropdownMenuItem
            className="text-destructive"
            onClick={() => onForgetFrom(message.id)}
          >
            <Trash2 className="mr-2 h-4 w-4" />
            Forget from here
          </DropdownMenuItem>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}