import { useEffect, useRef, memo, useMemo, useState } from 'react'
import { Bot, User, MoreVertical, Trash2 } from 'lucide-react'
import { cn } from '@/lib/utils'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import rehypeHighlight from 'rehype-highlight'
import rehypeRaw from 'rehype-raw'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Button } from '@/components/ui/button'

interface Message {
  id: string
  content: string
  role: 'user' | 'assistant' | 'system'
  createdAt: Date
}

interface MessagesProps {
  messages: Message[]
  isLoading?: boolean
  onForgetFrom?: (messageId: string) => void
}

// Memoized individual message component
const MessageItem = memo(({ message, onForgetFrom }: { message: Message; onForgetFrom?: (messageId: string) => void }) => {
  return (
    <div
      className={cn(
        'flex gap-3 max-w-2xl mb-6 relative group',
        message.role === 'user' ? 'ml-auto flex-row-reverse' : 'mr-auto'
      )}
    >
      <div className="flex h-8 w-8 shrink-0 select-none items-center justify-center rounded-md bg-muted">
        {message.role === 'user' ? (
          <User className="h-4 w-4" />
        ) : (
          <Bot className="h-4 w-4" />
        )}
      </div>
      <div className="flex flex-col gap-1 flex-1">
        <div>
          <div
            className={cn(
              'rounded-lg p-3 text-sm',
              message.role === 'user'
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted'
            )}
          >
            <div className="prose prose-sm max-w-none dark:prose-invert">
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                rehypePlugins={[rehypeHighlight, rehypeRaw]}
              >
                {message.content}
              </ReactMarkdown>
            </div>
          </div>
        </div>
        <div className="text-xs text-muted-foreground">
          {message.createdAt.toLocaleTimeString([], { 
            hour: '2-digit', 
            minute: '2-digit' 
          })}
        </div>
      </div>
      {onForgetFrom && (
        <div className="absolute -right-10 top-0 opacity-0 group-hover:opacity-100 transition-opacity">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
                <MoreVertical className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                onClick={() => onForgetFrom(message.id)}
                className="text-destructive"
              >
                <Trash2 className="mr-2 h-4 w-4" />
                Forget after this
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      )}
    </div>
  )
})

export function Messages({ messages, isLoading, onForgetFrom }: MessagesProps) {
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const [visibleRange, setVisibleRange] = useState({ start: 0, end: messages.length })

  // For performance with large message counts, only render recent messages
  const visibleMessages = useMemo(() => {
    if (messages.length <= 20) {
      return messages // Render all messages if count is reasonable
    }
    
    // For large lists, show last 15 messages + some buffer for scrolling
    const startIndex = Math.max(0, messages.length - 20)
    return messages.slice(startIndex)
  }, [messages])

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages, isLoading])

  return (
    <div className="flex flex-col flex-1">
      {messages.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center text-center">
          <div className="flex h-20 w-20 items-center justify-center rounded-full bg-muted">
            <Bot className="h-10 w-10" />
          </div>
          <h2 className="mt-4 text-xl font-semibold">Welcome to Clay Studio</h2>
          <p className="mt-2 text-muted-foreground">
            I'm here to help you analyze your data. What would you like to explore?
          </p>
        </div>
      ) : (
        <div className="flex flex-col gap-6 p-4">
          {/* Show indicator if messages are truncated */}
          {messages.length > 20 && (
            <div className="text-center py-2 text-sm text-muted-foreground border-b border-muted">
              Showing recent {visibleMessages.length} of {messages.length} messages
            </div>
          )}
          
          {visibleMessages.map((message) => (
            <MessageItem
              key={message.id}
              message={message}
              onForgetFrom={onForgetFrom}
            />
          ))}
          
          {isLoading && (
            <div className="flex gap-3 max-w-2xl mr-auto">
              <div className="flex h-8 w-8 shrink-0 select-none items-center justify-center rounded-md bg-muted">
                <Bot className="h-4 w-4" />
              </div>
              <div className="flex flex-col gap-1 flex-1">
                <div className="rounded-lg p-3 text-sm bg-muted">
                  <div className="flex items-center space-x-1">
                    <div className="h-2 w-2 animate-bounce rounded-full bg-muted-foreground [animation-delay:-0.3s]"></div>
                    <div className="h-2 w-2 animate-bounce rounded-full bg-muted-foreground [animation-delay:-0.15s]"></div>
                    <div className="h-2 w-2 animate-bounce rounded-full bg-muted-foreground"></div>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      )}
      <div ref={messagesEndRef} />
    </div>
  )
}