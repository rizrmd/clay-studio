import { Navigate, Route, Routes } from "react-router-dom";
import { AdminRoute } from "./components/auth/admin-route";
import { AuthRoute } from "./components/auth/auth-route";
import { RootRoute } from "./components/auth/root-route";
import { AppRoute } from "./components/auth/app-route";
import { AuthPage } from "./pages/AuthPage";
import { ClientDetailPage } from "./pages/ClientDetail";
import { ConfigPage } from "./pages/ConfigPage";
import { MainApp } from "./pages/MainApp";
import { ProfilePage } from "./pages/ProfilePage";
import { ProjectsPage } from "./pages/Projects";
import { RootDashboard } from "./pages/RootDashboard";
import { SetupPage } from "./pages/SetupPage";
import { WipTablePage } from "./pages/WipTable";

function App() {
  return (
    <Routes>
      <Route path="/" element={<Navigate to="/projects" replace />} />
      <Route
        path="/p/:projectId"
        element={
          <AppRoute>
            <MainApp />
          </AppRoute>
        }
      />
      <Route
        path="/p/:projectId/new"
        element={
          <AppRoute>
            <MainApp />
          </AppRoute>
        }
      />
      <Route
        path="/p/:projectId/c/:conversationId"
        element={
          <AppRoute>
            <MainApp />
          </AppRoute>
        }
      />
      <Route
        path="/p/:projectId/datasources"
        element={
          <AppRoute>
            <MainApp />
          </AppRoute>
        }
      />
      <Route
        path="/projects"
        element={
          <AppRoute>
            <ProjectsPage />
          </AppRoute>
        }
      />
      <Route path="/auth" element={<AuthPage />} />
      <Route path="/setup" element={<SetupPage />} />
      <Route
        path="/profile"
        element={
          <AuthRoute>
            <ProfilePage />
          </AuthRoute>
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
  );
}

export default App;
