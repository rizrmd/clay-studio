import { Link, useLocation } from 'react-router-dom'
import { Home, User, Settings, LogOut } from 'lucide-react'
import { useValtioAuth } from '@/hooks/use-valtio-auth'
import { navigationMenuTriggerStyle } from '@/components/ui/navigation-menu'
import { cn } from '@/lib/utils'

export function AppHeader() {
  const { user, logout } = useValtioAuth()
  const location = useLocation()

  const isActive = (path: string) => {
    return location.pathname === path
  }

  const clientName = user?.username || 'Clay Studio'

  return (
    <div className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-16">
          <Link
            to="/"
            className="text-xl font-semibold text-gray-900 dark:text-gray-100 hover:text-gray-700 dark:hover:text-gray-300 transition-colors"
          >
            {clientName}
          </Link>

          <div className="flex items-center gap-4">
            <Link
              to="/projects"
              className={cn(
                navigationMenuTriggerStyle(),
                "h-9 px-3",
                isActive('/projects') && "bg-accent"
              )}
            >
              <Home className="h-4 w-4 mr-2" />
              Projects
            </Link>
            <Link
              to="/profile"
              className={cn(
                navigationMenuTriggerStyle(),
                "h-9 px-3",
                isActive('/profile') && "bg-accent"
              )}
            >
              <User className="h-4 w-4 mr-2" />
              Profile
            </Link>
            {user?.role === 'admin' && (
              <Link
                to="/config"
                className={cn(
                  navigationMenuTriggerStyle(),
                  "h-9 px-3",
                  isActive('/config') && "bg-accent"
                )}
              >
                <Settings className="h-4 w-4 mr-2" />
                Settings
              </Link>
            )}
            <button
              onClick={logout}
              className={cn(
                navigationMenuTriggerStyle(),
                "h-9 px-3 hover:bg-destructive/10 hover:text-destructive"
              )}
            >
              <LogOut className="h-4 w-4 mr-2" />
              Logout
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}