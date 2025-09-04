import { useEffect } from "react";
import { useSnapshot } from "valtio";
// import { rootDashboardStore, rootDashboardActions } from "@/store/root-dashboard-store";

// Stub implementation
const rootDashboardStore = {
  stats: {
    totalProjects: 0,
    totalConversations: 0,
    totalUsers: 0,
    totalClients: 0,
    activeClients: 0,
  },
  clients: [] as ClientRootResponse[],
  loading: false,
  isLoading: false,
  error: null as string | null,
  addDialogOpen: false,
};

const rootDashboardActions = {
  setStats: (stats: any) => {
    (rootDashboardStore as any).stats = stats;
  },
  setClients: (clients: ClientRootResponse[]) => {
    (rootDashboardStore as any).clients = clients;
  },
  setLoading: (isLoading: boolean) => {
    (rootDashboardStore as any).loading = isLoading;
    (rootDashboardStore as any).isLoading = isLoading;
  },
  setError: (error: string | null) => {
    (rootDashboardStore as any).error = error;
  },
  setAddDialogOpen: (open: boolean) => {
    (rootDashboardStore as any).addDialogOpen = open;
  },
};
import { useNavigate } from "react-router-dom";
import { authStore } from "@/lib/store/auth-store";
import { rootService, ClientRootResponse } from "@/lib/services/root-service";
import { ClientManagement } from "@/components/root/client-management";
import { AddClientDialog } from "@/components/root/add-client-dialog";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
} from "@/components/ui/card";
import {
  Shield,
  Users,
  Server,
  Activity,
  LogOut,
  User,
} from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useAuth } from "@/hooks/use-auth";

export function RootDashboard() {
  const auth = useSnapshot(authStore);
  const rootDashboardSnapshot = useSnapshot(rootDashboardStore);
  const navigate = useNavigate();
  const { logout } = useAuth();

  useEffect(() => {
    // Check if user has root role
    if (auth.user?.role !== "root") {
      navigate("/");
      return;
    }

    loadClients();
  }, [auth.user, navigate]);

  const loadClients = async () => {
    try {
      rootDashboardActions.setLoading(true);
      rootDashboardActions.setError(null);
      const data = await rootService.getClientsRoot();
      rootDashboardActions.setClients(data);

      // Calculate stats
      const activeClients = data.filter((c: any) => c.status === "active").length;
      const totalUsers = data.reduce((sum: any, c: any) => sum + c.userCount, 0);
      const totalConversations = data.reduce(
        (sum: any, c: any) => sum + c.conversationCount,
        0
      );

      rootDashboardActions.setStats({
        totalClients: data.length,
        activeClients,
        totalUsers,
        totalConversations,
      });
    } catch (err: any) {
      rootDashboardActions.setError(err.response?.data?.error || "Failed to load clients");
    } finally {
      rootDashboardActions.setLoading(false);
    }
  };

  if (auth.user?.role !== "root") {
    return (
      <div className="flex h-screen items-center justify-center">
        <Alert className="max-w-md">
          <Shield className="h-4 w-4" />
          <AlertDescription>
            You don't have permission to access this page. Root access required.
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div className="flex h-screen flex-col">
      {/* Header */}
      <div className="border-b bg-background px-6 py-4">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold flex items-center gap-2">
              <Shield className="h-6 w-6 text-primary" />
              Root Dashboard
            </h1>
            <p className="text-sm text-muted-foreground mt-1">
              System administration and client management
            </p>
          </div>
          <div className="flex items-center gap-4">
            <Badge variant="outline" className="px-3 py-1">
              <span className="text-xs font-medium">ROOT USER</span>
            </Badge>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => navigate('/profile')}
              className="gap-2"
            >
              <User className="h-4 w-4" />
              Profile
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={logout}
              className="gap-2"
            >
              <LogOut className="h-4 w-4" />
              Logout
            </Button>
          </div>
        </div>
      </div>

      {/* Stats Cards */}
      <div className="px-6 py-4">
        <div className="grid grid-cols-4 gap-4">
          <Card>
            <CardHeader className="pb-2">
              <CardDescription className="text-xs">
                Total Clients
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex items-center gap-2">
                <Server className="h-4 w-4 text-muted-foreground" />
                <span className="text-2xl font-bold">{rootDashboardSnapshot.stats.totalClients}</span>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardDescription className="text-xs">
                Active Clients
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex items-center gap-2">
                <Activity className="h-4 w-4 text-green-500" />
                <span className="text-2xl font-bold">
                  {rootDashboardSnapshot.stats.activeClients}
                </span>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardDescription className="text-xs">Total Users</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex items-center gap-2">
                <Users className="h-4 w-4 text-muted-foreground" />
                <span className="text-2xl font-bold">{rootDashboardSnapshot.stats.totalUsers}</span>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardDescription className="text-xs">
                Total Conversations
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex items-center gap-2">
                <Activity className="h-4 w-4 text-muted-foreground" />
                <span className="text-2xl font-bold">
                  {rootDashboardSnapshot.stats.totalConversations}
                </span>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 overflow-auto px-6 pb-6">
        <ClientManagement
          clients={[...rootDashboardSnapshot.clients]}
          loading={rootDashboardSnapshot.loading}
          error={rootDashboardSnapshot.error}
          onRefresh={loadClients}
          onAddClient={() => rootDashboardActions.setAddDialogOpen(true)}
        />
      </div>

      {/* Add Client Dialog */}
      <AddClientDialog
        open={rootDashboardSnapshot.addDialogOpen}
        onOpenChange={rootDashboardActions.setAddDialogOpen}
        onSuccess={loadClients}
      />
    </div>
  );
}
