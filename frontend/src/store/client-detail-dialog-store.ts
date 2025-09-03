import { proxy } from 'valtio';
import { ClientRootResponse } from '@/lib/services/root-service';

interface ClientDetailDialogState {
  // Dialog state
  activeTab: string;
  loading: boolean;
  error: string | null;

  // Form states
  name: string;
  description: string;
  domains: string[];
  newDomain: string;
  config: string;

  // Registration settings
  registrationEnabled: boolean;
  requireInviteCode: boolean;
  inviteCode: string;
}

export const clientDetailDialogStore = proxy<ClientDetailDialogState>({
  activeTab: 'details',
  loading: false,
  error: null,
  name: '',
  description: '',
  domains: [],
  newDomain: '',
  config: '{}',
  registrationEnabled: false,
  requireInviteCode: false,
  inviteCode: '',
});

export const clientDetailDialogActions = {
  // Initialize state from client data
  initializeFromClient: (client: ClientRootResponse) => {
    clientDetailDialogStore.activeTab = 'details';
    clientDetailDialogStore.loading = false;
    clientDetailDialogStore.error = null;
    clientDetailDialogStore.name = client.name;
    clientDetailDialogStore.description = client.description || '';
    clientDetailDialogStore.domains = client.domains || [];
    clientDetailDialogStore.newDomain = '';

    // Parse config
    const configObj = typeof client.config === 'object' ? client.config : {};
    clientDetailDialogStore.config = JSON.stringify(client.config, null, 2);
    clientDetailDialogStore.registrationEnabled = configObj.registration_enabled || false;
    clientDetailDialogStore.requireInviteCode = configObj.require_invite_code || false;
    clientDetailDialogStore.inviteCode = configObj.invite_code || '';
  },

  // Dialog state
  setActiveTab: (tab: string) => {
    clientDetailDialogStore.activeTab = tab;
  },

  setLoading: (loading: boolean) => {
    clientDetailDialogStore.loading = loading;
  },

  setError: (error: string | null) => {
    clientDetailDialogStore.error = error;
  },

  // Form state setters
  setName: (name: string) => {
    clientDetailDialogStore.name = name;
  },

  setDescription: (description: string) => {
    clientDetailDialogStore.description = description;
  },

  setDomains: (domains: string[]) => {
    clientDetailDialogStore.domains = domains;
  },

  setNewDomain: (domain: string) => {
    clientDetailDialogStore.newDomain = domain;
  },

  setConfig: (config: string) => {
    clientDetailDialogStore.config = config;
  },

  // Registration settings
  setRegistrationEnabled: (enabled: boolean) => {
    clientDetailDialogStore.registrationEnabled = enabled;
  },

  setRequireInviteCode: (require: boolean) => {
    clientDetailDialogStore.requireInviteCode = require;
  },

  setInviteCode: (code: string) => {
    clientDetailDialogStore.inviteCode = code;
  },

  // Domain management
  addDomain: (domain: string) => {
    if (domain && !clientDetailDialogStore.domains.includes(domain)) {
      clientDetailDialogStore.domains = [...clientDetailDialogStore.domains, domain];
      clientDetailDialogStore.newDomain = '';
    }
  },

  removeDomain: (domain: string) => {
    clientDetailDialogStore.domains = clientDetailDialogStore.domains.filter(d => d !== domain);
  },

  // Reset state
  reset: () => {
    clientDetailDialogStore.activeTab = 'details';
    clientDetailDialogStore.loading = false;
    clientDetailDialogStore.error = null;
    clientDetailDialogStore.name = '';
    clientDetailDialogStore.description = '';
    clientDetailDialogStore.domains = [];
    clientDetailDialogStore.newDomain = '';
    clientDetailDialogStore.config = '{}';
    clientDetailDialogStore.registrationEnabled = false;
    clientDetailDialogStore.requireInviteCode = false;
    clientDetailDialogStore.inviteCode = '';
  },
};