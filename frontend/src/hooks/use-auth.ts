import { useEffect } from "react";
import { useSnapshot } from "valtio";
import {
  authStore,
  isAuthenticated,
  login as loginAction,
  register as registerAction,
  logout as logoutAction,
  checkRegistrationStatus,
} from "../lib/store/auth-store";

export function useAuth() {
  const snapshot = useSnapshot(authStore);

  // Update registration status when firstClient changes
  useEffect(() => {
    if (snapshot.firstClient) {
      checkRegistrationStatus();
    }
  }, [snapshot.firstClient]);

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
  };
}
