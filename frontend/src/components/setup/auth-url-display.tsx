import { Button } from '@/components/ui/button'
import { ExternalLink, Copy, CheckCircle } from 'lucide-react'
import { useState } from 'react'

interface AuthUrlDisplayProps {
  authUrl: string
  onAuthOpened?: () => void
}

export function AuthUrlDisplay({ authUrl, onAuthOpened }: AuthUrlDisplayProps) {
  const [copied, setCopied] = useState(false)
  
  const handleCopy = () => {
    navigator.clipboard.writeText(authUrl)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }
  
  const handleOpenAuth = () => {
    window.open(authUrl, '_blank', 'noopener,noreferrer')
    onAuthOpened?.()
  }
  
  return (
    <div className="mt-6 p-6 bg-gradient-to-br from-blue-50 to-indigo-50 dark:from-blue-950/30 dark:to-indigo-950/30 rounded-lg border border-blue-200 dark:border-blue-800">
      <div className="flex items-start justify-between mb-4">
        <div>
          <h3 className="text-lg font-semibold mb-1">
            🅰️ Authentication Required
          </h3>
          <p className="text-sm text-muted-foreground">
            Click the button below to open Claude authentication in a new tab
          </p>
        </div>
      </div>
      
      <div className="flex gap-3">
        <Button 
          onClick={handleOpenAuth}
          size="lg"
          className="flex-1"
        >
          <ExternalLink className="mr-2 h-4 w-4" />
          Open Authentication Page
        </Button>
        
        <Button
          onClick={handleCopy}
          variant="outline"
          size="lg"
        >
          {copied ? (
            <CheckCircle className="h-4 w-4 text-green-600" />
          ) : (
            <Copy className="h-4 w-4" />
          )}
        </Button>
      </div>
      
      <details className="mt-4">
        <summary className="text-xs text-muted-foreground cursor-pointer hover:text-foreground">
          Show authentication URL
        </summary>
        <div className="mt-2 p-2 bg-white/50 dark:bg-black/20 rounded border border-gray-200 dark:border-gray-700">
          <code className="text-xs break-all">
            {authUrl}
          </code>
        </div>
      </details>
    </div>
  )
}