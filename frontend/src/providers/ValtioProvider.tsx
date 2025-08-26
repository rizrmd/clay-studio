import { ReactNode, useEffect } from 'react'
import { initializeApp } from '../store/auth-store'

interface ValtioProviderProps {
  children: ReactNode
}

/**
 * Provider that initializes the Valtio stores
 * This replaces the need for React Context providers
 */
export function ValtioProvider({ children }: ValtioProviderProps) {
  // Initialize the auth store on app load
  useEffect(() => {
    initializeApp()
  }, [])

  return <>{children}</>
}