import { useCallback } from 'react';
import { inputActions } from '../store/input-store';

/**
 * Hook for managing input state for a specific conversation
 * This preserves draft messages, attachments, and typing state across conversation switches
 */
export function useInputState(conversationId: string) {
  const draftMessage = inputActions.getDraftMessage(conversationId);
  const attachments = inputActions.getAttachments(conversationId);
  const isTyping = inputActions.getTyping(conversationId);

  const setDraftMessage = useCallback((draft: string) => {
    inputActions.setDraftMessage(conversationId, draft);
  }, [conversationId]);

  const setAttachments = useCallback((attachments: File[]) => {
    inputActions.setAttachments(conversationId, attachments);
  }, [conversationId]);

  const addAttachment = useCallback((attachment: File) => {
    inputActions.setAttachments(conversationId, [...attachments, attachment]);
  }, [conversationId, attachments]);

  const removeAttachment = useCallback((index: number) => {
    const newAttachments = attachments.filter((_, i) => i !== index);
    inputActions.setAttachments(conversationId, newAttachments);
  }, [conversationId, attachments]);

  const setTyping = useCallback((isTyping: boolean) => {
    inputActions.setTyping(conversationId, isTyping);
  }, [conversationId]);

  const clearInput = useCallback(() => {
    inputActions.clearDraftMessage(conversationId);
    inputActions.clearAttachments(conversationId);
    inputActions.clearTyping(conversationId);
  }, [conversationId]);

  return {
    draftMessage,
    attachments,
    isTyping,
    setDraftMessage,
    setAttachments,
    addAttachment,
    removeAttachment,
    setTyping,
    clearInput
  };
}