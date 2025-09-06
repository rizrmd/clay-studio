import { Button } from "@/components/ui/button";
import { useChat } from "@/lib/hooks/use-chat";
import {
  inputActions,
  multimodalInputActions,
} from "@/lib/store/chat/input-store";
import { uiStore } from "@/lib/store/chat/ui-store";
import type { Message } from "@/lib/types/chat";
import { useEffect, useState } from "react";
import { useSnapshot } from "valtio";
import { WelcomeScreen } from "../display";
import { MultimodalInput } from "../input";

export function NewChat() {
  const uiSnapshot = useSnapshot(uiStore);
  const projectId = uiSnapshot.currentProject;
  const { createConversation, conversationId } = useChat();

  // Local state for new chat messages and creation status
  const [messages, setMessages] = useState<Message[]>([]);
  const [waitingForSubscription, setWaitingForSubscription] = useState(false);

  // Use input state from the store
  const [input, setInput] = useState("");
  const [files, setFiles] = useState<File[]>([]);

  // Handle conversation creation and subscription flow
  useEffect(() => {
    if (conversationId && conversationId !== "new" && !waitingForSubscription) {
      // Wait for subscription to be confirmed before sending message
      setWaitingForSubscription(true);
    }
  }, [conversationId, waitingForSubscription]);

  const handleSubmit = async (
    e: React.FormEvent,
    message: string,
    uploadFiles?: File[]
  ) => {
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

    createConversation(message);
  };

  return (
    <>
      <div className="flex-1 overflow-hidden relative flex flex-col">
        <div className="flex-1 overflow-y-auto">
          {messages.length === 0 ? (
            <WelcomeScreen />
          ) : (
            <div className="flex flex-1 justify-center items-center absolute inset-0">
              <Button disabled className="rounded-full">
                Creating new chat...
              </Button>
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
          />
        </div>
      </div>
    </>
  );
}
