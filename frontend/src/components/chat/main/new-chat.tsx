import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useSnapshot } from "valtio";
import { WelcomeScreen, Messages } from "../display";
import { MultimodalInput } from "../input";
import { api } from "@/lib/utils/api";
import { uiStore, uiActions } from "@/store/ui-store";
import type { Message } from "../display/types";

export function NewChat() {
  const uiSnapshot = useSnapshot(uiStore);
  const projectId = uiSnapshot.currentProjectId;
  const navigate = useNavigate();
  const [input, setInput] = useState("");
  const [files, setFiles] = useState<File[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [isCreatingConversation, setIsCreatingConversation] = useState(false);

  const handleSubmit = async (e: React.FormEvent, message: string, uploadFiles?: File[]) => {
    e.preventDefault();
    if (!message.trim() || !projectId) return;

    // Create a new message and add it to local state
    const newMessage: Message = {
      id: Date.now().toString(),
      content: message,
      role: "user",
      createdAt: new Date(),
      file_attachments: uploadFiles?.map(file => ({
        id: Date.now().toString(),
        file_name: file.name,
        original_name: file.name,
        file_path: "",
        file_size: file.size,
        mime_type: file.type,
        description: undefined,
        auto_description: undefined
      }))
    };

    const updatedMessages = [...messages, newMessage];
    setMessages(updatedMessages);
    
    // Clear input
    setInput("");
    setFiles([]);

    // If this is the first message, create a real conversation and transition
    if (messages.length === 0) {
      setIsCreatingConversation(true);
      
      try {
        // Create new conversation
        const response = await api.fetchStream("/conversations", {
          method: "POST", 
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            project_id: projectId,
          }),
        });

        if (!response.ok) {
          throw new Error("Failed to create new conversation");
        }

        const newConversation = await response.json();
        
        // Set transition state in valtio
        uiActions.setTransitioningFromNew(true);
        
        // Navigate to new conversation with transition flags
        navigate(`/p/${projectId}/c/${newConversation.id}`, { 
          replace: true,
          state: { 
            initialMessage: message,
            initialFiles: uploadFiles || [],
            fromNewChat: true,
            existingMessages: updatedMessages
          }
        });
      } catch (error) {
        console.error("Failed to create new conversation:", error);
        setIsCreatingConversation(false);
      }
    }
  };

  return (
    <>
      <div className="flex-1 overflow-hidden flex flex-col">
        <div className="flex-1 overflow-y-auto">
          {messages.length === 0 ? (
            <WelcomeScreen />
          ) : (
            <Messages
              messages={messages}
              isLoading={false}
              onForgetFrom={() => {}}
              conversationId="new"
              messageQueue={[]}
              isProcessingQueue={false}
              onEditQueued={() => {}}
              onCancelQueued={() => {}}
              isStreaming={false}
              canStop={false}
              onStop={() => {}}
              activeTools={[]}
              onResendMessage={() => {}}
              onNewChatFromHere={() => {}}
              onAskUserSubmit={() => {}}
            />
          )}
        </div>
      </div>
      <div className="bg-background border-t">
        <div className="mx-auto max-w-2xl sm:px-4">
          <MultimodalInput
            input={input}
            setInput={setInput}
            handleSubmit={handleSubmit}
            isLoading={isCreatingConversation}
            isStreaming={false}
            projectId={projectId}
            uploadedFiles={[]}
            externalFiles={files}
            onExternalFilesChange={setFiles}
            shouldFocus={true}
            isSubscribed={true}
            conversationId="new"
            className="lg:ml-[-10px] lg:mr-[40px]"
          />
        </div>
      </div>
    </>
  );
}