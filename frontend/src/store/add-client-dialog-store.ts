import { proxy } from 'valtio';
import { CreateClientRequest } from '@/lib/services/root-service';

interface AddClientDialogState {
  formData: CreateClientRequest;
  domainsInput: string;
  loading: boolean;
  error: string | null;
}

export const addClientDialogStore = proxy<AddClientDialogState>({
  formData: {
    name: '',
    description: '',
    domains: []
  },
  domainsInput: '',
  loading: false,
  error: null,
});

export const addClientDialogActions = {
  // Form data
  setFormData: (formData: CreateClientRequest) => {
    addClientDialogStore.formData = formData;
  },

  updateFormData: (updates: Partial<CreateClientRequest>) => {
    addClientDialogStore.formData = {
      ...addClientDialogStore.formData,
      ...updates,
    };
  },

  // Domains input
  setDomainsInput: (domainsInput: string) => {
    addClientDialogStore.domainsInput = domainsInput;
  },

  // Loading state
  setLoading: (loading: boolean) => {
    addClientDialogStore.loading = loading;
  },

  // Error state
  setError: (error: string | null) => {
    addClientDialogStore.error = error;
  },

  // Reset form
  resetForm: () => {
    addClientDialogStore.formData = {
      name: '',
      description: '',
      domains: []
    };
    addClientDialogStore.domainsInput = '';
    addClientDialogStore.error = null;
  },

  // Reset all state
  reset: () => {
    addClientDialogStore.formData = {
      name: '',
      description: '',
      domains: []
    };
    addClientDialogStore.domainsInput = '';
    addClientDialogStore.loading = false;
    addClientDialogStore.error = null;
  },
};