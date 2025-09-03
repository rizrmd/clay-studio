import { useState } from 'react'
import { rootService } from '@/lib/services/root-service'
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
import { Alert, AlertDescription } from '@/components/ui/alert'
import { AlertCircle, Key, ExternalLink } from 'lucide-react'

interface SetupClaudeDialogProps {
  clientId: string
  clientName: string
  open: boolean
  onOpenChange: (open: boolean) => void
  onSuccess: () => void
}

export function SetupClaudeDialog({ clientId, clientName, open, onOpenChange, onSuccess }: SetupClaudeDialogProps) {
  const [claudeToken, setClaudeToken] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const handleSubmit = async () => {
    if (!claudeToken.trim()) {
      setError('Claude token is required')
      return
    }

    try {
      setLoading(true)
      setError(null)
      
      await rootService.setClaudeToken(clientId, claudeToken)
      
      // Reset form
      setClaudeToken('')
      
      // Close dialog and refresh
      onOpenChange(false)
      onSuccess()
    } catch (err: any) {
      setError(err.response?.data?.error || 'Failed to set Claude token')
    } finally {
      setLoading(false)
    }
  }

  const handleClose = () => {
    // Reset form when closing
    setClaudeToken('')
    setError(null)
    onOpenChange(false)
  }

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-[525px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Key className="h-5 w-5" />
            Setup Claude Instance
          </DialogTitle>
          <DialogDescription>
            Configure Claude access for <span className="font-medium">{clientName}</span>. This will enable AI features for this client.
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
            <Label htmlFor="token">Claude Token</Label>
            <Input
              id="token"
              type="password"
              placeholder="Enter Claude authentication token"
              value={claudeToken}
              onChange={(e) => setClaudeToken(e.target.value)}
              disabled={loading}
            />
            <p className="text-xs text-muted-foreground">
              The token will be securely processed and stored as an OAuth token
            </p>
          </div>
          
          <Alert>
            <AlertDescription className="text-sm">
              <div className="flex items-start gap-2">
                <ExternalLink className="h-4 w-4 mt-0.5 flex-shrink-0" />
                <div>
                  <p className="font-medium mb-1">How to get a Claude token:</p>
                  <ol className="list-decimal list-inside space-y-1 text-xs">
                    <li>Visit claude.ai and sign in to your account</li>
                    <li>Open browser Developer Tools (F12)</li>
                    <li>Go to Application/Storage → Cookies</li>
                    <li>Find and copy the sessionKey value</li>
                  </ol>
                </div>
              </div>
            </AlertDescription>
          </Alert>
        </div>
        
        <DialogFooter>
          <Button variant="outline" onClick={handleClose} disabled={loading}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={loading || !claudeToken.trim()}>
            {loading ? 'Setting up...' : 'Setup Claude'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}