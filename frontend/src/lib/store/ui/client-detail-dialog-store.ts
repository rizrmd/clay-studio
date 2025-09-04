import { proxy } from 'valtio'
import type { ClientRootResponse } from '@/lib/services/root-service'

export const clientDetailDialogStore = proxy({
  isOpen: false,
  client: null as ClientRootResponse | null,
  activeTab: 'general' as string,
  name: '',
  description: '',
  loading: false,
  newDomain: '',
  domains: [] as string[],
  registrationEnabled: false,
  requireInviteCode: false,
  inviteCode: '',
  config: '',
  error: null as string | null,
})

export const clientDetailDialogActions = {
  open: (client: ClientRootResponse) => {
    clientDetailDialogStore.isOpen = true
    clientDetailDialogStore.client = client
    clientDetailDialogStore.name = client.name
    clientDetailDialogStore.description = client.description || ''
    clientDetailDialogStore.domains = [...(client.domains || [])]
    clientDetailDialogStore.registrationEnabled = client.registrationEnabled || false
    clientDetailDialogStore.requireInviteCode = client.requireInviteCode || false
    clientDetailDialogStore.inviteCode = client.inviteCode || ''
    clientDetailDialogStore.config = JSON.stringify(client.config || {}, null, 2)
    clientDetailDialogStore.activeTab = 'general'
  },

  close: () => {
    clientDetailDialogStore.isOpen = false
    clientDetailDialogStore.client = null
    clientDetailDialogStore.name = ''
    clientDetailDialogStore.description = ''
    clientDetailDialogStore.loading = false
    clientDetailDialogStore.newDomain = ''
    clientDetailDialogStore.domains = []
    clientDetailDialogStore.registrationEnabled = false
    clientDetailDialogStore.requireInviteCode = false
    clientDetailDialogStore.inviteCode = ''
    clientDetailDialogStore.config = ''
    clientDetailDialogStore.activeTab = 'general'
  },

  setActiveTab: (tab: string) => {
    clientDetailDialogStore.activeTab = tab
  },

  setName: (name: string) => {
    clientDetailDialogStore.name = name
  },

  setDescription: (description: string) => {
    clientDetailDialogStore.description = description
  },

  setLoading: (loading: boolean) => {
    clientDetailDialogStore.loading = loading
  },

  setNewDomain: (domain: string) => {
    clientDetailDialogStore.newDomain = domain
  },

  addDomain: (domain?: string) => {
    const domainToAdd = domain || clientDetailDialogStore.newDomain
    if (domainToAdd.trim()) {
      clientDetailDialogStore.domains.push(domainToAdd.trim())
      clientDetailDialogStore.newDomain = ''
    }
  },

  removeDomain: (domain: string) => {
    const index = clientDetailDialogStore.domains.indexOf(domain)
    if (index > -1) {
      clientDetailDialogStore.domains.splice(index, 1)
    }
  },

  setRegistrationEnabled: (enabled: boolean) => {
    clientDetailDialogStore.registrationEnabled = enabled
  },

  setRequireInviteCode: (required: boolean) => {
    clientDetailDialogStore.requireInviteCode = required
  },

  setInviteCode: (code: string) => {
    clientDetailDialogStore.inviteCode = code
  },

  setConfig: (config: string) => {
    clientDetailDialogStore.config = config
  },

  setError: (error: string | null) => {
    clientDetailDialogStore.error = error
  },

  initializeFromClient: (client: ClientRootResponse) => {
    clientDetailDialogStore.name = client.name
    clientDetailDialogStore.description = client.description || ''
    clientDetailDialogStore.domains = [...(client.domains || [])]
    clientDetailDialogStore.registrationEnabled = client.registrationEnabled || false
    clientDetailDialogStore.requireInviteCode = client.requireInviteCode || false
    clientDetailDialogStore.inviteCode = client.inviteCode || ''
    clientDetailDialogStore.config = JSON.stringify(client.config || {}, null, 2)
  },
}