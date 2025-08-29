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
    
    // Initialize and connect WebSocket service immediately
    const wsService = WebSocketService.getInstance()
    wsService.connect().catch(error => {
      console.warn('Initial WebSocket connection failed:', error)
    })
  }, [])

  return <>{children}</>
}