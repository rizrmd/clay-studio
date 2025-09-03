import { useEffect, useRef } from "react";
import { useSnapshot } from "valtio";
import {
  store,
  getConversationState,
  setActiveConversation,
  setConversationAbortController,
  getConversationAbortController,
} from "@/store/chat-store";
import {
  loadConversationMessages,
  loadUploadedFiles,
  checkForgottenStatus,
} from "./message-utils";

interface ConversationInitOptions {
  projectId: string;
  conversationId?: string;
  navigate: (path: string) => void;
}

/**
 * Hook to handle conversation initialization and state management
 */
export function useConversationInit({
  projectId,
  conversationId,
  navigate,
}: ConversationInitOptions) {
  const snapshot = useSnapshot(store);
  const currentConversationId = conversationId || "new";
  
  // Track previous conversation ID to detect actual navigation
  const previousConversationIdRef = useRef<string | undefined>();
  const isInitialMountRef = useRef(true);

  // Set active conversation
  useEffect(() => {
    setActiveConversation(currentConversationId);
  }, [currentConversationId]);

  // Load messages when conversation changes
  useEffect(() => {
    if (!conversationId) return;

    const previousConversationId = previousConversationIdRef.current;
    const isNavigation = previousConversationId !== conversationId;
    const isInitialMount = isInitialMountRef.current;
    previousConversationIdRef.current = conversationId;
    isInitialMountRef.current = false;

    if (conversationId === "new") {
      if (!isNavigation) return;
      
      initializeNewConversation();
    } else {
      // Check if we already have messages for this conversation
      const existingState = snapshot.conversations[conversationId];
      
      if (!isInitialMount && existingState && existingState.messages.length > 0) {
        const isTransitionFromNew = store.activeConversationId === conversationId && 
                                  previousConversationId === "new";
        
        if (!isTransitionFromNew) {
          // Don't reload if we have cached messages
          return;
        }
      }

      // Load messages from database
      loadConversationMessages(conversationId, projectId, navigate).then(() => {
        loadUploadedFiles(projectId, conversationId);
      });
      
      // Check forgotten status
      checkForgottenStatus(conversationId);
    }

    // Cleanup function
    return () => {
      if (conversationId === "new") {
        const controller = getConversationAbortController("new");
        if (controller) {
          controller.abort();
        }
      }
    };
  }, [conversationId, projectId, navigate]);

  return currentConversationId;
}

/**
 * Initialize state for a new conversation
 */
function initializeNewConversation() {
  const state = getConversationState("new");
  const activeId = store.activeConversationId;
  
  // activeId debug log removed

  if (activeId && activeId !== "new" && activeId.startsWith("conv-")) {
    // We have a real conversation that was created from 'new'
    state.isLoading = false;
    state.isLoadingMessages = false;
    state.isStreaming = false;
    state.error = null;
    state.activeTools = [];
    
    const controller = getConversationAbortController("new");
    if (controller) {
      controller.abort();
      setConversationAbortController("new", null);
    }
  } else {
    // Starting fresh - clear everything
    state.messages = [];
    state.error = null;
    state.forgottenAfterMessageId = null;
    state.forgottenCount = 0;
    state.uploadedFiles = [];
    state.isLoading = false;
    state.isLoadingMessages = false;
    state.isStreaming = false;
    state.messageQueue = [];
    state.activeTools = [];
    
    const freshController = getConversationAbortController("new");
    if (freshController) {
      freshController.abort();
      setConversationAbortController("new", null);
    }
    store.activeConversationId = null;
  }
}