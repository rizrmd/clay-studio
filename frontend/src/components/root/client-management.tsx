import { useState } from 'react'
import { ClientRootResponse, rootService } from '@/services/root-service'
import { ClientDetailDialog } from './client-detail-dialog'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { 
  RefreshCw, 
  Settings, 
  Trash2, 
  Power, 
  PowerOff,
  Edit,
  Globe,
  Users,
  MessageSquare,
  Calendar,
  MoreVertical
} from 'lucide-react'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { format } from 'date-fns'

interface ClientManagementProps {
  clients: ClientRootResponse[]
  loading: boolean
  error: string | null
  onRefresh: () => void
}

export function ClientManagement({ clients, loading, error, onRefresh }: ClientManagementProps) {
  const [selectedClient, setSelectedClient] = useState<ClientRootResponse | null>(null)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [actionLoading, setActionLoading] = useState<string | null>(null)

  const handleEnableClient = async (clientId: string) => {
    try {
      setActionLoading(clientId)
      await rootService.enableClient(clientId)
      onRefresh()
    } catch (err) {
      console.error('Failed to enable client:', err)
    } finally {
      setActionLoading(null)
    }
  }

  const handleDisableClient = async (clientId: string) => {
    try {
      setActionLoading(clientId)
      await rootService.disableClient(clientId)
      onRefresh()
    } catch (err) {
      console.error('Failed to disable client:', err)
    } finally {
      setActionLoading(null)
    }
  }

  const handleDeleteClient = async (clientId: string) => {
    if (!confirm('Are you sure you want to delete this client? This action cannot be undone.')) {
      return
    }
    
    try {
      setActionLoading(clientId)
      await rootService.deleteClient(clientId)
      onRefresh()
    } catch (err: any) {
      alert(err.response?.data?.error || 'Failed to delete client')
    } finally {
      setActionLoading(null)
    }
  }

  const handleEditClient = (client: ClientRootResponse) => {
    setSelectedClient(client)
    setDialogOpen(true)
  }

  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'active':
        return <Badge className="bg-green-500">Active</Badge>
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
      <Card>
        <CardContent className="flex items-center justify-center py-8">
          <RefreshCw className="h-6 w-6 animate-spin text-muted-foreground" />
        </CardContent>
      </Card>
    )
  }

  if (error) {
    return (
      <Alert variant="destructive">
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    )
  }

  return (
    <>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Client Management</CardTitle>
              <CardDescription>
                Manage all clients in the system
              </CardDescription>
            </div>
            <Button onClick={onRefresh} variant="outline" size="sm">
              <RefreshCw className="h-4 w-4 mr-2" />
              Refresh
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Domains</TableHead>
                <TableHead>Users</TableHead>
                <TableHead>Conversations</TableHead>
                <TableHead>Created</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {clients.map((client) => (
                <TableRow key={client.id} className={client.deletedAt ? 'opacity-50' : ''}>
                  <TableCell>
                    <div>
                      <div className="font-medium">{client.name}</div>
                      {client.description && (
                        <div className="text-xs text-muted-foreground">{client.description}</div>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-2">
                      {getStatusBadge(client.status)}
                      {client.hasClaudeToken && (
                        <Badge variant="outline" className="text-xs">
                          Claude ✓
                        </Badge>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>
                    {client.domains && client.domains.length > 0 ? (
                      <div className="flex items-center gap-1">
                        <Globe className="h-3 w-3 text-muted-foreground" />
                        <span className="text-sm">{client.domains.length}</span>
                      </div>
                    ) : (
                      <span className="text-sm text-muted-foreground">-</span>
                    )}
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1">
                      <Users className="h-3 w-3 text-muted-foreground" />
                      <span className="text-sm">{client.userCount}</span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1">
                      <MessageSquare className="h-3 w-3 text-muted-foreground" />
                      <span className="text-sm">{client.conversationCount}</span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1">
                      <Calendar className="h-3 w-3 text-muted-foreground" />
                      <span className="text-xs">
                        {format(new Date(client.createdAt), 'MMM d, yyyy')}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell className="text-right">
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button 
                          variant="ghost" 
                          size="sm"
                          disabled={actionLoading === client.id}
                        >
                          {actionLoading === client.id ? (
                            <RefreshCw className="h-4 w-4 animate-spin" />
                          ) : (
                            <MoreVertical className="h-4 w-4" />
                          )}
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuLabel>Actions</DropdownMenuLabel>
                        <DropdownMenuItem onClick={() => handleEditClient(client)}>
                          <Edit className="h-4 w-4 mr-2" />
                          Edit Details
                        </DropdownMenuItem>
                        <DropdownMenuItem onClick={() => handleEditClient(client)}>
                          <Settings className="h-4 w-4 mr-2" />
                          Configuration
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        {client.status === 'active' ? (
                          <DropdownMenuItem 
                            onClick={() => handleDisableClient(client.id)}
                            className="text-orange-600"
                          >
                            <PowerOff className="h-4 w-4 mr-2" />
                            Disable
                          </DropdownMenuItem>
                        ) : (
                          <DropdownMenuItem 
                            onClick={() => handleEnableClient(client.id)}
                            className="text-green-600"
                          >
                            <Power className="h-4 w-4 mr-2" />
                            Enable
                          </DropdownMenuItem>
                        )}
                        <DropdownMenuSeparator />
                        <DropdownMenuItem 
                          onClick={() => handleDeleteClient(client.id)}
                          className="text-red-600"
                          disabled={client.userCount > 0}
                        >
                          <Trash2 className="h-4 w-4 mr-2" />
                          Delete
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      {selectedClient && (
        <ClientDetailDialog
          client={selectedClient}
          open={dialogOpen}
          onOpenChange={setDialogOpen}
          onUpdate={onRefresh}
        />
      )}
    </>
  )
}