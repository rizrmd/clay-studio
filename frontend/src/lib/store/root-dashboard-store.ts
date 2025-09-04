import { proxy } from 'valtio'
import type { ClientRootResponse } from '@/lib/services/root-service'

interface DashboardStats {
  totalProjects: number
  totalConversations: number
  totalUsers: number
  totalClients: number
  activeClients: number
}

export const rootDashboardStore = proxy({
  stats: {
    totalProjects: 0,
    totalConversations: 0,
    totalUsers: 0,
    totalClients: 0,
    activeClients: 0,
  } as DashboardStats,
  clients: [] as ClientRootResponse[],
  loading: false,
  isLoading: false,
  error: null as string | null,
  addDialogOpen: false,
})

export const rootDashboardActions = {
  setStats: (stats: DashboardStats) => {
    rootDashboardStore.stats = stats
  },

  setClients: (clients: ClientRootResponse[]) => {
    rootDashboardStore.clients = clients
  },

  setLoading: (isLoading: boolean) => {
    rootDashboardStore.loading = isLoading
    rootDashboardStore.isLoading = isLoading
  },

  setError: (error: string | null) => {
    rootDashboardStore.error = error
  },

  setAddDialogOpen: (open: boolean) => {
    rootDashboardStore.addDialogOpen = open
  },

  addClient: (client: ClientRootResponse) => {
    rootDashboardStore.clients.push(client)
  },

  updateClient: (clientId: string, updates: Partial<ClientRootResponse>) => {
    const index = rootDashboardStore.clients.findIndex(c => c.id === clientId)
    if (index !== -1) {
      Object.assign(rootDashboardStore.clients[index], updates)
    }
  },

  removeClient: (clientId: string) => {
    rootDashboardStore.clients = rootDashboardStore.clients.filter(c => c.id !== clientId)
  },
}