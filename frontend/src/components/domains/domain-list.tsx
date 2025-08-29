import { Badge } from '@/components/ui/badge'
import { X } from 'lucide-react'

interface DomainListProps {
  domains: string[]
  onRemove?: (domain: string) => void
  disabled?: boolean
  emptyMessage?: string
  showCount?: boolean
}

export function DomainList({
  domains,
  onRemove,
  disabled = false,
  emptyMessage = "No domains configured",
  showCount = true
}: DomainListProps) {
  if (domains.length === 0) {
    return (
      <div className="text-center py-4 text-muted-foreground border border-dashed rounded">
        <p className="text-sm">{emptyMessage}</p>
      </div>
    )
  }

  return (
    <div className="space-y-2">
      {showCount && (
        <p className="text-sm text-muted-foreground mb-2">
          {domains.length} domain{domains.length !== 1 ? 's' : ''} configured
        </p>
      )}
      <div className="flex flex-wrap gap-2">
        {domains.map((domain) => (
          <Badge 
            key={domain} 
            variant="secondary" 
            className="pl-3 pr-1 py-1.5 text-sm"
          >
            <span className="mr-2">{domain}</span>
            {onRemove && (
              <button
                onClick={() => onRemove(domain)}
                disabled={disabled}
                className="ml-1 hover:bg-destructive/20 rounded p-0.5 transition-colors"
                aria-label={`Remove ${domain}`}
              >
                <X className="h-3.5 w-3.5" />
              </button>
            )}
          </Badge>
        ))}
      </div>
    </div>
  )
}