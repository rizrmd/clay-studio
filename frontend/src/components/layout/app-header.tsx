import { useState } from 'react'
import { Link, useLocation } from 'react-router-dom'
import { Home, User, Settings, LogOut, Menu, X } from 'lucide-react'
import { useValtioAuth } from '@/hooks/use-valtio-auth'
import { navigationMenuTriggerStyle } from '@/components/ui/navigation-menu'
import { cn } from '@/lib/utils'

export function AppHeader() {
  const { user, logout } = useValtioAuth()
  const location = useLocation()
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false)

  const isActive = (path: string) => {
    return location.pathname === path
  }

  const clientName = user?.username || 'Clay Studio'

  const handleLinkClick = () => {
    setIsMobileMenuOpen(false)
  }

  return (
    <div className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-16">
          <Link
            to="/"
            className="text-xl font-semibold text-gray-900 dark:text-gray-100 hover:text-gray-700 dark:hover:text-gray-300 transition-colors"
            onClick={handleLinkClick}
          >
            {clientName}
          </Link>

          {/* Desktop Menu */}
          <div className="hidden md:flex items-center gap-4">
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

          {/* Mobile Menu Button */}
          <button
            onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
            className="md:hidden p-2 rounded-md text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"
          >
            {isMobileMenuOpen ? (
              <X className="h-6 w-6" />
            ) : (
              <Menu className="h-6 w-6" />
            )}
          </button>
        </div>

        {/* Mobile Menu */}
        {isMobileMenuOpen && (
          <div className="md:hidden py-3 space-y-2 border-t border-gray-200 dark:border-gray-700">
            <Link
              to="/projects"
              onClick={handleLinkClick}
              className={cn(
                "block px-3 py-2 rounded-md text-base font-medium",
                isActive('/projects')
                  ? "bg-accent text-accent-foreground"
                  : "text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"
              )}
            >
              <div className="flex items-center">
                <Home className="h-4 w-4 mr-2" />
                Projects
              </div>
            </Link>
            <Link
              to="/profile"
              onClick={handleLinkClick}
              className={cn(
                "block px-3 py-2 rounded-md text-base font-medium",
                isActive('/profile')
                  ? "bg-accent text-accent-foreground"
                  : "text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"
              )}
            >
              <div className="flex items-center">
                <User className="h-4 w-4 mr-2" />
                Profile
              </div>
            </Link>
            {user?.role === 'admin' && (
              <Link
                to="/config"
                onClick={handleLinkClick}
                className={cn(
                  "block px-3 py-2 rounded-md text-base font-medium",
                  isActive('/config')
                    ? "bg-accent text-accent-foreground"
                    : "text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"
                )}
              >
                <div className="flex items-center">
                  <Settings className="h-4 w-4 mr-2" />
                  Settings
                </div>
              </Link>
            )}
            <button
              onClick={() => {
                logout()
                handleLinkClick()
              }}
              className="block w-full text-left px-3 py-2 rounded-md text-base font-medium text-gray-700 dark:text-gray-300 hover:bg-destructive/10 hover:text-destructive"
            >
              <div className="flex items-center">
                <LogOut className="h-4 w-4 mr-2" />
                Logout
              </div>
            </button>
          </div>
        )}
      </div>
    </div>
  )
}