import { useState, useEffect } from 'react'
import { Button } from '@/components/ui/button'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Save, RefreshCw } from 'lucide-react'
import { DomainList } from './domain-list'
import { DomainInput } from './domain-input'
import { hasDomainsChanged } from './domain-utils'

interface DomainEditorProps {
  initialDomains?: string[]
  onSave?: (domains: string[]) => Promise<void>
  onChange?: (domains: string[]) => void
  disabled?: boolean
  showSaveButtons?: boolean
  emptyMessage?: string
  showCount?: boolean
}

export function DomainEditor({
  initialDomains = [],
  onSave,
  onChange,
  disabled = false,
  showSaveButtons = true,
  emptyMessage = "No domains configured",
  showCount = true
}: DomainEditorProps) {
  const [domains, setDomains] = useState<string[]>(initialDomains)
  const [originalDomains, setOriginalDomains] = useState<string[]>(initialDomains)
  const [isSaving, setIsSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Update domains when initialDomains prop changes
  useEffect(() => {
    setDomains(initialDomains)
    setOriginalDomains(initialDomains)
  }, [initialDomains])

  const handleAddDomain = (domain: string) => {
    const newDomains = [...domains, domain]
    setDomains(newDomains)
    onChange?.(newDomains)
  }

  const handleRemoveDomain = (domainToRemove: string) => {
    const newDomains = domains.filter(d => d !== domainToRemove)
    setDomains(newDomains)
    onChange?.(newDomains)
  }

  const handleSave = async () => {
    if (!onSave) return
    
    try {
      setIsSaving(true)
      setError(null)
      await onSave(domains)
      setOriginalDomains(domains)
    } catch (err: any) {
      setError(err.message || 'Failed to save domains')
    } finally {
      setIsSaving(false)
    }
  }

  const handleCancel = () => {
    setDomains(originalDomains)
    onChange?.(originalDomains)
    setError(null)
  }

  const hasChanges = hasDomainsChanged(domains, originalDomains)

  return (
    <div className="space-y-3">
      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <DomainInput
        onAdd={handleAddDomain}
        disabled={disabled || isSaving}
        existingDomains={domains}
        onError={setError}
      />

      <DomainList
        domains={domains}
        onRemove={handleRemoveDomain}
        disabled={disabled || isSaving}
        emptyMessage={emptyMessage}
        showCount={showCount}
      />

      {showSaveButtons && onSave && hasChanges && (
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
    </div>
  )
}