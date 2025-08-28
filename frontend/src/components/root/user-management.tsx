import { useState, useEffect } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import { 
  Users,
  UserPlus,
  Search,
  Shield,
  MoreVertical,
  Trash2,
  Edit,
  Ban,
  CheckCircle
} from 'lucide-react'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { format } from 'date-fns'
import { rootService, UserResponse } from '@/services/root-service'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { RefreshCw } from 'lucide-react'

interface UserManagementProps {
  clientName: string
  clientId: string
}

export function UserManagement({ clientName, clientId }: UserManagementProps) {
  const [searchQuery, setSearchQuery] = useState('')
  const [addUserDialogOpen, setAddUserDialogOpen] = useState(false)
  const [newUser, setNewUser] = useState({ username: '', password: '', role: 'user' as ('admin' | 'user') })
  const [users, setUsers] = useState<UserResponse[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [actionLoading, setActionLoading] = useState(false)

  const fetchUsers = async () => {
    if (!clientId) return
    
    try {
      setLoading(true)
      setError(null)
      const fetchedUsers = await rootService.getUsersForClient(clientId)
      setUsers(fetchedUsers)
    } catch (err: any) {
      setError(err.message || 'Failed to fetch users')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchUsers()
  }, [clientId])

  const filteredUsers = users.filter(user => 
    user.username.toLowerCase().includes(searchQuery.toLowerCase())
  )

  const handleAddUser = async () => {
    if (!newUser.username || !newUser.password) return
    
    try {
      setActionLoading(true)
      await rootService.createUser(clientId, {
        username: newUser.username,
        password: newUser.password,
        role: newUser.role
      })
      await fetchUsers()
      setAddUserDialogOpen(false)
      setNewUser({ username: '', password: '', role: 'user' })
    } catch (err: any) {
      alert(err.response?.data?.error || 'Failed to create user')
    } finally {
      setActionLoading(false)
    }
  }

  const handleSuspendUser = (userId: string) => {
    // Note: Backend doesn't support suspend/activate yet, just delete
    console.log('Suspending user:', userId)
  }

  const handleActivateUser = (userId: string) => {
    // Note: Backend doesn't support suspend/activate yet, just delete
    console.log('Activating user:', userId)
  }

  const handleDeleteUser = async (userId: string) => {
    if (!confirm('Are you sure you want to delete this user? This action cannot be undone.')) {
      return
    }
    
    try {
      setActionLoading(true)
      await rootService.deleteUser(clientId, userId)
      await fetchUsers()
    } catch (err: any) {
      alert(err.response?.data?.error || 'Failed to delete user')
    } finally {
      setActionLoading(false)
    }
  }

  const getRoleBadge = (role: string) => {
    return role === 'admin' ? (
      <Badge className="bg-purple-500">
        <Shield className="h-3 w-3 mr-1" />
        Admin
      </Badge>
    ) : (
      <Badge variant="secondary">User</Badge>
    )
  }

  const getStatusBadge = (status: string) => {
    return status === 'active' ? (
      <Badge className="bg-green-500">
        <CheckCircle className="h-3 w-3 mr-1" />
        Active
      </Badge>
    ) : (
      <Badge className="bg-orange-500">
        <Ban className="h-3 w-3 mr-1" />
        Suspended
      </Badge>
    )
  }

  if (loading) {
    return (
      <Card>
        <CardContent className="flex items-center justify-center py-16">
          <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground" />
        </CardContent>
      </Card>
    )
  }

  if (error) {
    return (
      <Card>
        <CardContent className="py-8">
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
          <Button className="mt-4" onClick={fetchUsers} variant="outline">
            <RefreshCw className="h-4 w-4 mr-2" />
            Retry
          </Button>
        </CardContent>
      </Card>
    )
  }

  return (
    <>
      <Card>
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between">
            <CardTitle className="text-base flex items-center gap-2">
              <Users className="h-4 w-4" />
              User Management
            </CardTitle>
            <Button 
              size="sm"
              onClick={() => setAddUserDialogOpen(true)}
              className="h-8"
            >
              <UserPlus className="h-4 w-4 mr-1.5" />
              Add User
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {/* Search Bar */}
          <div className="mb-4">
            <div className="relative">
              <Search className="absolute left-2 top-2.5 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Search users..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-8 h-9"
              />
            </div>
          </div>

          {/* Users Table */}
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Username</TableHead>
                  <TableHead>Role</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Last Active</TableHead>
                  <TableHead>Joined</TableHead>
                  <TableHead className="w-[50px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredUsers.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={6} className="text-center py-8 text-muted-foreground">
                      {searchQuery ? 'No users found matching your search' : 'No users yet'}
                    </TableCell>
                  </TableRow>
                ) : (
                  filteredUsers.map((user) => (
                    <TableRow key={user.id}>
                      <TableCell>
                        <div className="font-medium">{user.username}</div>
                      </TableCell>
                      <TableCell>{getRoleBadge(user.role)}</TableCell>
                      <TableCell>{getStatusBadge(user.status)}</TableCell>
                      <TableCell className="text-sm text-muted-foreground">
                        {user.lastActive ? format(new Date(user.lastActive), 'MMM d, h:mm a') : '-'}
                      </TableCell>
                      <TableCell className="text-sm text-muted-foreground">
                        {format(new Date(user.createdAt), 'MMM d, yyyy')}
                      </TableCell>
                      <TableCell>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
                              <MoreVertical className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem onClick={() => console.log('Edit user:', user)}>
                              <Edit className="h-4 w-4 mr-2" />
                              Edit Details
                            </DropdownMenuItem>
                            <DropdownMenuSeparator />
                            {user.status === 'active' ? (
                              <DropdownMenuItem 
                                onClick={() => handleSuspendUser(user.id)}
                                className="text-orange-600"
                              >
                                <Ban className="h-4 w-4 mr-2" />
                                Suspend User
                              </DropdownMenuItem>
                            ) : (
                              <DropdownMenuItem 
                                onClick={() => handleActivateUser(user.id)}
                                className="text-green-600"
                              >
                                <CheckCircle className="h-4 w-4 mr-2" />
                                Activate User
                              </DropdownMenuItem>
                            )}
                            <DropdownMenuItem 
                              onClick={() => handleDeleteUser(user.id)}
                              className="text-destructive"
                            >
                              <Trash2 className="h-4 w-4 mr-2" />
                              Delete User
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </div>

          {/* Summary */}
          <div className="mt-4 flex items-center justify-between text-sm text-muted-foreground">
            <div>
              Showing {filteredUsers.length} of {users.length} users
            </div>
            <div className="flex gap-4">
              <span>{users.filter(u => u.status === 'active').length} active</span>
              <span>{users.filter(u => u.role === 'admin').length} admins</span>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Add User Dialog */}
      <Dialog open={addUserDialogOpen} onOpenChange={setAddUserDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add New User</DialogTitle>
            <DialogDescription>
              Add a new user to {clientName}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="username">Username</Label>
              <Input
                id="username"
                placeholder="Enter username"
                value={newUser.username}
                onChange={(e) => setNewUser({ ...newUser, username: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="password">Password</Label>
              <Input
                id="password"
                type="password"
                placeholder="Enter password"
                value={newUser.password}
                onChange={(e) => setNewUser({ ...newUser, password: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="role">Role</Label>
              <select
                id="role"
                className="w-full px-3 py-2 border rounded-md"
                value={newUser.role}
                onChange={(e) => setNewUser({ ...newUser, role: e.target.value as ('admin' | 'user') })}
              >
                <option value="user">User</option>
                <option value="admin">Admin</option>
              </select>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setAddUserDialogOpen(false)}>
              Cancel
            </Button>
            <Button 
              onClick={handleAddUser} 
              disabled={!newUser.username || !newUser.password || actionLoading}
            >
              {actionLoading ? (
                <>
                  <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                  Adding...
                </>
              ) : (
                'Add User'
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}