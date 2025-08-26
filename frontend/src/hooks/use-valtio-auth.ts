import { useEffect } from 'react'
import { useSnapshot } from 'valtio'
import {
  authStore,
  isAuthenticated,
  initializeApp,
  login as loginAction,
  register as registerAction,
  logout as logoutAction,
  checkRegistrationStatus
} from '../store/auth-store'

export function useValtioAuth() {
  const snapshot = useSnapshot(authStore)

  // Initialize app on mount
  useEffect(() => {
    initializeApp()
  }, [])

  // Update registration status when firstClient changes
  useEffect(() => {
    if (snapshot.firstClient) {
      checkRegistrationStatus()
    }
  }, [snapshot.firstClient])

  return {
    user: snapshot.user,
    firstClient: snapshot.firstClient,
    loading: snapshot.loading,
    isAuthenticated: isAuthenticated(),
    isSetupComplete: snapshot.isSetupComplete,
    needsInitialSetup: snapshot.needsInitialSetup,
    needsFirstUser: snapshot.needsFirstUser,
    registrationEnabled: snapshot.registrationEnabled,
    requireInviteCode: snapshot.requireInviteCode,
    login: loginAction,
    register: registerAction,
    logout: logoutAction,
    checkRegistrationStatus,
  }
}