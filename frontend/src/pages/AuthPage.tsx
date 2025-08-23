import { useState, useEffect } from 'react'
import { LoginForm } from '@/components/auth/LoginForm'
import { RegisterForm } from '@/components/auth/RegisterForm'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useAuth } from '@/contexts/AuthContext'

export function AuthPage() {
  const [activeTab, setActiveTab] = useState<'login' | 'register'>('login')
  const { registrationEnabled, firstClient } = useAuth()

  // If registration is disabled, always show login tab
  useEffect(() => {
    if (!registrationEnabled && activeTab === 'register') {
      setActiveTab('login')
    }
  }, [registrationEnabled, activeTab])

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