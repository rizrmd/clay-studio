import { Navigate, Route, Routes } from "react-router-dom";
import { MainApp } from "./pages/MainApp";
import { ProjectsPage } from "./pages/Projects";
import { AuthPage } from "./pages/AuthPage";
import { SetupPage } from "./pages/SetupPage";
import { RootDashboard } from "./pages/RootDashboard";
import { ClientDetailPage } from "./pages/ClientDetail";
import { ConfigPage } from "./pages/ConfigPage";
import { ProfilePage } from "./pages/ProfilePage";
import { WipTablePage } from "./pages/WipTable";
import { ValtioProvider } from "./lib/valtio-provider";
import { SetupRoute } from "./components/auth/setup-route";
import { ProtectedRoute } from "./components/auth/protected-route";
import { AdminRoute } from "./components/auth/admin-route";
import { RootRoute } from "./components/auth/root-route";

function App() {
  return (
    <ValtioProvider>
      <Routes>
        <Route
          path="/"
          element={<Navigate to="/projects" replace />}
        />
        <Route
          path="/chat/:projectId"
          element={
            <SetupRoute>
              <MainApp />
            </SetupRoute>
          }
        />
        <Route
          path="/chat/:projectId/:conversationId"
          element={
            <SetupRoute>
              <MainApp />
            </SetupRoute>
          }
        />
        <Route
          path="/projects"
          element={
            <SetupRoute>
              <ProjectsPage />
            </SetupRoute>
          }
        />
        <Route path="/auth" element={<AuthPage />} />
        <Route path="/setup" element={<SetupPage />} />
        <Route
          path="/profile"
          element={
            <ProtectedRoute>
              <ProfilePage />
            </ProtectedRoute>
          }
        />
        <Route
          path="/root"
          element={
            <RootRoute>
              <RootDashboard />
            </RootRoute>
          }
        />
        <Route
          path="/root/client/:clientId"
          element={
            <RootRoute>
              <ClientDetailPage />
            </RootRoute>
          }
        />
        <Route
          path="/config"
          element={
            <AdminRoute>
              <ConfigPage />
            </AdminRoute>
          }
        />
        <Route path="/wip-table" element={<WipTablePage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </ValtioProvider>
  );
}

export default App;
