import { proxy } from 'valtio';

interface SystemConfig {
  registrationEnabled: boolean;
  requireInviteCode: boolean;
  allowedDomains: string[];
}

interface ConfigPageState {
  loading: boolean;
  config: SystemConfig;
  clientId: string | null;
}

export const configPageStore = proxy<ConfigPageState>({
  loading: true,
  config: {
    registrationEnabled: false,
    requireInviteCode: false,
    allowedDomains: [],
  },
  clientId: null,
});

export const configPageActions = {
  // Loading state
  setLoading: (loading: boolean) => {
    configPageStore.loading = loading;
  },

  // Config state
  setConfig: (config: SystemConfig) => {
    configPageStore.config = config;
  },

  updateConfig: (updates: Partial<SystemConfig>) => {
    configPageStore.config = {
      ...configPageStore.config,
      ...updates,
    };
  },

  // Client ID
  setClientId: (clientId: string | null) => {
    configPageStore.clientId = clientId;
  },

  // Reset state
  reset: () => {
    configPageStore.loading = true;
    configPageStore.config = {
      registrationEnabled: false,
      requireInviteCode: false,
      allowedDomains: [],
    };
    configPageStore.clientId = null;
  },
};