import { api } from "@/lib/utils/api";

export interface ConversationVisibilityResponse {
  conversation_id: string;
  visibility: "private" | "public";
}

/**
 * Toggle conversation visibility between private and public
 * Only the conversation creator or project owner can change visibility
 */
export async function toggleConversationVisibility(
  conversationId: string
): Promise<ConversationVisibilityResponse> {
  return api.patch(`/api/conversations/${conversationId}/visibility`);
}