import { useState } from 'react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Plus } from 'lucide-react'
import { validateDomain } from './domain-utils'

interface DomainInputProps {
  onAdd: (domain: string) => void
  disabled?: boolean
  placeholder?: string
  existingDomains?: string[]
  onError?: (error: string | null) => void
}

export function DomainInput({
  onAdd,
  disabled = false,
  placeholder = "example.com or localhost:3000",
  existingDomains = [],
  onError
}: DomainInputProps) {
  const [newDomain, setNewDomain] = useState('')

  const handleAddDomain = () => {
    const trimmedDomain = newDomain.trim().toLowerCase()
    
    if (!trimmedDomain) {
      onError?.('Please enter a domain')
      return
    }

    if (!validateDomain(trimmedDomain)) {
      onError?.('Please enter a valid domain (e.g., example.com, subdomain.example.com, or localhost:3000)')
      return
    }

    if (existingDomains.includes(trimmedDomain)) {
      onError?.('This domain is already added')
      return
    }

    onAdd(trimmedDomain)
    setNewDomain('')
    onError?.(null)
  }

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault()
      handleAddDomain()
    }
  }

  return (
    <div className="flex gap-2">
      <Input
        placeholder={placeholder}
        value={newDomain}
        onChange={(e) => setNewDomain(e.target.value)}
        onKeyPress={handleKeyPress}
        disabled={disabled}
        className="h-9"
      />
      <Button
        onClick={handleAddDomain}
        disabled={disabled || !newDomain.trim()}
        size="sm"
        className="h-9"
      >
        <Plus className="h-4 w-4" />
        <span className="ml-1.5">Add</span>
      </Button>
    </div>
  )
}