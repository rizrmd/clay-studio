import { ReactNode, useEffect } from "react";
import { initializeApp, authStore } from "../store/auth-store";
import { WebSocketService } from "./services/chat/websocket-service";
import { useSnapshot } from "valtio";

interface ValtioProviderProps {
  children: ReactNode;
}

/**
 * Provider that initializes the Valtio stores
 * This replaces the need for React Context providers
 */
export function ValtioProvider({ children }: ValtioProviderProps) {
  const { user } = useSnapshot(authStore);

  // Initialize the auth store on app load
  useEffect(() => {
    initializeApp();
  }, []);

  // Connect WebSocket only when user is authenticated
  useEffect(() => {
    if (user) {
      const connectWebSocket = async () => {
        // Longer delay to ensure cookies are properly set after login in all browsers
        await new Promise((resolve) => setTimeout(resolve, 300));

        const wsService = WebSocketService.getInstance();

        // Try to connect with retries
        let connected = false;
        let attempts = 0;
        const maxAttempts = 3;

        while (!connected && attempts < maxAttempts) {
          attempts++;
          try {
            await wsService.connect();
            connected = true;
          } catch (error) {
            console.warn(
              `WebSocket connection attempt ${attempts} failed:`,
              error
            );
            if (attempts < maxAttempts) {
              // Wait before retrying, with exponential backoff
              await new Promise((resolve) =>
                setTimeout(resolve, 1000 * attempts)
              );
            }
          }
        }

        if (!connected) {
          console.error("WebSocket failed to connect after all attempts");
        }
      };

      connectWebSocket();
    }
  }, [user]);

  return <>{children}</>;
}
