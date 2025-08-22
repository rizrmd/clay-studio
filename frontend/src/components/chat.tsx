import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Messages } from './messages'
import { MultimodalInput } from './multimodal-input'
import { useClayChat } from '@/hooks/useClayChat'
import { Settings } from 'lucide-react'

const PROJECT_ID = '6c14f284-44c3-4f78-8d2e-85cd3facb259'

export function Chat() {
  const [input, setInput] = useState('')
  const navigate = useNavigate()
  
  const {
    messages,
    sendMessage,
    isLoading,
    error,
    hasDataSources,
    canUseAdvancedAnalysis,
    projectContext
  } = useClayChat(PROJECT_ID)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!input.trim() || isLoading) return

    const messageContent = input.trim()
    setInput('')
    await sendMessage(messageContent)
  }

  const stop = () => {
    // TODO: Implement stop functionality for streaming
  }

  return (
    <div className="group w-full overflow-auto pl-0 peer-[[data-state=open]]:lg:pl-[250px] peer-[[data-state=open]]:xl:pl-[300px]">

      {/* Context indicators */}
      {projectContext && (
        <div className="border-b bg-muted/50 px-4 py-2">
          <div className="mx-auto max-w-2xl">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2 text-sm">
                <span className="font-medium">{projectContext.project_settings.name}</span>
                {hasDataSources && (
                  <span className="text-muted-foreground">
                    • {projectContext.data_sources.length} data sources
                  </span>
                )}
                {canUseAdvancedAnalysis && (
                  <span className="text-green-600">• AI Analysis Ready</span>
                )}
              </div>
              <button
                onClick={() => navigate('/auth')}
                className="p-1.5 rounded-lg hover:bg-gray-100 transition-colors"
                title="Clay Authentication Settings"
              >
                <Settings className="h-4 w-4 text-gray-600" />
              </button>
            </div>
          </div>
        </div>
      )}

      <div className="pb-[200px] pt-4 md:pt-10">
        <div className="mx-auto max-w-2xl px-4">
          {/* Error display */}
          {error && (
            <div className="mb-4 rounded-lg border border-red-200 bg-red-50 p-4 text-red-800">
              <p className="text-sm">{error}</p>
            </div>
          )}
          
          <Messages 
            messages={messages.map(msg => ({
              ...msg,
              createdAt: msg.createdAt ? new Date(msg.createdAt) : new Date()
            }))} 
            isLoading={isLoading} 
          />
        </div>
      </div>
      <div className="fixed inset-x-0 bottom-0 w-full bg-gradient-to-b from-muted/30 from-0% to-muted/30 to-50% duration-300 ease-in-out animate-in dark:from-background/10 dark:from-10% dark:to-background/80 peer-[[data-state=open]]:group-[]:lg:pl-[250px] peer-[[data-state=open]]:group-[]:xl:pl-[300px]">
        <div className="mx-auto max-w-2xl px-4">
          <MultimodalInput
            input={input}
            setInput={setInput}
            handleSubmit={handleSubmit}
            isLoading={isLoading}
            stop={stop}
          />
        </div>
      </div>
    </div>
  )
}