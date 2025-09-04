import { useState, useEffect } from "react";
import { useSnapshot } from "valtio";
import { useNavigate } from "react-router-dom";
import { Messages, WelcomeScreen } from "../display";
import type { Message } from "@/lib/types/chat";
import { MultimodalInput } from "../input";
import { useChat } from "@/lib/hooks/use-chat";
import { uiStore } from "@/lib/store/chat/ui-store";
import {
  inputActions,
  multimodalInputActions,
} from "@/lib/store/chat/input-store";
import { messageUIActions } from "@/lib/store/chat/message-ui-store";
import { wsService } from "@/lib/services/ws-service";
import { Button } from "@/components/ui/button";

export function NewChat() {
  const uiSnapshot = useSnapshot(uiStore);
  const projectId = uiSnapshot.currentProject;
  const navigate = useNavigate();
  const { sendMessage, createConversation, conversationId } = useChat();

  // Local state for new chat messages and creation status
  const [messages, setMessages] = useState<Message[]>([]);
  const [pendingMessage, setPendingMessage] = useState<string>("");
  const [waitingForSubscription, setWaitingForSubscription] = useState(false);

  // Use input state from the store
  const [input, setInput] = useState("");
  const [files, setFiles] = useState<File[]>([]);

  // Handle conversation creation and subscription flow
  useEffect(() => {
    if (
      conversationId &&
      conversationId !== "new" &&
      pendingMessage &&
      !waitingForSubscription
    ) {
      // Wait for subscription to be confirmed before sending message
      setWaitingForSubscription(true);
    }
  }, [conversationId, pendingMessage, waitingForSubscription]);

  // Listen for subscription confirmation
  useEffect(() => {
    if (!waitingForSubscription) return;

    const handleSubscribed = (message: any) => {
      if (message.conversation_id === conversationId && pendingMessage) {
        // Now we can safely send the message
        sendMessage(pendingMessage);
        setPendingMessage("");
        setWaitingForSubscription(false);

        messageUIActions.setPreviousConversationId(conversationId);

        // Navigate to the new conversation
        navigate(`/p/${projectId}/c/${conversationId}`, {
          replace: true,
        });
      }
    };

    wsService.on("subscribed", handleSubscribed);

    return () => {
      wsService.off("subscribed", handleSubscribed);
    };
  }, [
    waitingForSubscription,
    conversationId,
    pendingMessage,
    sendMessage,
    navigate,
    projectId,
  ]);

  const handleSubmit = async (
    e: React.FormEvent,
    message: string,
    uploadFiles?: File[]
  ) => {
    console.log("asda");
    e.preventDefault();
    if (!message.trim() || !projectId) return;

    // Add message to local state optimistically
    const newMessage: Message = {
      id: Date.now().toString(),
      content: message,
      role: "user",
      createdAt: new Date().toISOString(),
      file_attachments: uploadFiles?.map((file) => ({
        id: Date.now().toString(),
        file_name: file.name,
        original_name: file.name,
        file_path: "",
        file_size: file.size,
        mime_type: file.type,
        description: undefined,
        auto_description: undefined,
        created_at: new Date().toISOString(),
        is_text_file: false,
      })),
    };

    // Add message optimistically
    setMessages((prev) => [...prev, newMessage]);
    setInput("");
    setFiles([]);

    // Clear input state
    inputActions.clearSelectedFiles();
    multimodalInputActions.setLocalInput("new", "");

    // Store the message to send after conversation is created
    setPendingMessage(message);

    // Create new conversation with title from first part of message
    const conversationTitle =
      message.slice(0, 50).trim() + (message.length > 50 ? "..." : "");
    createConversation(conversationTitle);
  };

  return (
    <>
      <div className="flex-1 overflow-hidden relative flex flex-col">
        <div className="absolute top-0 left-0 text-xs">
          {JSON.stringify({
            messages: messages.length,
          })}
        </div>

        <div className="flex-1 overflow-y-auto">
          {messages.length === 0 ? (
            <WelcomeScreen />
          ) : (
            <div className="flex flex-1 justify-center items-center absolute inset-0">
              <Button disabled className="rounded-full">Creating new chat...</Button>
            </div>
          )}
        </div>
      </div>
      <div className="bg-background border-t">
        <div className="mx-auto max-w-2xl sm:px-4">
          <MultimodalInput
            input={input}
            setInput={setInput}
            handleSubmit={handleSubmit}
            isLoading={false}
            isStreaming={false}
            projectId={projectId || undefined}
            uploadedFiles={[]}
            externalFiles={[...files]}
            onExternalFilesChange={setFiles}
            shouldFocus={true}
            isSubscribed={true}
            className="lg:ml-[-10px] lg:mr-[40px]"
          />
        </div>
      </div>
    </>
  );
}
