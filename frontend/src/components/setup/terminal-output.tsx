import { useEffect, useRef } from 'react'

interface TerminalOutputProps {
  cliOutput: string[]
  setupProgress: string
}

export function TerminalOutput({ cliOutput, setupProgress }: TerminalOutputProps) {
  const terminalRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight
    }
  }, [cliOutput, setupProgress])

  return (
    <div ref={terminalRef} className="bg-black text-green-400 p-4 rounded-lg font-mono text-xs overflow-auto max-h-96">
      {cliOutput.length > 0 ? (
        cliOutput.map((line, index) => (
          <div key={index} className="mb-1">
            <span className="text-gray-500 mr-2">[{index + 1}]</span>
            {line}
          </div>
        ))
      ) : (
        <div className="text-gray-500">
          Waiting for output from Claude CLI...
        </div>
      )}
      {setupProgress && (
        <div className="mt-2 text-yellow-400">
          → {setupProgress}
        </div>
      )}
    </div>
  )
}