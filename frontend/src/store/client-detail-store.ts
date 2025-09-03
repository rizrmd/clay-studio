import { proxy } from 'valtio';
import { ClientRootResponse } from '@/lib/services/root-service';

interface ClientDetailState {
  client: ClientRootResponse | null;
  loading: boolean;
  error: string | null;
  actionLoading: boolean;
  editDialogOpen: boolean;
  claudeDialogOpen: boolean;
}

export const clientDetailStore = proxy<ClientDetailState>({
  client: null,
  loading: true,
  error: null,
  actionLoading: false,
  editDialogOpen: false,
  claudeDialogOpen: false,
});

export const clientDetailActions = {
  setClient: (client: ClientRootResponse | null) => {
    clientDetailStore.client = client;
  },

  setLoading: (loading: boolean) => {
    clientDetailStore.loading = loading;
  },

  setError: (error: string | null) => {
    clientDetailStore.error = error;
  },

  setActionLoading: (actionLoading: boolean) => {
    clientDetailStore.actionLoading = actionLoading;
  },

  setEditDialogOpen: (open: boolean) => {
    clientDetailStore.editDialogOpen = open;
  },

  setClaudeDialogOpen: (open: boolean) => {
    clientDetailStore.claudeDialogOpen = open;
  },

  reset: () => {
    clientDetailStore.client = null;
    clientDetailStore.loading = true;
    clientDetailStore.error = null;
    clientDetailStore.actionLoading = false;
    clientDetailStore.editDialogOpen = false;
    clientDetailStore.claudeDialogOpen = false;
  },
};