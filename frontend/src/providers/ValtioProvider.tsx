import { ReactNode, useEffect } from 'react'
import { initializeApp, authStore } from '../store/auth-store'
import { WebSocketService } from '../services/chat/websocket-service'
import { useSnapshot } from 'valtio'

interface ValtioProviderProps {
  children: ReactNode
}

/**
 * Provider that initializes the Valtio stores
 * This replaces the need for React Context providers
 */
export function ValtioProvider({ children }: ValtioProviderProps) {
  const { user } = useSnapshot(authStore)
  
  // Initialize the auth store on app load
  useEffect(() => {
    initializeApp()
  }, [])
  
  // Connect WebSocket only when user is authenticated
  useEffect(() => {
    if (user) {
      const connectWebSocket = async () => {
        // Small delay to ensure cookies are properly set after login
        await new Promise(resolve => setTimeout(resolve, 100))
        
        const wsService = WebSocketService.getInstance()
        wsService.connect().catch(error => {
          console.warn('WebSocket connection failed:', error)
          // Retry once after another delay
          setTimeout(() => {
            wsService.connect().catch(err => {
              console.warn('WebSocket retry failed:', err)
            })
          }, 1000)
        })
      }
      
      connectWebSocket()
    }
  }, [user])

  return <>{children}</>
}