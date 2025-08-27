import { Link, useLocation } from 'react-router-dom'
import { Home, User, Settings, LogOut } from 'lucide-react'
import { useValtioAuth } from '@/hooks/use-valtio-auth'

export function AppHeader() {
  const { user, logout } = useValtioAuth()
  const location = useLocation()

  const isActive = (path: string) => {
    return location.pathname === path
  }

  return (
    <div className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-16">
          <div className="flex items-center">
            <h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100">
              Clay Studio
            </h2>
          </div>
          <div className="flex items-center gap-4">
            {user && (
              <span className="text-sm text-gray-600 dark:text-gray-400">
                {user.username}
              </span>
            )}
            <Link
              to="/projects"
              className={`p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors ${
                isActive('/projects') ? 'bg-gray-100 dark:bg-gray-700' : ''
              }`}
              title="Projects"
            >
              <Home className="h-5 w-5 text-gray-600 dark:text-gray-400" />
            </Link>
            <Link
              to="/profile"
              className={`p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors ${
                isActive('/profile') ? 'bg-gray-100 dark:bg-gray-700' : ''
              }`}
              title="Profile"
            >
              <User className="h-5 w-5 text-gray-600 dark:text-gray-400" />
            </Link>
            {user?.role === 'admin' && (
              <Link
                to="/config"
                className={`p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors ${
                  isActive('/config') ? 'bg-gray-100 dark:bg-gray-700' : ''
                }`}
                title="Settings"
              >
                <Settings className="h-5 w-5 text-gray-600 dark:text-gray-400" />
              </Link>
            )}
            <button
              onClick={logout}
              className="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
              title="Logout"
            >
              <LogOut className="h-5 w-5 text-gray-600 dark:text-gray-400" />
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}