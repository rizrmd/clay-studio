import { useState } from 'react'
import { Paperclip, ArrowUp } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'

interface ChatInputProps {
  onSendMessage?: (message: string) => void
}

export function ChatInput({ onSendMessage }: ChatInputProps) {
  const [message, setMessage] = useState('')

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (message.trim() && onSendMessage) {
      onSendMessage(message.trim())
      setMessage('')
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSubmit(e)
    }
  }

  return (
    <div className="border-t bg-background p-4">
      <form onSubmit={handleSubmit} className="max-w-4xl mx-auto">
        <div className="relative">
          <Textarea
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Send a message..."
            className="min-h-[60px] max-h-[200px] resize-none pr-20 pl-12"
            rows={1}
          />
          
          {/* Attach button */}
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="absolute left-3 top-3 h-8 w-8 p-0"
          >
            <Paperclip className="h-4 w-4" />
          </Button>
          
          {/* Send button */}
          <Button
            type="submit"
            size="sm"
            className="absolute right-3 top-3 h-8 w-8 p-0 rounded-full"
            disabled={!message.trim()}
          >
            <ArrowUp className="h-4 w-4" />
          </Button>
        </div>
      </form>
    </div>
  )
}