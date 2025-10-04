import { useState } from "react";
import { Lock, Globe, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { toast } from "sonner";
import { toggleConversationVisibility } from "@/lib/api/conversations";
import { chatStore } from "@/lib/store/chat/chat-store";

interface ConversationVisibilityToggleProps {
  conversationId: string;
  visibility?: "private" | "public";
  size?: "sm" | "default" | "lg";
  showLabel?: boolean;
}

export function ConversationVisibilityToggle({
  conversationId,
  visibility = "private",
  size = "sm",
  showLabel = false,
}: ConversationVisibilityToggleProps) {
  const [loading, setLoading] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);

  const isPrivate = visibility === "private";
  const Icon = isPrivate ? Lock : Globe;

  const handleToggle = async () => {
    setLoading(true);
    try {
      const result = await toggleConversationVisibility(conversationId);

      // Update local store
      const conversation = chatStore.map[conversationId];
      if (conversation) {
        conversation.visibility = result.visibility;
      }

      toast.success(
        result.visibility === "public"
          ? "Conversation is now public"
          : "Conversation is now private"
      );
      setConfirmOpen(false);
    } catch (error: any) {
      const message = error?.message || "Failed to change visibility";
      toast.error(message);
      console.error(error);
    } finally {
      setLoading(false);
    }
  };

  const tooltipText = isPrivate
    ? "Private - Only you can see this conversation"
    : "Public - All project members can see this conversation";

  const buttonContent = (
    <>
      {loading ? (
        <Loader2 className="h-4 w-4 animate-spin" />
      ) : (
        <Icon className="h-4 w-4" />
      )}
      {showLabel && (
        <span className="ml-2">{isPrivate ? "Private" : "Public"}</span>
      )}
    </>
  );

  return (
    <>
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size={size}
              onClick={() => setConfirmOpen(true)}
              disabled={loading}
              className="gap-2"
            >
              {buttonContent}
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <p>{tooltipText}</p>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>

      {/* Confirmation Dialog */}
      <AlertDialog open={confirmOpen} onOpenChange={setConfirmOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              Make conversation {isPrivate ? "public" : "private"}?
            </AlertDialogTitle>
            <AlertDialogDescription>
              {isPrivate ? (
                <>
                  This conversation will be visible to all project members.
                  Anyone in the project will be able to see all messages and
                  continue the conversation.
                </>
              ) : (
                <>
                  This conversation will be private. Only you will be able to
                  see and access it. Other project members will no longer have
                  access.
                </>
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={handleToggle} disabled={loading}>
              {loading ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  Changing...
                </>
              ) : (
                <>Make {isPrivate ? "Public" : "Private"}</>
              )}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}