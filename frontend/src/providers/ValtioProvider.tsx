import { ReactNode, useEffect } from 'react'
import { initializeApp } from '../store/auth-store'
import { WebSocketService } from '../services/chat/websocket-service'

interface ValtioProviderProps {
  children: ReactNode
}

/**
 * Provider that initializes the Valtio stores
 * This replaces the need for React Context providers
 */
export function ValtioProvider({ children }: ValtioProviderProps) {
  // Initialize the auth store and WebSocket service on app load
  useEffect(() => {
    initializeApp()
    
    // Delay WebSocket connection to ensure session cookies are ready
    // This prevents auth failures on first visit after deployment
    const connectWebSocket = async () => {
      // Small delay to ensure cookies are properly set
      await new Promise(resolve => setTimeout(resolve, 100))
      
      const wsService = WebSocketService.getInstance()
      wsService.connect().catch(error => {
        console.warn('Initial WebSocket connection failed:', error)
        // Retry once after another delay
        setTimeout(() => {
          wsService.connect().catch(err => {
            console.warn('WebSocket retry failed:', err)
          })
        }, 1000)
      })
    }
    
    connectWebSocket()
  }, [])

  return <>{children}</>
}