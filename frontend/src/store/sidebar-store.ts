import { proxy } from 'valtio';

interface Conversation {
  id: string;
  project_id: string;
  title: string;
  message_count: number;
  created_at: string;
  updated_at: string;
  is_title_manually_set?: boolean;
}

interface SidebarState {
  // Conversation list
  conversations: Conversation[];
  loading: boolean;
  error: string | null;
  
  // UI states
  showNewConversationButton: boolean;
  isRefreshing: boolean;
  
  // Dialog states
  renameDialogOpen: boolean;
  renamingConversation: Conversation | null;
  newTitle: string;
  isMobileMenuOpen: boolean;
  
  // Recently updated tracking
  recentlyUpdatedConversations: Set<string>;
  
  // Delete mode
  isDeleteMode: boolean;
  selectedConversations: Set<string>;
}

export const sidebarStore = proxy<SidebarState>({
  conversations: [],
  loading: false,
  error: null,
  showNewConversationButton: true,
  isRefreshing: false,
  
  // Dialog states
  renameDialogOpen: false,
  renamingConversation: null,
  newTitle: '',
  isMobileMenuOpen: false,
  
  // Recently updated tracking
  recentlyUpdatedConversations: new Set(),
  
  // Delete mode
  isDeleteMode: false,
  selectedConversations: new Set(),
});

export const sidebarActions = {
  setConversations: (conversations: Conversation[]) => {
    sidebarStore.conversations = conversations;
  },
  
  addConversation: (conversation: Conversation) => {
    sidebarStore.conversations.unshift(conversation);
  },
  
  updateConversation: (id: string, updates: Partial<Conversation>) => {
    const index = sidebarStore.conversations.findIndex(c => c.id === id);
    if (index !== -1) {
      sidebarStore.conversations[index] = { ...sidebarStore.conversations[index], ...updates };
    }
  },
  
  removeConversation: (id: string) => {
    sidebarStore.conversations = sidebarStore.conversations.filter(c => c.id !== id);
  },
  
  setLoading: (loading: boolean) => {
    sidebarStore.loading = loading;
  },
  
  setError: (error: string | null) => {
    sidebarStore.error = error;
  },
  
  setRefreshing: (refreshing: boolean) => {
    sidebarStore.isRefreshing = refreshing;
  },
  
  clearError: () => {
    sidebarStore.error = null;
  },
  
  toggleNewConversationButton: () => {
    sidebarStore.showNewConversationButton = !sidebarStore.showNewConversationButton;
  },
  
  // Dialog actions
  openRenameDialog: (conversation: Conversation) => {
    sidebarStore.renamingConversation = conversation;
    sidebarStore.newTitle = conversation.title || '';
    sidebarStore.renameDialogOpen = true;
  },
  
  closeRenameDialog: () => {
    sidebarStore.renameDialogOpen = false;
    sidebarStore.renamingConversation = null;
    sidebarStore.newTitle = '';
  },
  
  setNewTitle: (title: string) => {
    sidebarStore.newTitle = title;
  },
  
  toggleMobileMenu: () => {
    sidebarStore.isMobileMenuOpen = !sidebarStore.isMobileMenuOpen;
  },
  
  setMobileMenuOpen: (open: boolean) => {
    sidebarStore.isMobileMenuOpen = open;
  },
  
  // Recently updated tracking
  addRecentlyUpdated: (conversationId: string) => {
    sidebarStore.recentlyUpdatedConversations.add(conversationId);
    // Auto-remove after 5 seconds
    setTimeout(() => {
      sidebarStore.recentlyUpdatedConversations.delete(conversationId);
    }, 5000);
  },
  
  clearRecentlyUpdated: () => {
    sidebarStore.recentlyUpdatedConversations.clear();
  },
  
  // Delete mode actions
  enterDeleteMode: (currentConversationId?: string) => {
    sidebarStore.isDeleteMode = true;
    sidebarStore.selectedConversations.clear();
    // Auto-select current conversation if provided
    if (currentConversationId) {
      sidebarStore.selectedConversations.add(currentConversationId);
    }
  },
  
  exitDeleteMode: () => {
    sidebarStore.isDeleteMode = false;
    sidebarStore.selectedConversations.clear();
  },
  
  toggleConversationSelection: (conversationId: string) => {
    const newSelected = new Set(sidebarStore.selectedConversations);
    if (newSelected.has(conversationId)) {
      newSelected.delete(conversationId);
    } else {
      newSelected.add(conversationId);
    }
    sidebarStore.selectedConversations = newSelected;
  },
  
  selectAllConversations: () => {
    sidebarStore.conversations.forEach(conv => {
      sidebarStore.selectedConversations.add(conv.id);
    });
  },
  
  deselectAllConversations: () => {
    sidebarStore.selectedConversations.clear();
  },
};