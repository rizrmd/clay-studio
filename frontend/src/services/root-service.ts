import axios from '@/lib/axios'

export interface ClientAdminResponse {
  id: string
  name: string
  description?: string
  status: 'pending' | 'installing' | 'active' | 'suspended' | 'error'
  installPath: string
  domains?: string[]
  userCount: number
  projectCount: number
  createdAt: string
  updatedAt: string
}

export interface ClientRootResponse extends ClientAdminResponse {
  config: any
  hasClaudeToken: boolean
  conversationCount: number
  deletedAt?: string | null
}

export interface ClientUpdateRequest {
  name?: string
  description?: string
  domains?: string[]
}

export interface UpdateConfigRequest {
  config: any
}

export interface UpdateDomainsRequest {
  domains: string[]
}

export interface CreateClientRequest {
  name: string
  description?: string
  domains?: string[]
}

export interface UserResponse {
  id: string
  client_id: string
  username: string
  role: 'user' | 'admin' | 'root'
  status: 'active' | 'suspended'
  lastActive?: string | null
  createdAt: string
  updatedAt: string
}

export interface CreateUserRequest {
  username: string
  password: string
  role?: 'user' | 'admin'
}

export interface UpdateUserRequest {
  username?: string
  role?: 'user' | 'admin' | 'root'
}

class RootService {
  // Admin endpoints (read-only, for admin and root roles)
  async getClientsAdmin(): Promise<ClientAdminResponse[]> {
    const response = await axios.get('/admin/clients')
    return response.data
  }

  async getClientAdmin(clientId: string): Promise<ClientAdminResponse> {
    const response = await axios.get(`/admin/clients/${clientId}`)
    return response.data
  }

  // Root-only endpoints (full access)
  async getClientsRoot(): Promise<ClientRootResponse[]> {
    const response = await axios.get('/root/clients')
    return response.data
  }

  async getClientRoot(clientId: string): Promise<ClientRootResponse> {
    const response = await axios.get(`/root/clients/${clientId}`)
    return response.data
  }

  async updateClient(clientId: string, data: ClientUpdateRequest): Promise<void> {
    await axios.put(`/root/clients/${clientId}`, data)
  }

  async deleteClient(clientId: string): Promise<void> {
    await axios.delete(`/root/clients/${clientId}`)
  }

  async enableClient(clientId: string): Promise<void> {
    await axios.post(`/root/clients/${clientId}/enable`)
  }

  async disableClient(clientId: string): Promise<void> {
    await axios.post(`/root/clients/${clientId}/disable`)
  }

  async updateClientConfig(clientId: string, config: any): Promise<void> {
    await axios.put(`/root/clients/${clientId}/config`, { config })
  }

  async updateClientDomains(clientId: string, domains: string[]): Promise<void> {
    await axios.put(`/root/clients/${clientId}/domains`, { domains })
  }

  async createClient(data: CreateClientRequest): Promise<ClientRootResponse> {
    const response = await axios.post('/root/clients', data)
    return response.data
  }

  async suspendClient(clientId: string): Promise<void> {
    await axios.post(`/root/clients/${clientId}/suspend`)
  }

  async setClaudeToken(clientId: string, claudeToken: string): Promise<void> {
    await axios.post(`/root/clients/${clientId}/claude-token`, { claude_token: claudeToken })
  }

  // User management endpoints (root-only)
  async getUsersForClient(clientId: string): Promise<UserResponse[]> {
    const response = await axios.get(`/root/clients/${clientId}/users`)
    return response.data
  }

  async getUser(clientId: string, userId: string): Promise<UserResponse> {
    const response = await axios.get(`/root/clients/${clientId}/users/${userId}`)
    return response.data
  }

  async createUser(clientId: string, userData: CreateUserRequest): Promise<UserResponse> {
    const response = await axios.post(`/root/clients/${clientId}/users`, userData)
    return response.data
  }

  async updateUser(clientId: string, userId: string, userData: UpdateUserRequest): Promise<void> {
    await axios.put(`/root/clients/${clientId}/users/${userId}`, userData)
  }

  async deleteUser(clientId: string, userId: string): Promise<void> {
    await axios.delete(`/root/clients/${clientId}/users/${userId}`)
  }
}

export const rootService = new RootService()