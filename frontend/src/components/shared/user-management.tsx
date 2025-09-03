import { useState, useEffect } from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { 
  Search, 
  Plus, 
  MoreHorizontal, 
  Edit, 
  Trash2,
  UserPlus,
  Settings,
  RefreshCw
} from "lucide-react";
import api from "@/lib/utils/api";
import { format } from "date-fns";
import { useValtioAuth } from "@/hooks/use-valtio-auth";

interface User {
  id: string;
  username: string;
  email?: string;
  role: string;
  created_at: string;
  last_active?: string;
  status?: string;
}

interface UserManagementProps {
  initialRegistrationEnabled: boolean;
  initialRequireInviteCode: boolean;
  clientId?: string;
  onUpdate?: () => void;
}

export function UserManagement({
  initialRegistrationEnabled,
  initialRequireInviteCode,
  clientId,
  onUpdate,
}: UserManagementProps) {
  const { user } = useValtioAuth();
  const isRoot = user?.role === "root";
  
  const [registrationEnabled, setRegistrationEnabled] = useState(initialRegistrationEnabled);
  const [requireInviteCode, setRequireInviteCode] = useState(initialRequireInviteCode);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{
    type: "success" | "error";
    text: string;
  } | null>(null);
  const [hasChanges, setHasChanges] = useState(false);
  
  // User CRUD states
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedRole, setSelectedRole] = useState<string>("all");
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showEditDialog, setShowEditDialog] = useState(false);
  const [selectedUser, setSelectedUser] = useState<User | null>(null);
  
  // Form states for create/edit
  const [formData, setFormData] = useState({
    email: "",
    password: "",
    role: "user",
  });

  useEffect(() => {
    setRegistrationEnabled(initialRegistrationEnabled);
    setRequireInviteCode(initialRequireInviteCode);
  }, [initialRegistrationEnabled, initialRequireInviteCode]);

  useEffect(() => {
    const changed = 
      registrationEnabled !== initialRegistrationEnabled ||
      requireInviteCode !== initialRequireInviteCode;
    setHasChanges(changed);
  }, [registrationEnabled, requireInviteCode, initialRegistrationEnabled, initialRequireInviteCode]);

  useEffect(() => {
    if ((clientId && isRoot) || (!clientId && user?.role === "admin")) {
      fetchUsers();
    }
  }, [clientId, isRoot, user?.role]);

  const fetchUsers = async () => {
    setLoading(true);
    try {
      let response;
      if (isRoot && clientId) {
        // Root user viewing specific client
        response = await api.get(`/root/clients/${clientId}/users`);
      } else if (user?.role === "admin") {
        // Admin viewing their own client's users
        response = await api.get(`/admin/users`);
      } else {
        return;
      }
      setUsers(Array.isArray(response) ? response : []);
    } catch (error) {
      console.error("Failed to fetch users:", error);
      setMessage({ type: "error", text: "Failed to load users" });
    } finally {
      setLoading(false);
    }
  };

  const handleSaveSettings = async () => {
    setSaving(true);
    setMessage(null);
    try {
      const response = await api.get("/admin/config");
      const configToSave = {
        ...response,
        registrationEnabled,
        requireInviteCode,
        sessionTimeout: 86400,
      };
      await api.put("/admin/config", configToSave);
      setMessage({ type: "success", text: "Settings saved successfully" });
      setHasChanges(false);
      onUpdate?.();
    } catch (error) {
      setMessage({ type: "error", text: "Failed to save settings" });
    } finally {
      setSaving(false);
    }
  };

  const handleCancel = () => {
    setRegistrationEnabled(initialRegistrationEnabled);
    setRequireInviteCode(initialRequireInviteCode);
    setMessage(null);
  };

  const handleCreateUser = async () => {
    try {
      const endpoint = isRoot && clientId 
        ? `/root/clients/${clientId}/users`
        : `/admin/users`;
      await api.post(endpoint, {
        username: formData.email,
        password: formData.password,
        role: formData.role,
      });
      setMessage({ type: "success", text: "User created successfully" });
      setShowCreateDialog(false);
      setFormData({ email: "", password: "", role: "user" });
      fetchUsers();
    } catch (error: any) {
      setMessage({ 
        type: "error", 
        text: error.response?.data?.error || "Failed to create user" 
      });
    }
  };

  const handleUpdateUser = async () => {
    if (!selectedUser) return;
    
    try {
      const updateData: any = { role: formData.role };
      if (formData.password) {
        updateData.password = formData.password;
      }
      
      const endpoint = isRoot && clientId
        ? `/root/clients/${clientId}/users/${selectedUser.id}`
        : `/admin/users/${selectedUser.id}`;
      await api.put(endpoint, updateData);
      setMessage({ type: "success", text: "User updated successfully" });
      setShowEditDialog(false);
      setSelectedUser(null);
      setFormData({ email: "", password: "", role: "user" });
      fetchUsers();
    } catch (error: any) {
      setMessage({ 
        type: "error", 
        text: error.response?.data?.error || "Failed to update user" 
      });
    }
  };

  const handleDeleteUser = async (userId: string) => {
    if (!confirm("Are you sure you want to delete this user? This action cannot be undone.")) {
      return;
    }
    
    try {
      const endpoint = isRoot && clientId
        ? `/root/clients/${clientId}/users/${userId}`
        : `/admin/users/${userId}`;
      await api.delete(endpoint);
      setMessage({ type: "success", text: "User deleted successfully" });
      fetchUsers();
    } catch (error: any) {
      setMessage({ 
        type: "error", 
        text: error.response?.data?.error || "Failed to delete user" 
      });
    }
  };

  const openEditDialog = (user: User) => {
    setSelectedUser(user);
    setFormData({
      email: user.username,
      password: "",
      role: user.role,
    });
    setShowEditDialog(true);
  };

  const filteredUsers = users.filter(user => {
    const matchesSearch = user.username.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesRole = selectedRole === "all" || user.role === selectedRole;
    return matchesSearch && matchesRole;
  });

  const getRoleBadge = (role: string) => {
    switch (role) {
      case "admin":
        return <Badge className="bg-purple-500">Admin</Badge>;
      case "user":
        return <Badge variant="secondary">User</Badge>;
      default:
        return <Badge variant="outline">{role}</Badge>;
    }
  };

  return (
    <div className="space-y-6">
      {/* Registration Settings Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Settings className="h-5 w-5" />
            Registration Settings
          </CardTitle>
          <CardDescription>
            Configure how new users can join the system
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {message && (
            <Alert
              className={`${
                message.type === "success" ? "border-green-500" : "border-red-500"
              }`}
            >
              <AlertDescription>{message.text}</AlertDescription>
            </Alert>
          )}

          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <Label>Enable Registration</Label>
              <p className="text-sm text-muted-foreground">
                Allow new users to register
              </p>
            </div>
            <Button
              variant={registrationEnabled ? "default" : "outline"}
              onClick={() => setRegistrationEnabled(!registrationEnabled)}
            >
              {registrationEnabled ? "Enabled" : "Disabled"}
            </Button>
          </div>

          {registrationEnabled && (
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label>Require Invite Code</Label>
                <p className="text-sm text-muted-foreground">
                  Users need an invite code to register
                </p>
              </div>
              <Button
                variant={requireInviteCode ? "default" : "outline"}
                onClick={() => setRequireInviteCode(!requireInviteCode)}
              >
                {requireInviteCode ? "Required" : "Not Required"}
              </Button>
            </div>
          )}

          {hasChanges && (
            <div className="flex justify-end gap-2 pt-4">
              <Button variant="outline" onClick={handleCancel}>
                Cancel
              </Button>
              <Button onClick={handleSaveSettings} disabled={saving}>
                {saving ? "Saving..." : "Save Changes"}
              </Button>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Users List Card - Show for root users with clientId OR admin users */}
      {(clientId && isRoot) || (!clientId && user?.role === "admin") ? (
        <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <UserPlus className="h-5 w-5" />
                Users
              </CardTitle>
              <CardDescription>
                Manage system users and their permissions
              </CardDescription>
            </div>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={fetchUsers}
                disabled={loading}
              >
                <RefreshCw className={`h-4 w-4 mr-2 ${loading ? "animate-spin" : ""}`} />
                Refresh
              </Button>
              <Button
                size="sm"
                onClick={() => {
                  setFormData({ email: "", password: "", role: "user" });
                  setShowCreateDialog(true);
                }}
              >
                <Plus className="h-4 w-4 mr-2" />
                Add User
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {/* Search and Filter */}
          <div className="flex gap-2 mb-4">
            <div className="relative flex-1">
              <Search className="absolute left-2 top-2.5 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Search users..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-8"
              />
            </div>
            <Select value={selectedRole} onValueChange={setSelectedRole}>
              <SelectTrigger className="w-[150px]">
                <SelectValue placeholder="Filter by role" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All Roles</SelectItem>
                <SelectItem value="admin">Admin</SelectItem>
                <SelectItem value="user">User</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* Users Table */}
          {loading ? (
            <div className="flex items-center justify-center py-8">
              <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : filteredUsers.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              {searchQuery || selectedRole !== "all" 
                ? "No users found matching your criteria" 
                : "No users found"}
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Username</TableHead>
                  <TableHead>Role</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredUsers.map((user) => (
                  <TableRow key={user.id}>
                    <TableCell className="font-medium">{user.username}</TableCell>
                    <TableCell>{getRoleBadge(user.role)}</TableCell>
                    <TableCell>
                      {format(new Date(user.created_at), "MMM d, yyyy")}
                    </TableCell>
                    <TableCell className="text-right">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="sm">
                            <MoreHorizontal className="h-4 w-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem onClick={() => openEditDialog(user)}>
                            <Edit className="h-4 w-4 mr-2" />
                            Edit
                          </DropdownMenuItem>
                          <DropdownMenuItem 
                            onClick={() => handleDeleteUser(user.id)}
                            className="text-red-600"
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
          )}
        </CardContent>
      </Card>
      ) : !clientId && user?.role !== "admin" ? (
        <Card>
          <CardContent className="flex items-center justify-center py-16">
            <p className="text-muted-foreground">
              No client selected. User management is not available.
            </p>
          </CardContent>
        </Card>
      ) : null}

      {/* Create User Dialog */}
      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create New User</DialogTitle>
            <DialogDescription>
              Add a new user to the system
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="email">Username</Label>
              <Input
                id="email"
                type="email"
                value={formData.email}
                onChange={(e) => setFormData({ ...formData, email: e.target.value })}
                placeholder="username"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="password">Password</Label>
              <Input
                id="password"
                type="password"
                value={formData.password}
                onChange={(e) => setFormData({ ...formData, password: e.target.value })}
                placeholder="Enter password"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="role">Role</Label>
              <Select 
                value={formData.role} 
                onValueChange={(value) => setFormData({ ...formData, role: value })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="user">User</SelectItem>
                  <SelectItem value="admin">Admin</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowCreateDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleCreateUser}>
              Create User
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit User Dialog */}
      <Dialog open={showEditDialog} onOpenChange={setShowEditDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit User</DialogTitle>
            <DialogDescription>
              Update user information
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="edit-email">Username</Label>
              <Input
                id="edit-email"
                type="email"
                value={formData.email}
                disabled
                className="bg-muted"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-password">New Password (optional)</Label>
              <Input
                id="edit-password"
                type="password"
                value={formData.password}
                onChange={(e) => setFormData({ ...formData, password: e.target.value })}
                placeholder="Leave blank to keep current password"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-role">Role</Label>
              <Select 
                value={formData.role} 
                onValueChange={(value) => setFormData({ ...formData, role: value })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="user">User</SelectItem>
                  <SelectItem value="admin">Admin</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowEditDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleUpdateUser}>
              Update User
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}