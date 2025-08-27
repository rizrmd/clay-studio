import { useValtioAuth } from '@/hooks/use-valtio-auth'
import { Navigate } from 'react-router-dom'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
// import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useState, useEffect } from 'react'
import axios from '@/lib/axios'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Skeleton } from '@/components/ui/skeleton'

interface SystemConfig {
  registrationEnabled: boolean
  requireInviteCode: boolean
  sessionTimeout: number
  allowedDomains: string[]
}

export function ConfigPage() {
  const { user, isAuthenticated } = useValtioAuth()
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [config, setConfig] = useState<SystemConfig>({
    registrationEnabled: false,
    requireInviteCode: false,
    sessionTimeout: 86400,
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
      const response = await axios.get('/admin/config')
      setConfig({
        ...response.data,
        allowedDomains: response.data.allowedDomains || []
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
      await axios.put('/admin/config', config)
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
    <div className="container mx-auto py-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-2">System Configuration</h1>
        <p className="text-muted-foreground">Manage system-wide settings and preferences</p>
      </div>

      {message && (
        <Alert className={`mb-6 ${message.type === 'success' ? 'border-green-500' : 'border-red-500'}`}>
          <AlertDescription>{message.text}</AlertDescription>
        </Alert>
      )}

      <Tabs defaultValue="general" className="space-y-6">
        <TabsList>
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="security">Security</TabsTrigger>
        </TabsList>

        <TabsContent value="general" className="space-y-6">
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
        </TabsContent>

        <TabsContent value="security" className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Session Settings</CardTitle>
              <CardDescription>Configure session security parameters</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="sessionTimeout">Session Timeout (seconds)</Label>
                <Input
                  id="sessionTimeout"
                  type="number"
                  value={config.sessionTimeout}
                  onChange={(e) => setConfig({ ...config, sessionTimeout: parseInt(e.target.value) || 86400 })}
                />
                <p className="text-sm text-muted-foreground">
                  Current: {Math.floor(config.sessionTimeout / 3600)} hours
                </p>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Allowed Domains</CardTitle>
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
        </TabsContent>
      </Tabs>

      <div className="mt-8 flex justify-end">
        <Button onClick={saveConfig} disabled={saving}>
          {saving ? 'Saving...' : 'Save Changes'}
        </Button>
      </div>
    </div>
  )
}