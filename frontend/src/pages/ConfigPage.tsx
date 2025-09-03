import { useAuth } from "@/hooks/use-auth";
import { Navigate } from "react-router-dom";
import { useEffect } from "react";
import { useSnapshot } from "valtio";
import api from "@/lib/utils/api";
import { configPageStore, configPageActions } from "@/store/config-page-store";
import { Skeleton } from "@/components/ui/skeleton";
import { DomainManagement } from "@/components/root/domain-management";
import { UserManagement } from "@/components/shared/user-management";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Card, CardContent } from "@/components/ui/card";
import { Users, Globe } from "lucide-react";



export function ConfigPage() {
  const { user, isAuthenticated } = useAuth();
  const configSnapshot = useSnapshot(configPageStore);

  useEffect(() => {
    if (isAuthenticated && (user?.role === "admin" || user?.role === "root")) {
      const storedClientId = localStorage.getItem('activeClientId');
      configPageActions.setClientId(storedClientId);
      fetchConfig();
    }
  }, [isAuthenticated, user]);

  const fetchConfig = async () => {
    try {
      const response = await api.get("/admin/config");

      configPageActions.setConfig({
        ...response,
        allowedDomains: response.allowedDomains || [],
      });
    } catch (error) {
      console.error("Failed to load configuration:", error);
    } finally {
      configPageActions.setLoading(false);
    }
  };


  const handleUpdate = () => {
    // Refresh config after any update
    fetchConfig();
  };

  if (!isAuthenticated || (user?.role !== "admin" && user?.role !== "root")) {
    return <Navigate to="/" replace />;
  }

  if (configSnapshot.loading) {
    return (
      <div className="container mx-auto py-8 space-y-6">
        <Skeleton className="h-10 w-48" />
        <div className="grid gap-6">
          <Skeleton className="h-96 w-full" />
          <Skeleton className="h-96 w-full" />
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900">

      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold mb-2 text-gray-900 dark:text-gray-100">
            System Configuration
          </h1>
          <p className="text-gray-600 dark:text-gray-400">
            Manage system-wide settings and preferences
          </p>
        </div>

        {/* Tabbed Interface */}
        <Tabs defaultValue="users" className="space-y-4">
          <TabsList className="grid w-full grid-cols-2 lg:w-auto lg:inline-flex">
            <TabsTrigger value="users" className="flex items-center gap-2">
              <Users className="h-4 w-4" />
              <span className="hidden sm:inline">Users</span>
            </TabsTrigger>
            <TabsTrigger value="domains" className="flex items-center gap-2">
              <Globe className="h-4 w-4" />
              <span className="hidden sm:inline">Domains</span>
            </TabsTrigger>
          </TabsList>

           <TabsContent value="users" className="space-y-4">
             <UserManagement
               initialRegistrationEnabled={configSnapshot.config.registrationEnabled}
               initialRequireInviteCode={configSnapshot.config.requireInviteCode}
               {...(user?.role === "root" && configSnapshot.clientId ? { clientId: configSnapshot.clientId } : {})}
               onUpdate={handleUpdate}
             />
           </TabsContent>

           <TabsContent value="domains" className="space-y-4">
             {configSnapshot.clientId ? (
               <DomainManagement
                 clientId={configSnapshot.clientId}
                 initialDomains={[...configSnapshot.config.allowedDomains]}
                 onUpdate={handleUpdate}
               />
             ) : (
              <Card>
                <CardContent className="flex items-center justify-center py-16">
                  <p className="text-muted-foreground">
                    No client selected. Domain management is not available.
                  </p>
                </CardContent>
              </Card>
            )}
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
}
