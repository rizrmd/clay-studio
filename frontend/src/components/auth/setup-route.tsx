import { useAuth } from '@/contexts/AuthContext'
import { AuthPage } from '@/pages/AuthPage'
import { SetupPage } from '@/pages/SetupPage'
import { Skeleton } from '@/components/ui/skeleton'
import { Navigate, useLocation } from 'react-router-dom'

interface SetupRouteProps {
  children: React.ReactNode
  fallback?: React.ReactNode
}

export function SetupRoute({ children, fallback }: SetupRouteProps) {
  const { isAuthenticated, isSetupComplete, needsInitialSetup, needsFirstUser, loading, firstClient } = useAuth()
  const location = useLocation()

  if (loading) {
    return fallback || <LoadingFallback />
  }

  // Priority 1: Check if user is authenticated
  // For authenticated users, we handle differently based on setup status
  if (isAuthenticated) {
    // If setup is complete (has projects), allow access to all pages
    if (isSetupComplete) {
      // If user just logged in and is on the root path, redirect to projects
      if (location.pathname === '/') {
        return <Navigate to="/projects" replace />
      }
      // Show the requested page
      return <>{children}</>
    } else {
      // User is authenticated but has no projects yet
      // Allow access to chat pages (conversations can exist without projects)
      const isChatRoute = location.pathname.startsWith('/chat/')
      if (isChatRoute) {
        return <>{children}</>
      }
      // For root path, redirect to projects
      if (location.pathname === '/') {
        return <Navigate to="/projects" replace />
      }
      // For other non-project routes, redirect to projects page to create first project
      if (location.pathname !== '/projects') {
        return <Navigate to="/projects" replace />
      }
      // Allow access to projects page
      return <>{children}</>
    }
  }

  // Priority 2: Check if initial setup is needed (no clients at all)
  if (needsInitialSetup) {
    return <SetupPage />
  }

  // Priority 3: Check if client exists but isn't active (needs Claude setup)
  if (firstClient && firstClient.status !== 'active') {
    return <SetupPage />
  }

  // Priority 4: Client is active but no users exist - show setup page to create first user
  if (needsFirstUser) {
    return <SetupPage />
  }

  // Priority 5: Client is active but user not authenticated - show login/register
  if (!isAuthenticated) {
    return <AuthPage />
  }

  // Priority 6: Authenticated but setup not complete - show setup
  // This case should rarely happen given the checks above
  if (!isSetupComplete) {
    return <SetupPage />
  }

  // Default: show the requested page
  return <>{children}</>
}

function LoadingFallback() {
  return (
    <div className="min-h-screen flex items-center justify-center bg-background p-4">
      <div className="w-full max-w-md space-y-4">
        <div className="text-center mb-8">
          <Skeleton className="h-8 w-32 mx-auto mb-2" />
          <Skeleton className="h-4 w-48 mx-auto" />
        </div>
        <div className="space-y-4">
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      </div>
    </div>
  )
}