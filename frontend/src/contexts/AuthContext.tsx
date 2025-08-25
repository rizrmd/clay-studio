import { createContext, useContext, useState, useEffect, ReactNode } from 'react'
import axios from '@/lib/axios'

export interface User {
  id: string
  client_id: string
  username: string
}

export interface Client {
  id: string
  name: string
  description?: string
  status?: 'pending' | 'installing' | 'active' | 'error'
  installPath?: string
  createdAt?: string
  updatedAt?: string
}

interface AuthContextType {
  user: User | null
  firstClient: Client | null
  login: (username: string, password: string) => Promise<void>
  register: (username: string, password: string, inviteCode?: string) => Promise<void>
  logout: () => Promise<void>
  loading: boolean
  isAuthenticated: boolean
  isSetupComplete: boolean
  needsInitialSetup: boolean
  needsFirstUser: boolean
  registrationEnabled: boolean
  requireInviteCode: boolean
  checkRegistrationStatus: () => Promise<void>
}

const AuthContext = createContext<AuthContextType | undefined>(undefined)

interface AuthProviderProps {
  children: ReactNode
}

export function AuthProvider({ children }: AuthProviderProps) {
  const [user, setUser] = useState<User | null>(null)
  const [loading, setLoading] = useState(true)
  const [isSetupComplete, setIsSetupComplete] = useState(false)
  const [firstClient, setFirstClient] = useState<Client | null>(null)
  const [needsInitialSetup, setNeedsInitialSetup] = useState(false)
  const [needsFirstUser, setNeedsFirstUser] = useState(false)
  const [registrationEnabled, setRegistrationEnabled] = useState(false)
  const [requireInviteCode, setRequireInviteCode] = useState(false)

  const isAuthenticated = !!user

  // Check authentication status on app load
  useEffect(() => {
    initializeApp()
  }, [])

  const initializeApp = async () => {
    try {
      // First fetch the client data
      const clientData = await fetchFirstClient()
      
      // Then check auth status (this needs the client to be set for proper context)
      const userData = await checkAuthStatus()
      
      // After fetching client, check if users exist (only if client is active and no user is logged in)
      if (!userData && clientData?.status === 'active' && clientData?.id) {
        await checkUsersExist(clientData.id)
      }
    } finally {
      setLoading(false)
    }
  }

  const checkAuthStatus = async () => {
    try {
      const response = await axios.get('/auth/me')
      setUser(response.data.user)
      setIsSetupComplete(response.data.is_setup_complete)
      return response.data.user
    } catch (error: any) {
      // Only log if it's not a 401 (which is expected when no user is logged in)
      // 401 is expected when no user is logged in
      setUser(null)
      setIsSetupComplete(false)
      return null
    }
  }

  const fetchFirstClient = async () => {
    try {
      // First try to get all clients (including incomplete ones)
      // This endpoint should work even when not authenticated
      const response = await axios.get('/clients')
      if (response.data && response.data.length > 0) {
        const client = response.data[0]
        setFirstClient(client)
        // Store the active client ID in localStorage for file uploads
        if (client.id) {
          localStorage.setItem('activeClientId', client.id)
        }
        // Only need initial setup if no clients exist at all
        setNeedsInitialSetup(false)
        return client // Return client for further processing
      } else {
        // No clients exist - initial setup needed
        setFirstClient(null)
        setNeedsInitialSetup(true)
        return null
      }
    } catch (error) {
      // Don't assume initial setup is needed if we can't fetch clients
      // The endpoint might be temporarily unavailable
      setNeedsInitialSetup(false)
      return null
    }
  }

  const checkUsersExist = async (clientId: string) => {
    try {
      const response = await axios.get('/auth/users/exists', {
        params: { client_id: clientId }
      })
      const usersExist = response.data.users_exist
      // If client is active but no users exist, we need to create the first user
      setNeedsFirstUser(!usersExist)
    } catch (error) {
      // Assume users exist if we can't check (safer default)
      setNeedsFirstUser(false)
    }
  }

  const checkRegistrationStatus = async () => {
    if (!firstClient) return
    
    try {
      const response = await axios.get('/auth/registration-status', {
        params: { client_id: firstClient.id }
      })
      setRegistrationEnabled(response.data.registration_enabled)
      setRequireInviteCode(response.data.require_invite_code)
    } catch (error) {
      setRegistrationEnabled(false)
      setRequireInviteCode(false)
    }
  }

  const login = async (username: string, password: string) => {
    if (!firstClient) {
      throw new Error('No client available')
    }
    
    try {
      const response = await axios.post('/auth/login', {
        client_id: firstClient.id,
        username,
        password,
      })
      setUser(response.data.user)
      
      // After login, fetch the complete auth status including isSetupComplete
      // Use a small delay to ensure session cookie is set
      await new Promise(resolve => setTimeout(resolve, 100))
      
      try {
        const meResponse = await axios.get('/auth/me')
        setIsSetupComplete(meResponse.data.is_setup_complete)
      } catch (error) {
        // If we can't get the setup status, assume it's not complete
        // This is safer than assuming it's complete
        setIsSetupComplete(false)
      }
    } catch (error: any) {
      if (error.response?.data?.error) {
        throw new Error(error.response.data.error)
      }
      throw new Error('Login failed')
    }
  }

  const register = async (username: string, password: string, inviteCode?: string) => {
    if (!firstClient) {
      throw new Error('No client available')
    }
    
    try {
      const response = await axios.post('/auth/register', {
        client_id: firstClient.id,
        username,
        password,
        invite_code: inviteCode,
      })
      setUser(response.data.user)
      
      // After registration, fetch the complete auth status including isSetupComplete
      // Use a small delay to ensure session cookie is set
      await new Promise(resolve => setTimeout(resolve, 100))
      
      try {
        const meResponse = await axios.get('/auth/me')
        setIsSetupComplete(meResponse.data.is_setup_complete)
      } catch (error) {
        // If we can't get the setup status, assume it's not complete
        // This is safer than assuming it's complete
        setIsSetupComplete(false)
      }
    } catch (error: any) {
      if (error.response?.data?.error) {
        throw new Error(error.response.data.error)
      }
      throw new Error('Registration failed')
    }
  }

  const logout = async () => {
    try {
      await axios.post('/auth/logout')
    } catch (error) {
      // Logout error
    } finally {
      setUser(null)
      setIsSetupComplete(false)
    }
  }

  // Update registration status when firstClient changes
  useEffect(() => {
    if (firstClient) {
      checkRegistrationStatus()
    }
  }, [firstClient])

  const value = {
    user,
    firstClient,
    login,
    register,
    logout,
    loading,
    isAuthenticated,
    isSetupComplete,
    needsInitialSetup,
    needsFirstUser,
    registrationEnabled,
    requireInviteCode,
    checkRegistrationStatus,
  }

  return (
    <AuthContext.Provider value={value}>
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth() {
  const context = useContext(AuthContext)
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}