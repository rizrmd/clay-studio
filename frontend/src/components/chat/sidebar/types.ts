export interface Conversation {
  id: string;
  project_id: string;
  title: string;
  message_count: number;
  created_at: string;
  updated_at: string;
  is_title_manually_set?: boolean;
}
