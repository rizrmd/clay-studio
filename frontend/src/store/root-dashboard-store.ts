import { proxy } from 'valtio';
import type { ClientRootResponse } from '@/lib/services/root-service';

interface RootDashboardState {
  // Data states
  clients: ClientRootResponse[];
  stats: {
    totalClients: number;
    activeClients: number;
    totalUsers: number;
    totalConversations: number;
  };

  // UI states
  loading: boolean;
  error: string | null;
  addDialogOpen: boolean;
}

export const rootDashboardStore = proxy<RootDashboardState>({
  // Data states
  clients: [],
  stats: {
    totalClients: 0,
    activeClients: 0,
    totalUsers: 0,
    totalConversations: 0,
  },

  // UI states
  loading: true,
  error: null,
  addDialogOpen: false,
});

export const rootDashboardActions = {
  // Data actions
  setClients: (clients: ClientRootResponse[]) => {
    rootDashboardStore.clients = clients;
  },

  addClient: (client: ClientRootResponse) => {
    rootDashboardStore.clients.push(client);
  },

  updateClient: (id: string, updates: Partial<ClientRootResponse>) => {
    const index = rootDashboardStore.clients.findIndex(c => c.id === id);
    if (index !== -1) {
      rootDashboardStore.clients[index] = { ...rootDashboardStore.clients[index], ...updates };
    }
  },

  removeClient: (id: string) => {
    rootDashboardStore.clients = rootDashboardStore.clients.filter(c => c.id !== id);
  },

  setStats: (stats: typeof rootDashboardStore.stats) => {
    rootDashboardStore.stats = stats;
  },

  // UI actions
  setLoading: (loading: boolean) => {
    rootDashboardStore.loading = loading;
  },

  setError: (error: string | null) => {
    rootDashboardStore.error = error;
  },

  setAddDialogOpen: (open: boolean) => {
    rootDashboardStore.addDialogOpen = open;
  },

  // Cleanup
  reset: () => {
    rootDashboardStore.clients = [];
    rootDashboardStore.stats = {
      totalClients: 0,
      activeClients: 0,
      totalUsers: 0,
      totalConversations: 0,
    };
    rootDashboardStore.loading = true;
    rootDashboardStore.error = null;
    rootDashboardStore.addDialogOpen = false;
  },
};