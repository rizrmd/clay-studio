import { useAuth } from '@/hooks/use-auth'
import { AuthPage } from '@/pages/AuthPage'
import { ClientSetup } from '@/components/setup/client-setup'
import { Skeleton } from '@/components/ui/skeleton'

interface ProtectedRouteProps {
  children: React.ReactNode
  fallback?: React.ReactNode
  requireSetup?: boolean
}

export function ProtectedRoute({ children, fallback, requireSetup = true }: ProtectedRouteProps) {
  const { isAuthenticated, loading, isSetupComplete } = useAuth()

  if (loading) {
    return fallback || <LoadingFallback />
  }

  if (!isAuthenticated) {
    return <AuthPage />
  }

  // If setup is required and not complete, show client setup
  if (requireSetup && !isSetupComplete) {
    return <InitialSetupPage />
  }

  return <>{children}</>
}

function InitialSetupPage() {
  const handleClientAdded = () => {
    // Refresh the page to update auth status and redirect to main app
    window.location.reload()
  }

  return (
    <div className="min-h-screen bg-background p-8">
      <div className="max-w-6xl mx-auto">
        <div className="mb-8 text-center">
          <h1 className="text-4xl font-bold mb-4">Welcome to Clay Studio</h1>
          <p className="text-xl text-muted-foreground">
            Let's get started by setting up your first AI client
          </p>
        </div>
        <ClientSetup onClientAdded={handleClientAdded} />
      </div>
    </div>
  )
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