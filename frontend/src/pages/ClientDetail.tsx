import { useEffect, useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { ClientRootResponse, rootService } from '@/lib/services/root-service'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { 
  ArrowLeft,
  RefreshCw,
  Power,
  Pause,
  Trash2,
  Edit,
  Users,
  MessageSquare,
  Calendar,
  Shield,
  Key
} from 'lucide-react'
import { format } from 'date-fns'
import { ProtectedRoute } from '@/components/auth/protected-route'
import { ClientDetailDialog } from '@/components/root/client-detail-dialog'
import { SetupClaudeDialog } from '@/components/root/setup-claude-dialog'
import { DomainManagement } from '@/components/root/domain-management'
import { UserManagement } from '@/components/shared/user-management'

export function ClientDetailPage() {
  const { clientId } = useParams()
  const navigate = useNavigate()
  const [client, setClient] = useState<ClientRootResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [actionLoading, setActionLoading] = useState(false)
  const [editDialogOpen, setEditDialogOpen] = useState(false)
  const [claudeDialogOpen, setClaudeDialogOpen] = useState(false)

  const fetchClient = async () => {
    if (!clientId) return
    
    try {
      setLoading(true)
      setError(null)
      const clients = await rootService.getClientsRoot()
      const foundClient = clients.find((c: ClientRootResponse) => c.id === clientId)
      if (foundClient) {
        setClient(foundClient)
      } else {
        setError('Client not found')
      }
    } catch (err: any) {
      setError(err.message || 'Failed to fetch client details')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchClient()
  }, [clientId])

  const handleEnableClient = async () => {
    if (!client) return
    
    try {
      setActionLoading(true)
      await rootService.enableClient(client.id)
      await fetchClient()
    } catch (err) {
      console.error('Failed to enable client:', err)
    } finally {
      setActionLoading(false)
    }
  }

  const handleReactivateClient = async () => {
    if (!client) return
    
    try {
      setActionLoading(true)
      await rootService.enableClient(client.id)
      await fetchClient()
    } catch (err) {
      console.error('Failed to reactivate client:', err)
    } finally {
      setActionLoading(false)
    }
  }

  const handleSuspendClient = async () => {
    if (!client) return
    
    if (!confirm('Are you sure you want to suspend this client? Users will not be able to access it.')) {
      return
    }
    
    try {
      setActionLoading(true)
      await rootService.suspendClient(client.id)
      await fetchClient()
    } catch (err) {
      console.error('Failed to suspend client:', err)
    } finally {
      setActionLoading(false)
    }
  }

  const handleDeleteClient = async () => {
    if (!client) return
    
    if (!confirm('Are you sure you want to delete this client? This action cannot be undone.')) {
      return
    }
    
    try {
      setActionLoading(true)
      await rootService.deleteClient(client.id)
      navigate('/root')
    } catch (err: any) {
      alert(err.response?.data?.error || 'Failed to delete client')
    } finally {
      setActionLoading(false)
    }
  }

  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'active':
        return <Badge className="bg-green-500">Active</Badge>
      case 'suspended':
        return <Badge className="bg-orange-500">Suspended</Badge>
      case 'error':
        return <Badge variant="destructive">Error</Badge>
      case 'installing':
        return <Badge className="bg-blue-500">Installing</Badge>
      case 'pending':
        return <Badge variant="secondary">Pending</Badge>
      default:
        return <Badge variant="outline">{status}</Badge>
    }
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-background">
        <div className="container mx-auto py-8 px-4">
          <Card>
            <CardContent className="flex items-center justify-center py-16">
              <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground" />
            </CardContent>
          </Card>
        </div>
      </div>
    )
  }

  if (error || !client) {
    return (
      <div className="min-h-screen bg-background">
        <div className="container mx-auto py-8 px-4">
          <Alert variant="destructive">
            <AlertDescription>{error || 'Client not found'}</AlertDescription>
          </Alert>
          <Button className="mt-4" onClick={() => navigate('/root')}>
            <ArrowLeft className="h-4 w-4 mr-2" />
            Back to Dashboard
          </Button>
        </div>
      </div>
    )
  }

  return (
    <ProtectedRoute>
      <div className="min-h-screen bg-background">
        <div className="container mx-auto py-8 px-4">
          {/* Header */}
          <div className="flex items-center justify-between mb-6">
            <div className="flex items-center gap-4">
              <Button 
                variant="ghost" 
                size="sm"
                onClick={() => navigate('/root')}
              >
                <ArrowLeft className="h-4 w-4 mr-2" />
                Back
              </Button>
              <div>
                <h1 className="text-2xl font-bold">{client.name}</h1>
                {client.description && (
                  <p className="text-muted-foreground">{client.description}</p>
                )}
              </div>
            </div>
            <div className="flex items-center gap-2">
              <Button onClick={() => setEditDialogOpen(true)} variant="outline">
                <Edit className="h-4 w-4 mr-2" />
                Edit
              </Button>
              <Button onClick={fetchClient} variant="outline">
                <RefreshCw className="h-4 w-4 mr-2" />
                Refresh
              </Button>
            </div>
          </div>

          {/* Status Bar */}
          <Card className="mb-6">
            <CardContent className="flex items-center justify-between py-4">
              <div className="flex items-center gap-4">
                <div>
                  <p className="text-sm text-muted-foreground">Status</p>
                  <div className="flex items-center gap-2 mt-1">
                    {getStatusBadge(client.status)}
                    {client.hasClaudeToken && (
                      <Badge variant="outline" className="text-xs">
                        Claude ✓
                      </Badge>
                    )}
                  </div>
                </div>
                <div className="border-l pl-4">
                  <p className="text-sm text-muted-foreground">Created</p>
                  <div className="flex items-center gap-1 mt-1">
                    <Calendar className="h-3 w-3 text-muted-foreground" />
                    <span className="text-sm font-medium">
                      {format(new Date(client.createdAt), 'MMM d, yyyy')}
                    </span>
                  </div>
                </div>
              </div>
              
              <div className="flex items-center gap-2">
                {client.status === 'active' ? (
                  <Button 
                    onClick={handleSuspendClient}
                    variant="outline"
                    className="text-orange-600"
                    disabled={actionLoading}
                  >
                    {actionLoading ? (
                      <RefreshCw className="h-4 w-4 animate-spin mr-2" />
                    ) : (
                      <Pause className="h-4 w-4 mr-2" />
                    )}
                    Suspend
                  </Button>
                ) : client.status === 'suspended' ? (
                  <Button 
                    onClick={handleReactivateClient}
                    variant="outline"
                    className="text-green-600"
                    disabled={actionLoading}
                  >
                    {actionLoading ? (
                      <RefreshCw className="h-4 w-4 animate-spin mr-2" />
                    ) : (
                      <Power className="h-4 w-4 mr-2" />
                    )}
                    Reactivate
                  </Button>
                ) : (
                  <Button 
                    onClick={handleEnableClient}
                    variant="outline"
                    className="text-green-600"
                    disabled={actionLoading}
                  >
                    {actionLoading ? (
                      <RefreshCw className="h-4 w-4 animate-spin mr-2" />
                    ) : (
                      <Power className="h-4 w-4 mr-2" />
                    )}
                    Enable
                  </Button>
                )}
                <Button
                  onClick={handleDeleteClient}
                  variant="destructive"
                  disabled={actionLoading}
                >
                  <Trash2 className="h-4 w-4 mr-2" />
                  Delete
                </Button>
              </div>
            </CardContent>
          </Card>

          {/* Main Content */}
          <Tabs defaultValue="overview" className="space-y-4">
            <TabsList>
              <TabsTrigger value="overview">Overview</TabsTrigger>
              <TabsTrigger value="users">Users</TabsTrigger>
              <TabsTrigger value="configuration">Configuration</TabsTrigger>
              <TabsTrigger value="security">Security</TabsTrigger>
              <TabsTrigger value="activity">Activity</TabsTrigger>
            </TabsList>

            <TabsContent value="overview" className="space-y-4">
              {/* Domain Management Section */}
              <DomainManagement
                clientId={client.id}
                initialDomains={client.domains || []}
                onUpdate={fetchClient}
              />

              {/* Stats Grid */}
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                <Card>
                  <CardHeader className="pb-3">
                    <CardTitle className="text-sm font-medium">
                      <Users className="h-4 w-4 inline mr-2" />
                      Users
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <p className="text-2xl font-bold">{client.userCount}</p>
                    <p className="text-xs text-muted-foreground">Total users</p>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="pb-3">
                    <CardTitle className="text-sm font-medium">
                      <MessageSquare className="h-4 w-4 inline mr-2" />
                      Conversations
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <p className="text-2xl font-bold">{client.conversationCount}</p>
                    <p className="text-xs text-muted-foreground">Total conversations</p>
                  </CardContent>
                </Card>


                <Card>
                  <CardHeader className="pb-3">
                    <CardTitle className="text-sm font-medium">
                      <Shield className="h-4 w-4 inline mr-2" />
                      Security
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-2">
                      <div className="flex items-center justify-between">
                        <span className="text-sm">API Access</span>
                        {client.config?.apiKey ? (
                          <Badge variant="outline" className="text-xs">Enabled</Badge>
                        ) : (
                          <Badge variant="secondary" className="text-xs">Disabled</Badge>
                        )}
                      </div>
                      <div className="flex items-center justify-between">
                        <span className="text-sm">Claude Token</span>
                        {client.hasClaudeToken ? (
                          <Badge variant="outline" className="text-xs">Configured</Badge>
                        ) : (
                          <Badge variant="secondary" className="text-xs">Not Set</Badge>
                        )}
                      </div>
                    </div>
                  </CardContent>
                </Card>

              </div>
            </TabsContent>

            <TabsContent value="users">
              <UserManagement 
                initialRegistrationEnabled={false}
                initialRequireInviteCode={false}
                clientId={client.id}
              />
            </TabsContent>

            <TabsContent value="configuration">
              <Card>
                <CardHeader>
                  <CardTitle>Configuration Settings</CardTitle>
                  <CardDescription>
                    Manage client configuration and settings
                  </CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div>
                    <h3 className="font-medium mb-2">API Configuration</h3>
                    <div className="space-y-2">
                      <div className="flex items-center justify-between py-2 border-b">
                        <span className="text-sm">API Key</span>
                        {client.config?.apiKey ? (
                          <Badge variant="outline">Configured</Badge>
                        ) : (
                          <Badge variant="secondary">Not configured</Badge>
                        )}
                      </div>
                      <div className="flex items-center justify-between py-2 border-b">
                        <div>
                          <span className="text-sm">Claude Token</span>
                          {!client.hasClaudeToken && (
                            <p className="text-xs text-muted-foreground mt-1">
                              Required for AI features
                            </p>
                          )}
                        </div>
                        <div className="flex items-center gap-2">
                          {client.hasClaudeToken ? (
                            <Badge variant="outline">Configured</Badge>
                          ) : (
                            <Badge variant="secondary">Not configured</Badge>
                          )}
                          <Button
                            size="sm"
                            variant={client.hasClaudeToken ? "outline" : "default"}
                            onClick={() => setClaudeDialogOpen(true)}
                          >
                            <Key className="h-4 w-4 mr-2" />
                            {client.hasClaudeToken ? "Update Token" : "Setup Claude"}
                          </Button>
                        </div>
                      </div>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </TabsContent>

            <TabsContent value="security">
              <Card>
                <CardHeader>
                  <CardTitle>Security Settings</CardTitle>
                  <CardDescription>
                    Security configuration and access control
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="space-y-4">
                    <div className="flex items-center justify-between py-2">
                      <div>
                        <p className="font-medium">API Access</p>
                        <p className="text-sm text-muted-foreground">
                          Allow API access for this client
                        </p>
                      </div>
                      {client.config?.apiKey ? (
                        <Badge variant="outline">Enabled</Badge>
                      ) : (
                        <Badge variant="secondary">Disabled</Badge>
                      )}
                    </div>
                  </div>
                </CardContent>
              </Card>
            </TabsContent>

            <TabsContent value="activity">
              <Card>
                <CardHeader>
                  <CardTitle>Activity Log</CardTitle>
                  <CardDescription>
                    Recent activity and usage statistics
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="text-center py-8 text-muted-foreground">
                    Activity log coming soon...
                  </div>
                </CardContent>
              </Card>
            </TabsContent>
          </Tabs>
        </div>

        {/* Edit Dialog */}
        {client && (
          <ClientDetailDialog
            client={client}
            open={editDialogOpen}
            onOpenChange={setEditDialogOpen}
            onUpdate={fetchClient}
          />
        )}

        {/* Setup Claude Dialog */}
        {client && (
          <SetupClaudeDialog
            clientId={client.id}
            clientName={client.name}
            open={claudeDialogOpen}
            onOpenChange={setClaudeDialogOpen}
            onSuccess={fetchClient}
          />
        )}
      </div>
    </ProtectedRoute>
  )
}

export default ClientDetailPage