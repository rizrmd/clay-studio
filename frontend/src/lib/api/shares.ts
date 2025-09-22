import { api } from "@/lib/utils/api";

export type ShareType = "new_chat" | "all_history" | "specific_conversations";

export interface ShareSettings {
  theme?: string;
  custom_css?: string;
  show_branding?: boolean;
  allow_file_upload?: boolean;
  show_conversation_list?: boolean;
  show_project_name?: boolean;
  enable_markdown?: boolean;
  layout_mode?: string;
  width?: string;
  height?: string;
  title?: string;
  description?: string;
  logo_url?: string;
  metadata?: Record<string, any>;
}

export interface CreateShareRequest {
  share_type: ShareType;
  settings: ShareSettings;
  is_read_only?: boolean;
  max_messages_per_session?: number;
  expires_at?: string;
  conversation_ids?: string[];
}

export interface UpdateShareRequest {
  settings?: ShareSettings;
  is_read_only?: boolean;
  max_messages_per_session?: number;
  expires_at?: string;
  conversation_ids?: string[];
}

export interface ProjectShare {
  id: string;
  project_id: string;
  share_token: string;
  share_type: ShareType;
  settings: ShareSettings;
  is_public: boolean;
  is_read_only: boolean;
  max_messages_per_session?: number;
  expires_at?: string;
  created_by?: string;
  created_at: string;
  updated_at: string;
  deleted_at?: string;
  view_count: number;
  last_accessed_at?: string;
}

export interface EmbedCodes {
  iframe_simple: string;
  iframe_responsive: string;
  javascript_sdk: string;
  react_component: string;
}

export interface ShareResponse {
  share: ProjectShare;
  conversations?: ProjectShareConversation[];
  embed_url: string;
  embed_codes: EmbedCodes;
}

export interface ProjectShareConversation {
  id: string;
  project_share_id: string;
  conversation_id: string;
  created_at: string;
}

export interface SharedProjectData {
  share: ProjectShare;
  project: {
    id: string;
    name: string;
    created_at: string;
    updated_at: string;
  };
  conversations: Array<{
    id: string;
    project_id: string;
    title?: string;
    created_at: string;
    updated_at: string;
    message_count: number;
  }>;
  session?: {
    id: string;
    session_token: string;
    expires_at: string;
    message_count: number;
    max_messages: number;
  };
}

export const sharesApi = {
  // Create a new project share
  async createShare(projectId: string, request: CreateShareRequest): Promise<ShareResponse> {
    console.log('Creating share for project:', projectId);
    console.log('Share request:', request);
    const response = await api.post(`/projects/${projectId}/shares`, request);
    console.log('Share response:', response);
    return response as ShareResponse;
  },

  // List all shares for a project
  async listShares(projectId: string): Promise<ProjectShare[]> {
    const response = await api.get(`/projects/${projectId}/shares`);
    return response as ProjectShare[];
  },

  // Get share details by token
  async getShare(shareToken: string): Promise<ShareResponse> {
    const response = await api.get(`/shares/${shareToken}`);
    return response as ShareResponse;
  },

  // Update share settings
  async updateShare(shareToken: string, request: UpdateShareRequest): Promise<ProjectShare> {
    const response = await api.put(`/shares/${shareToken}`, request);
    return response as ProjectShare;
  },

  // Delete a share
  async deleteShare(shareToken: string): Promise<void> {
    await api.delete(`/shares/${shareToken}`);
  },

  // Get shared project data (public endpoint)
  async getSharedData(shareToken: string): Promise<SharedProjectData> {
    const response = await api.get(`/shares/${shareToken}/data`);
    return response as SharedProjectData;
  },

  // Create a session for interacting with shared project
  async createSession(shareToken: string): Promise<{session_token: string; expires_at: string}> {
    const response = await api.post(`/shares/${shareToken}/session`, {});
    return response as {session_token: string; expires_at: string};
  },
};