import { useValtioAuth } from '@/hooks/use-valtio-auth'
import { Navigate } from 'react-router-dom'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
// import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useState, useEffect } from 'react'
import api from '@/lib/api'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Skeleton } from '@/components/ui/skeleton'
import { AppHeader } from '@/components/layout/app-header'

interface SystemConfig {
  registrationEnabled: boolean
  requireInviteCode: boolean
  allowedDomains: string[]
}

export function ConfigPage() {
  const { user, isAuthenticated } = useValtioAuth()
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [config, setConfig] = useState<SystemConfig>({
    registrationEnabled: false,
    requireInviteCode: false,
    allowedDomains: []
  })
  const [message, setMessage] = useState<{ type: 'success' | 'error', text: string } | null>(null)
  const [newDomain, setNewDomain] = useState('')

  useEffect(() => {
    if (isAuthenticated && user?.role === 'admin') {
      fetchConfig()
    }
  }, [isAuthenticated, user])

  const fetchConfig = async () => {
    try {
      const response = await api.get('/admin/config')
      setConfig({
        ...response,
        allowedDomains: response.allowedDomains || []
      })
    } catch (error) {
      setMessage({ type: 'error', text: 'Failed to load configuration' })
    } finally {
      setLoading(false)
    }
  }

  const saveConfig = async () => {
    setSaving(true)
    setMessage(null)
    try {
      await api.put('/admin/config', config)
      setMessage({ type: 'success', text: 'Configuration saved successfully' })
    } catch (error) {
      setMessage({ type: 'error', text: 'Failed to save configuration' })
    } finally {
      setSaving(false)
    }
  }

  const handleAddDomain = () => {
    const domains = config.allowedDomains || []
    if (newDomain && !domains.includes(newDomain)) {
      setConfig({
        ...config,
        allowedDomains: [...domains, newDomain]
      })
      setNewDomain('')
    }
  }

  const handleRemoveDomain = (domain: string) => {
    setConfig({
      ...config,
      allowedDomains: (config.allowedDomains || []).filter(d => d !== domain)
    })
  }

  if (!isAuthenticated || user?.role !== 'admin') {
    return <Navigate to="/" replace />
  }

  if (loading) {
    return (
      <div className="container mx-auto py-8 space-y-6">
        <Skeleton className="h-10 w-48" />
        <div className="grid gap-6">
          <Skeleton className="h-96 w-full" />
          <Skeleton className="h-96 w-full" />
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900">
      <AppHeader />

      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="mb-8">
          <h1 className="text-3xl font-bold mb-2 text-gray-900 dark:text-gray-100">System Configuration</h1>
          <p className="text-gray-600 dark:text-gray-400">Manage system-wide settings and preferences</p>
        </div>

      {message && (
        <Alert className={`mb-6 ${message.type === 'success' ? 'border-green-500' : 'border-red-500'}`}>
          <AlertDescription>{message.text}</AlertDescription>
        </Alert>
      )}

      <div className="space-y-6">
        <Card>
          <CardHeader>
            <CardTitle>Registration Settings</CardTitle>
            <CardDescription>Configure how new users can join the system</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label>Enable Registration</Label>
                <p className="text-sm text-muted-foreground">Allow new users to register</p>
              </div>
              <Button
                variant={config.registrationEnabled ? "default" : "outline"}
                onClick={() => setConfig({ ...config, registrationEnabled: !config.registrationEnabled })}
              >
                {config.registrationEnabled ? 'Enabled' : 'Disabled'}
              </Button>
            </div>

            {config.registrationEnabled && (
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label>Require Invite Code</Label>
                  <p className="text-sm text-muted-foreground">Users need an invite code to register</p>
                </div>
                <Button
                  variant={config.requireInviteCode ? "default" : "outline"}
                  onClick={() => setConfig({ ...config, requireInviteCode: !config.requireInviteCode })}
                >
                  {config.requireInviteCode ? 'Required' : 'Not Required'}
                </Button>
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Domain Management</CardTitle>
            <CardDescription>Restrict registration to specific email domains</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex gap-2">
              <Input
                placeholder="example.com"
                value={newDomain}
                onChange={(e) => setNewDomain(e.target.value)}
                onKeyPress={(e) => e.key === 'Enter' && handleAddDomain()}
              />
              <Button onClick={handleAddDomain}>Add Domain</Button>
            </div>
            <div className="space-y-2">
              {!config.allowedDomains || config.allowedDomains.length === 0 ? (
                <p className="text-sm text-muted-foreground">No domain restrictions (all domains allowed)</p>
              ) : (
                config.allowedDomains.map((domain) => (
                  <div key={domain} className="flex items-center justify-between p-2 border rounded">
                    <span>{domain}</span>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleRemoveDomain(domain)}
                    >
                      Remove
                    </Button>
                  </div>
                ))
              )}
            </div>
          </CardContent>
        </Card>
      </div>

        <div className="mt-8 flex justify-end">
          <Button onClick={saveConfig} disabled={saving}>
            {saving ? 'Saving...' : 'Save Changes'}
          </Button>
        </div>
      </div>
    </div>
  )
}