import { proxy } from 'valtio'

interface FormData {
  name: string
  description: string
}

export const addClientDialogStore = proxy({
  isOpen: false,
  formData: { name: "", description: "" } as FormData,
  isSubmitting: false,
  error: null as string | null,
  loading: false,
  domainsInput: "",
})

export const addClientDialogActions = {
  open: () => {
    addClientDialogStore.isOpen = true
    addClientDialogStore.formData = { name: "", description: "" }
    addClientDialogStore.error = null
  },

  close: () => {
    addClientDialogStore.isOpen = false
    addClientDialogStore.formData = { name: "", description: "" }
    addClientDialogStore.error = null
  },

  updateFormData: (updates: Partial<FormData>) => {
    Object.assign(addClientDialogStore.formData, updates)
  },

  setSubmitting: (isSubmitting: boolean) => {
    addClientDialogStore.isSubmitting = isSubmitting
  },

  setError: (error: string | null) => {
    addClientDialogStore.error = error
  },

  setLoading: (loading: boolean) => {
    addClientDialogStore.loading = loading
  },

  setDomainsInput: (domains: string) => {
    addClientDialogStore.domainsInput = domains
  },

  resetForm: () => {
    addClientDialogStore.formData = { name: "", description: "" }
    addClientDialogStore.domainsInput = ""
    addClientDialogStore.error = null
    addClientDialogStore.loading = false
  },
}