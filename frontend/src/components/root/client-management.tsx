import { useNavigate } from 'react-router-dom'
import { ClientRootResponse } from '@/services/root-service'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { 
  RefreshCw,
  Globe,
  Users,
  MessageSquare,
  Calendar,
  Plus
} from 'lucide-react'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { format } from 'date-fns'

interface ClientManagementProps {
  clients: ClientRootResponse[]
  loading: boolean
  error: string | null
  onRefresh: () => void
  onAddClient?: () => void
}

export function ClientManagement({ clients, loading, error, onRefresh, onAddClient }: ClientManagementProps) {
  const navigate = useNavigate()

  const handleRowClick = (clientId: string) => {
    navigate(`/root/client/${clientId}`)
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
            <div className="flex items-center gap-2">
              {onAddClient && (
                <Button onClick={onAddClient} size="sm">
                  <Plus className="h-4 w-4 mr-2" />
                  Add Client
                </Button>
              )}
              <Button onClick={onRefresh} variant="outline" size="sm">
                <RefreshCw className="h-4 w-4 mr-2" />
                Refresh
              </Button>
            </div>
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
              </TableRow>
            </TableHeader>
            <TableBody>
              {clients.map((client) => (
                <TableRow 
                  key={client.id} 
                  className={`${client.deletedAt ? 'opacity-50' : ''} cursor-pointer hover:bg-muted/50 transition-colors`}
                  onClick={() => handleRowClick(client.id)}>
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
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </>
  )
}