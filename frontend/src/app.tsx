import { Navigate, Route, Routes } from "react-router-dom";
import { MainApp } from "./pages/MainApp";
import { AuthProvider } from "./contexts/AuthContext";
import { SetupRoute } from "./components/auth/SetupRoute";

function App() {
  return (
    <AuthProvider>
      <Routes>
        <Route 
          path="/" 
          element={
            <SetupRoute>
              <MainApp />
            </SetupRoute>
          } 
        />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </AuthProvider>
  );
}

export default App;
