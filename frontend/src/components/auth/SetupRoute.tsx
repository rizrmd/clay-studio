import { useAuth } from '@/contexts/AuthContext'
import { AuthPage } from '@/pages/AuthPage'
import { SetupPage } from '@/pages/SetupPage'
import { Skeleton } from '@/components/ui/skeleton'

interface SetupRouteProps {
  children: React.ReactNode
  fallback?: React.ReactNode
}

export function SetupRoute({ children, fallback }: SetupRouteProps) {
  const { isAuthenticated, isSetupComplete, needsInitialSetup, needsFirstUser, loading, firstClient } = useAuth()

  console.log('SetupRoute debug:', {
    loading,
    needsInitialSetup,
    needsFirstUser,
    firstClient,
    clientStatus: firstClient?.status,
    isAuthenticated,
    isSetupComplete,
    shouldShowSetup: firstClient && firstClient.status !== 'active'
  })

  if (loading) {
    return fallback || <LoadingFallback />
  }

  // Check client status FIRST - if client exists but isn't active, show setup
  // This takes priority over authentication check
  if (firstClient && firstClient.status !== 'active') {
    console.log('Showing SetupPage because client status is:', firstClient.status)
    return <SetupPage />
  }

  // If no clients exist at all - show setup page
  if (needsInitialSetup) {
    console.log('Showing SetupPage because no clients exist')
    return <SetupPage />
  }

  // Client is active but no users exist - show setup page to create first user
  if (needsFirstUser) {
    console.log('Showing SetupPage because client is active but no users exist')
    return <SetupPage />
  }

  // Client is active but user not authenticated - show login/register
  if (!isAuthenticated) {
    console.log('Showing AuthPage because not authenticated and client is active')
    return <AuthPage />
  }

  // Authenticated but setup not complete - show setup
  if (!isSetupComplete) {
    console.log('Showing SetupPage because setup is not complete')
    return <SetupPage />
  }

  // Authenticated and setup complete - show app
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