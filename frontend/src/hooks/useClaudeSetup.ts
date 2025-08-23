import { useState, useEffect } from 'react'

function stripAnsi(str: string): string {
  return str
    .replace(/\x1b\[[0-9;]*m/g, '')
    .replace(/\x1b\[[0-9]*[A-Za-z]/g, '')
    .replace(/\x1b\[[?][0-9]+[hl]/g, '')
    .replace(/\x1b\[[\d;]*[HfJ]/g, '')
    .replace(/\x1b\[\d*[ABCD]/g, '')
    .replace(/\x1b\[\d*K/g, '')
    .replace(/\r/g, '')
    .replace(/\[2J\[3J\[H/g, '')
    .replace(/\[\?25[lh]/g, '')
    .replace(/\[\?2004[lh]/g, '')
    .replace(/\[\?1004[lh]/g, '')
}

interface UseClaudeSetupProps {
  clientId?: string
  clientStatus?: string
}

export function useClaudeSetup({ clientId, clientStatus }: UseClaudeSetupProps) {
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState('')
  const [claudeSetupResponse, setClaudeSetupResponse] = useState<any>(null)
  const [setupProgress, setSetupProgress] = useState<string>('')
  const [cliOutput, setCliOutput] = useState<string[]>([])


  useEffect(() => {
    if (!clientId || !clientStatus || !['installing', 'pending'].includes(clientStatus)) {
      return
    }
    const eventSource = new EventSource(`/api/claude-sse?client_id=${clientId}`)
    

    const handleMessage = (message: string) => {
      const cleanMessage = stripAnsi(message)
      
      // Check for auth URL in the message
      if (message.includes('https://claude.ai/oauth/authorize')) {
        // Extract the URL from the message
        const urlMatch = message.match(/https:\/\/claude\.ai\/oauth\/authorize[^\s]+/)
        if (urlMatch) {
          const cleanUrl = stripAnsi(urlMatch[0])
          setClaudeSetupResponse((prev: any) => ({
            ...prev,
            auth_url: cleanUrl
          }))
          setSetupProgress('Authentication URL received. Click the link below to authenticate.')
        }
      } else if (message.includes('AUTH_URL:')) {
        const authUrl = message.replace('AUTH_URL: ', '').trim()
        const cleanUrl = stripAnsi(authUrl)
        setClaudeSetupResponse((prev: any) => ({
          ...prev,
          auth_url: cleanUrl
        }))
        setSetupProgress('Authentication URL received. Click the link below to authenticate.')
      } else {
        setSetupProgress(cleanMessage)
        if (cleanMessage.trim()) {
          setCliOutput((prev: string[]) => [...prev, cleanMessage])
        }
      }
    }
    
    eventSource.addEventListener('start', (event) => {
      setIsLoading(true)
      setError('')
      try {
        const data = JSON.parse(event.data)
        handleMessage(data.message)
      } catch {
        handleMessage(event.data)
      }
    })
    
    eventSource.addEventListener('progress', (event) => {
      let message = ''
      try {
        const data = JSON.parse(event.data)
        message = data.message || ''
      } catch {
        message = event.data
      }
      
      handleMessage(message)
    })
    
    eventSource.addEventListener('complete', () => {
      eventSource.close()
      setSetupProgress('Setup completed successfully! Please use the authentication URL above to get your token.')
      setIsLoading(false)
    })
    
    return () => {
      eventSource.close()
    }
  }, [clientId, clientStatus])

  const updateClaudeSetupResponse = (update: any) => {
    setClaudeSetupResponse((prev: any) => ({
      ...prev,
      ...update
    }))
  }

  return {
    isLoading,
    error,
    setError,
    claudeSetupResponse,
    setupProgress,
    cliOutput,
    updateClaudeSetupResponse
  }
}