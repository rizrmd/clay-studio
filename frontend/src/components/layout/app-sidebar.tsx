import { Plus } from 'lucide-react'

import {
  Sidebar,
  SidebarContent,
  SidebarHeader,
  SidebarFooter,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
} from '@/components/ui/sidebar'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { Button } from '@/components/ui/button'

const PROJECT_ID = '6c14f284-44c3-4f78-8d2e-85cd3facb259'

interface AppSidebarProps {
  user?: any
}

export function AppSidebar({ user }: AppSidebarProps) {
  return (
    <Sidebar className="group-data-[side=left]:border-r-0 w-64 md:w-64">
      <SidebarHeader className="px-3 py-4">
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton size="lg" asChild className="h-10 p-2">
              <a href="/" className="flex items-center gap-3">
                <div className="flex aspect-square size-8 items-center justify-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground">
                  <div className="size-4 font-bold">C</div>
                </div>
                <div className="hidden sm:grid flex-1 text-left text-sm leading-tight">
                  <span className="truncate font-semibold">Clay Studio</span>
                  <span className="truncate text-xs">AI Data Analysis</span>
                </div>
              </a>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>
      <SidebarContent className="px-3">
        <SidebarMenu>
          <SidebarMenuItem>
            <Tooltip>
              <TooltipTrigger asChild>
                <SidebarMenuButton asChild>
                  <Button
                    variant="outline"
                    className="w-full justify-start gap-2 px-3 py-2"
                    onClick={() => window.location.href = '/'}
                  >
                    <Plus className="size-4 flex-shrink-0" />
                    <span className="hidden sm:inline">New Chat</span>
                  </Button>
                </SidebarMenuButton>
              </TooltipTrigger>
              <TooltipContent side="right" className="sm:hidden">
                Start a new conversation
              </TooltipContent>
            </Tooltip>
          </SidebarMenuItem>
        </SidebarMenu>
        
        {/* Chat History Placeholder */}
        <div className="px-2 py-1 text-xs text-muted-foreground hidden sm:block">
          Recent Conversations
        </div>
        <div className="px-2 py-1 text-xs text-muted-foreground/70 hidden sm:block">
          No conversations yet
        </div>
      </SidebarContent>
      <SidebarFooter className="px-3 py-4">
        {user && (
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton size="lg" asChild>
                <div className="flex items-center gap-2 p-2">
                  <div className="flex aspect-square size-8 items-center justify-center rounded-lg bg-muted text-muted-foreground flex-shrink-0">
                    U
                  </div>
                  <div className="hidden sm:grid flex-1 text-left text-sm leading-tight">
                    <span className="truncate font-semibold">User</span>
                    <span className="truncate text-xs">Project: {PROJECT_ID.slice(0, 8)}...</span>
                  </div>
                </div>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        )}
      </SidebarFooter>
    </Sidebar>
  )
}