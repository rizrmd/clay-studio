import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./app";
import "./styles/globals.css";
import "./styles/markdown.css";
import "react-datasheet-grid/dist/style.css";
import "./lib/utils/axios"; // Configure axios
import { initializeApp } from "./lib/store/auth-store";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5, // 5 minutes
      retry: 1,
    },
  },
});

// Initialize app authentication and setup
initializeApp();

ReactDOM.createRoot(document.getElementById("root")!).render(
  // <React.StrictMode>
  <QueryClientProvider client={queryClient}>
    <BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
      <App />
    </BrowserRouter>
  </QueryClientProvider>
  // </React.StrictMode>
);
