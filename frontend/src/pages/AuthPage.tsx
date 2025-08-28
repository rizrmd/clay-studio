import { useState, useEffect } from 'react'
import { LoginForm } from '@/components/auth/login-form'
import { RegisterForm } from '@/components/auth/register-form'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useValtioAuth } from '@/hooks/use-valtio-auth'
import { Navigate } from 'react-router-dom'

export function AuthPage() {
  const [activeTab, setActiveTab] = useState<'login' | 'register'>('login')
  const { registrationEnabled, firstClient, isAuthenticated, isSetupComplete } = useValtioAuth()

  // If registration is disabled, always show login tab
  useEffect(() => {
    if (!registrationEnabled && activeTab === 'register') {
      setActiveTab('login')
    }
  }, [registrationEnabled, activeTab])

  // If user is already authenticated, redirect them
  if (isAuthenticated) {
    // If setup is complete, go to projects
    if (isSetupComplete) {
      return <Navigate to="/projects" replace />
    } else {
      // If setup is not complete (no projects), still go to projects page
      // where they can create their first project
      return <Navigate to="/projects" replace />
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-background p-4">
      <div className="w-full max-w-md">
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-foreground">Clay Studio</h1>
          <p className="text-muted-foreground mt-2">
            {firstClient ? `Welcome to ${firstClient.name}` : 'Welcome to your AI workspace'}
          </p>
        </div>

        {registrationEnabled ? (
          <Tabs value={activeTab} onValueChange={(value) => setActiveTab(value as 'login' | 'register')}>
            <TabsList className="grid w-full grid-cols-2 mb-6">
              <TabsTrigger value="login">Sign In</TabsTrigger>
              <TabsTrigger value="register">Sign Up</TabsTrigger>
            </TabsList>
            
            <TabsContent value="login">
              <LoginForm onSwitchToRegister={() => setActiveTab('register')} />
            </TabsContent>
            
            <TabsContent value="register">
              <RegisterForm onSwitchToLogin={() => setActiveTab('login')} />
            </TabsContent>
          </Tabs>
        ) : (
          <LoginForm />
        )}
      </div>
    </div>
  )
}