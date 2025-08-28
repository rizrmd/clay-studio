import { useState } from 'react'
import { CreateClientRequest, rootService } from '@/services/root-service'
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
  const [formData, setFormData] = useState<CreateClientRequest>({
    name: '',
    description: '',
    domains: []
  })
  const [domainsInput, setDomainsInput] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const handleSubmit = async () => {
    if (!formData.name.trim()) {
      setError('Client name is required')
      return
    }

    try {
      setLoading(true)
      setError(null)
      
      // Parse domains from comma-separated input
      const domains = domainsInput
        .split(',')
        .map(d => d.trim())
        .filter(d => d.length > 0)
      
      const requestData: CreateClientRequest = {
        ...formData,
        domains: domains.length > 0 ? domains : undefined
      }
      
      await rootService.createClient(requestData)
      
      // Reset form
      setFormData({ name: '', description: '', domains: [] })
      setDomainsInput('')
      
      // Close dialog and refresh
      onOpenChange(false)
      onSuccess()
    } catch (err: any) {
      setError(err.response?.data?.error || 'Failed to create client')
    } finally {
      setLoading(false)
    }
  }

  const handleClose = () => {
    // Reset form when closing
    setFormData({ name: '', description: '', domains: [] })
    setDomainsInput('')
    setError(null)
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
        
        {error && (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}
        
        <div className="grid gap-4 py-4">
          <div className="grid gap-2">
            <Label htmlFor="name">Client Name *</Label>
            <Input
              id="name"
              placeholder="e.g., Acme Corporation"
              value={formData.name}
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              disabled={loading}
            />
          </div>
          
          <div className="grid gap-2">
            <Label htmlFor="description">Description</Label>
            <Textarea
              id="description"
              placeholder="Brief description of the client..."
              value={formData.description || ''}
              onChange={(e) => setFormData({ ...formData, description: e.target.value })}
              disabled={loading}
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
              value={domainsInput}
              onChange={(e) => setDomainsInput(e.target.value)}
              disabled={loading}
            />
            <p className="text-xs text-muted-foreground">
              Optional: Restrict client access to specific domains
            </p>
          </div>
        </div>
        
        <DialogFooter>
          <Button variant="outline" onClick={handleClose} disabled={loading}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={loading || !formData.name.trim()}>
            {loading ? 'Creating...' : 'Create Client'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}