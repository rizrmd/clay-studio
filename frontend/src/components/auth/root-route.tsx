import { useValtioAuth } from '@/hooks/use-valtio-auth'
import { Navigate } from 'react-router-dom'
import { Skeleton } from '@/components/ui/skeleton'
import { Alert, AlertDescription } from '@/components/ui/alert'

interface RootRouteProps {
  children: React.ReactNode
}

export function RootRoute({ children }: RootRouteProps) {
  const { user, isAuthenticated, loading } = useValtioAuth()

  if (loading) {
    return <LoadingFallback />
  }

  if (!isAuthenticated) {
    return <Navigate to="/auth" replace />
  }

  if (user?.role !== 'root') {
    return <AccessDeniedPage />
  }

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

function AccessDeniedPage() {
  return (
    <div className="min-h-screen flex items-center justify-center bg-background p-4">
      <div className="w-full max-w-md">
        <Alert className="border-red-500">
          <AlertDescription>
            <div className="text-center">
              <h2 className="text-xl font-semibold mb-2">Access Denied</h2>
              <p className="text-muted-foreground">
                You don't have permission to access this page. This area is restricted to root users only.
              </p>
              <div className="mt-4">
                <a href="/" className="text-primary hover:underline">
                  Return to Home
                </a>
              </div>
            </div>
          </AlertDescription>
        </Alert>
      </div>
    </div>
  )
}