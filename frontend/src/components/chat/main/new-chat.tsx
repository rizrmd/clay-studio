import { useChat } from "@/lib/hooks/use-chat";
import {
  inputActions,
  inputStore,
  multimodalInputActions,
} from "@/lib/store/chat/input-store";
import { uiStore } from "@/lib/store/chat/ui-store";
import { chatStore } from "@/lib/store/chat/chat-store";
import { useEffect, useState } from "react";
import { useSnapshot } from "valtio";
import { WelcomeScreen } from "../display";
import { MultimodalInput } from "../input";
import { useFileUpload } from "../input/use-file-upload";

export function NewChat() {
  const uiSnapshot = useSnapshot(uiStore);
  const chatSnapshot = useSnapshot(chatStore);
  const inputSnapshot = useSnapshot(inputStore);
  const projectId = uiSnapshot.currentProject;
  const { createConversation, conversationId } = useChat();
  const { uploadFiles } = useFileUpload("new", projectId || undefined);

  const [waitingForSubscription, setWaitingForSubscription] = useState(false);
  const [isCreating, setIsCreating] = useState(false);

  // Use input state from the store
  const [input, setInput] = useState("");
  const [files, setFiles] = useState<File[]>([]);

  // Check for pending input text on mount (e.g., from "Fix with Chat" button)
  useEffect(() => {
    if (chatSnapshot.pendingInputText) {
      console.log('NewChat: applying pending input text:', chatSnapshot.pendingInputText);
      setInput(chatSnapshot.pendingInputText);
      chatStore.pendingInputText = ''; // Clear after applying
    }
  }, []); // Only run once on mount

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
    uploadedFiles?: File[]
  ) => {
    e.preventDefault();
    if (!message.trim() || !projectId) return;

    // Collect file IDs from already selected files
    const initialFileIds: string[] = inputSnapshot.selectedFiles.map(f => f.id);
    let fileIds: string[] = [...initialFileIds];

    // Upload new files if any and get their IDs
    const allFiles = [...(uploadedFiles || []), ...files];
    if (allFiles.length > 0) {
      try {
        await uploadFiles(allFiles);
        // After upload, get the newly added file IDs
        const updatedSelectedFiles = inputStore.selectedFiles;
        const newFileIds = updatedSelectedFiles
          .filter(f => !initialFileIds.includes(f.id))
          .map(f => f.id);
        fileIds = [...initialFileIds, ...newFileIds];
      } catch (error) {
        console.error('Failed to upload files:', error);
        // Still send message without files if upload fails
      }
    }

    setInput("");
    setFiles([]);
    setIsCreating(true);

    // Clear input state
    inputActions.clearSelectedFiles();
    multimodalInputActions.setLocalInput("new", "");

    // Create conversation with first message and file IDs
    createConversation(message, fileIds.length > 0 ? fileIds : undefined);
  };

  return (
    <>
      <div className="flex-1 overflow-hidden relative flex flex-col">
        <div className="flex-1 overflow-y-auto">
          {isCreating ? (
            <div className="flex items-center justify-center h-full">
              <div className="text-muted-foreground">Creating New Chat...</div>
            </div>
          ) : (
            <WelcomeScreen />
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
