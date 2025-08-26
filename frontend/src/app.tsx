import { Navigate, Route, Routes } from "react-router-dom";
import { MainApp } from "./pages/MainApp";
import { ProjectsPage } from "./pages/Projects";
import { AuthPage } from "./pages/AuthPage";
import { SetupPage } from "./pages/SetupPage";
import { ValtioProvider } from "./providers/ValtioProvider";
import { SetupRoute } from "./components/auth/setup-route";

function App() {
  return (
    <ValtioProvider>
      <Routes>
        <Route
          path="/"
          element={
            <SetupRoute>
              <MainApp />
            </SetupRoute>
          }
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
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </ValtioProvider>
  );
}

export default App;
