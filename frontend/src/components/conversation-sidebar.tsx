import { ChevronDown, Plus, PanelLeftClose, PanelLeftOpen } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'

interface ConversationSidebarProps {
  isCollapsed: boolean
  onToggle: () => void
}

export function ConversationSidebar({ isCollapsed, onToggle }: ConversationSidebarProps) {
  return (
    <div className={`${isCollapsed ? 'w-12' : 'w-64'} border-r bg-background flex flex-col transition-all duration-300`}>
      {/* Header with toggle and new chat */}
      <div className="p-3 border-b">
        <div className="flex items-center justify-between">
          <Button
            variant="ghost"
            size="sm"
            onClick={onToggle}
            className="h-8 w-8 p-0"
          >
            {isCollapsed ? (
              <PanelLeftOpen className="h-4 w-4" />
            ) : (
              <PanelLeftClose className="h-4 w-4" />
            )}
          </Button>
          
          {!isCollapsed && (
            <Button
              variant="ghost"
              size="sm"
              className="h-8 gap-1 text-sm"
            >
              <Plus className="h-4 w-4" />
              New Chat
            </Button>
          )}
        </div>
      </div>
      
      {/* Conversations area */}
      {!isCollapsed && (
        <div className="flex-1 p-4">
          <p className="text-sm text-muted-foreground">
            Your conversations will appear here once you start chatting!
          </p>
        </div>
      )}
      
      {/* Bottom user section */}
      <div className="border-t p-3">
        {isCollapsed ? (
          <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
            <div className="h-5 w-5 rounded-full bg-green-500 flex items-center justify-center">
              <div className="h-1.5 w-1.5 rounded-full bg-white" />
            </div>
          </Button>
        ) : (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" className="w-full justify-between p-2">
                <div className="flex items-center gap-2">
                  <div className="h-6 w-6 rounded-full bg-green-500 flex items-center justify-center">
                    <div className="h-2 w-2 rounded-full bg-white" />
                  </div>
                  <span className="text-sm">Guest</span>
                </div>
                <ChevronDown className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start" className="w-56">
              <DropdownMenuItem>Profile</DropdownMenuItem>
              <DropdownMenuItem>Settings</DropdownMenuItem>
              <DropdownMenuItem>Sign out</DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        )}
      </div>
    </div>
  )
}