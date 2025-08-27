import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useSnapshot } from "valtio";
import { authStore } from "@/store/auth-store";
import { rootService, ClientRootResponse } from "@/services/root-service";
import { ClientManagement } from "@/components/root/client-management";
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
import { useValtioAuth } from "@/hooks/use-valtio-auth";

export function RootDashboard() {
  const auth = useSnapshot(authStore);
  const navigate = useNavigate();
  const { logout } = useValtioAuth();
  const [clients, setClients] = useState<ClientRootResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [stats, setStats] = useState({
    totalClients: 0,
    activeClients: 0,
    totalUsers: 0,
    totalConversations: 0,
  });

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
      setLoading(true);
      setError(null);
      const data = await rootService.getClientsRoot();
      setClients(data);

      // Calculate stats
      const activeClients = data.filter((c) => c.status === "active").length;
      const totalUsers = data.reduce((sum, c) => sum + c.userCount, 0);
      const totalConversations = data.reduce(
        (sum, c) => sum + c.conversationCount,
        0
      );

      setStats({
        totalClients: data.length,
        activeClients,
        totalUsers,
        totalConversations,
      });
    } catch (err: any) {
      setError(err.response?.data?.error || "Failed to load clients");
    } finally {
      setLoading(false);
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
                <span className="text-2xl font-bold">{stats.totalClients}</span>
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
                  {stats.activeClients}
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
                <span className="text-2xl font-bold">{stats.totalUsers}</span>
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
                  {stats.totalConversations}
                </span>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 overflow-auto px-6 pb-6">
        <ClientManagement
          clients={clients}
          loading={loading}
          error={error}
          onRefresh={loadClients}
        />
      </div>
    </div>
  );
}
