import { Navigate, Route, Routes } from "react-router-dom";
import { MainApp } from "./pages/MainApp";

function App() {
  return (
    <Routes>
      <Route path="/" element={<MainApp />} />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}

export default App;
