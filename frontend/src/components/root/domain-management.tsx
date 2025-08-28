import { useState, useEffect } from 'react'
import { rootService } from '@/services/root-service'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { 
  Globe,
  Plus,
  X,
  Save,
  RefreshCw
} from 'lucide-react'
import { Alert, AlertDescription } from '@/components/ui/alert'

interface DomainManagementProps {
  clientId: string
  initialDomains: string[]
  onUpdate: () => void
}

export function DomainManagement({ 
  clientId, 
  initialDomains = [], 
  onUpdate 
}: DomainManagementProps) {
  const [domains, setDomains] = useState<string[]>(initialDomains)
  const [newDomain, setNewDomain] = useState('')
  const [isSaving, setIsSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [originalDomains, setOriginalDomains] = useState<string[]>(initialDomains)

  // Update domains when initialDomains prop changes
  useEffect(() => {
    setDomains(initialDomains)
    setOriginalDomains(initialDomains)
  }, [initialDomains])

  const validateDomain = (domain: string) => {
    // Basic domain validation
    const domainRegex = /^([a-z0-9]+(-[a-z0-9]+)*\.)+[a-z]{2,}$/i
    const ipRegex = /^(\d{1,3}\.){3}\d{1,3}(:\d{1,5})?$/
    const localhostRegex = /^localhost(:\d{1,5})?$/i
    
    return domainRegex.test(domain) || ipRegex.test(domain) || localhostRegex.test(domain)
  }

  const handleAddDomain = () => {
    const trimmedDomain = newDomain.trim().toLowerCase()
    
    if (!trimmedDomain) {
      setError('Please enter a domain')
      return
    }

    if (!validateDomain(trimmedDomain)) {
      setError('Please enter a valid domain (e.g., example.com, subdomain.example.com, or localhost:3000)')
      return
    }

    if (domains.includes(trimmedDomain)) {
      setError('This domain is already added')
      return
    }

    setDomains([...domains, trimmedDomain])
    setNewDomain('')
    setError(null)
  }

  const handleRemoveDomain = (domainToRemove: string) => {
    setDomains(domains.filter(d => d !== domainToRemove))
  }

  const handleSave = async () => {
    try {
      setIsSaving(true)
      setError(null)
      await rootService.updateClientDomains(clientId, domains)
      setOriginalDomains(domains)
      onUpdate()
    } catch (err: any) {
      setError(err.response?.data?.error || 'Failed to update domains')
    } finally {
      setIsSaving(false)
    }
  }

  const handleCancel = () => {
    setDomains(originalDomains)
    setNewDomain('')
    setError(null)
  }

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault()
      handleAddDomain()
    }
  }

  const hasChanges = JSON.stringify(domains.sort()) !== JSON.stringify(originalDomains.sort())

  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-base flex items-center gap-2">
          <Globe className="h-4 w-4" />
          Allowed Domains
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        {error && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        {/* Add Domain Input */}
        <div className="flex gap-2">
          <Input
            placeholder="example.com or localhost:3000"
            value={newDomain}
            onChange={(e) => setNewDomain(e.target.value)}
            onKeyPress={handleKeyPress}
            disabled={isSaving}
            className="h-9"
          />
          <Button
            onClick={handleAddDomain}
            disabled={isSaving || !newDomain.trim()}
            size="sm"
            className="h-9"
          >
            <Plus className="h-4 w-4" />
            <span className="ml-1.5">Add</span>
          </Button>
        </div>

        {/* Domain List */}
        <div className="space-y-2">
          {domains.length === 0 ? (
            <div className="text-center py-4 text-muted-foreground border border-dashed rounded">
              <p className="text-sm">No domains configured</p>
            </div>
          ) : (
            <div className="space-y-2">
              <p className="text-sm text-muted-foreground mb-2">
                {domains.length} domain{domains.length !== 1 ? 's' : ''} configured
              </p>
              <div className="flex flex-wrap gap-2">
                {domains.map((domain) => (
                  <Badge 
                    key={domain} 
                    variant="secondary" 
                    className="pl-3 pr-1 py-1.5 text-sm"
                  >
                    <span className="mr-2">{domain}</span>
                    <button
                      onClick={() => handleRemoveDomain(domain)}
                      disabled={isSaving}
                      className="ml-1 hover:bg-destructive/20 rounded p-0.5 transition-colors"
                      aria-label={`Remove ${domain}`}
                    >
                      <X className="h-3.5 w-3.5" />
                    </button>
                  </Badge>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Action Buttons */}
        {hasChanges && (
          <div className="flex justify-end gap-2 pt-2 border-t">
            <Button
              variant="outline"
              size="sm"
              onClick={handleCancel}
              disabled={isSaving}
            >
              Cancel
            </Button>
            <Button
              size="sm"
              onClick={handleSave}
              disabled={isSaving}
            >
              {isSaving ? (
                <>
                  <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                  Saving...
                </>
              ) : (
                <>
                  <Save className="h-4 w-4 mr-2" />
                  Save Changes
                </>
              )}
            </Button>
          </div>
        )}

      </CardContent>
    </Card>
  )
}