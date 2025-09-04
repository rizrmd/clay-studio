import { useSnapshot } from 'valtio'
import { CreateClientRequest, rootService } from '@/lib/services/root-service'
import { addClientDialogStore, addClientDialogActions } from '@/lib/store/add-client-dialog-store'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { AlertCircle, Plus } from 'lucide-react'

interface AddClientDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onSuccess: () => void
}

export function AddClientDialog({ open, onOpenChange, onSuccess }: AddClientDialogProps) {
  const dialogSnapshot = useSnapshot(addClientDialogStore)

  const handleSubmit = async () => {
    if (!dialogSnapshot.formData.name.trim()) {
      addClientDialogActions.setError('Client name is required')
      return
    }

    try {
      addClientDialogActions.setLoading(true)
      addClientDialogActions.setError(null)

      // Parse domains from comma-separated input
      const domains = dialogSnapshot.domainsInput
        .split(',')
        .map((d: string) => d.trim())
        .filter((d: string) => d.length > 0)

      const requestData: CreateClientRequest = {
        ...dialogSnapshot.formData,
        domains: domains.length > 0 ? domains : undefined
      }

      await rootService.createClient(requestData)

      // Reset form
      addClientDialogActions.resetForm()

      // Close dialog and refresh
      onOpenChange(false)
      onSuccess()
    } catch (err: any) {
      addClientDialogActions.setError(err.response?.data?.error || 'Failed to create client')
    } finally {
      addClientDialogActions.setLoading(false)
    }
  }

  const handleClose = () => {
    // Reset form when closing
    addClientDialogActions.resetForm()
    onOpenChange(false)
  }

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-[525px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Plus className="h-5 w-5" />
            Add New Client
          </DialogTitle>
          <DialogDescription>
            Create a new client for the Clay Studio system. Clients can manage their own users, projects, and conversations.
          </DialogDescription>
        </DialogHeader>
        
        {dialogSnapshot.error && (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{dialogSnapshot.error}</AlertDescription>
          </Alert>
        )}
        
        <div className="grid gap-4 py-4">
          <div className="grid gap-2">
            <Label htmlFor="name">Client Name *</Label>
            <Input
              id="name"
              placeholder="e.g., Acme Corporation"
              value={dialogSnapshot.formData.name}
              onChange={(e) => addClientDialogActions.updateFormData({ name: e.target.value })}
              disabled={dialogSnapshot.loading}
            />
          </div>
          
          <div className="grid gap-2">
            <Label htmlFor="description">Description</Label>
            <Textarea
              id="description"
              placeholder="Brief description of the client..."
              value={dialogSnapshot.formData.description || ''}
              onChange={(e) => addClientDialogActions.updateFormData({ description: e.target.value })}
              disabled={dialogSnapshot.loading}
              rows={3}
            />
          </div>
          
          <div className="grid gap-2">
            <Label htmlFor="domains">
              Allowed Domains
              <span className="text-xs text-muted-foreground ml-2">(comma-separated)</span>
            </Label>
            <Input
              id="domains"
              placeholder="e.g., example.com, app.example.com"
              value={dialogSnapshot.domainsInput}
              onChange={(e) => addClientDialogActions.setDomainsInput(e.target.value)}
              disabled={dialogSnapshot.loading}
            />
            <p className="text-xs text-muted-foreground">
              Optional: Restrict client access to specific domains
            </p>
          </div>
        </div>
        
        <DialogFooter>
          <Button variant="outline" onClick={handleClose} disabled={dialogSnapshot.loading}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={dialogSnapshot.loading || !dialogSnapshot.formData.name.trim()}>
            {dialogSnapshot.loading ? 'Creating...' : 'Create Client'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}