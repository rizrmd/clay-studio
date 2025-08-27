import { useCallback } from 'react';
import { useSnapshot } from 'valtio';
import { 
  store, 
  updateInputDraft,
  setInputAttachments,
  addInputAttachment,
  removeInputAttachment,
  setInputTyping
} from '../store/chat-store';

/**
 * Hook for managing input state for a specific conversation
 * This preserves draft messages, attachments, and typing state across conversation switches
 */
export function useInputState(conversationId: string) {
  const snapshot = useSnapshot(store);
  const inputState = snapshot.inputs[conversationId] || {
    draftMessage: '',
    attachments: [],
    isTyping: false
  };

  const setDraftMessage = useCallback((draft: string) => {
    updateInputDraft(conversationId, draft);
  }, [conversationId]);

  const setAttachments = useCallback((attachments: File[]) => {
    setInputAttachments(conversationId, attachments);
  }, [conversationId]);

  const addAttachment = useCallback((attachment: File) => {
    addInputAttachment(conversationId, attachment);
  }, [conversationId]);

  const removeAttachment = useCallback((index: number) => {
    removeInputAttachment(conversationId, index);
  }, [conversationId]);

  const setTyping = useCallback((isTyping: boolean) => {
    setInputTyping(conversationId, isTyping);
  }, [conversationId]);

  const clearInput = useCallback(() => {
    updateInputDraft(conversationId, '');
    setInputAttachments(conversationId, []);
    setInputTyping(conversationId, false);
  }, [conversationId]);

  return {
    draftMessage: inputState.draftMessage,
    attachments: inputState.attachments,
    isTyping: inputState.isTyping,
    setDraftMessage,
    setAttachments,
    addAttachment,
    removeAttachment,
    setTyping,
    clearInput
  };
}