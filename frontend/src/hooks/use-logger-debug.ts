import { useEffect } from "react";
import { logger } from "@/lib/utils/logger";

const internal = { init: false };

export function useLoggerDebug() {
  useEffect(() => {
    // Make logger available in window for console access
    (window as any).__clayLogger = {
      toggle: () => {
        const enabled = logger.toggle();
        console.log(
          `%cLogging ${enabled ? "enabled" : "disabled"}`,
          `color: ${enabled ? "green" : "red"}; font-weight: bold;`
        );
        return enabled;
      },
      setLevel: (level: string) => {
        logger.setLevel(level as any);
        console.log(
          `%cLog level set to: ${level}`,
          "color: blue; font-weight: bold;"
        );
      },
      getConfig: () => {
        const config = logger.getConfig();
        return config;
      },
      help: () => {
        console.log(
          `%cClay Studio Logger Debug Commands:
  __clayLogger.toggle()     - Toggle logging on/off
  __clayLogger.setLevel()   - Set log level (debug, info, warn, error)  
  __clayLogger.getConfig()  - Show current configuration
  __clayLogger.help()       - Show this help`,
          "color: gray;"
        );
      },
    };

    // Keyboard shortcut: Ctrl/Cmd + Shift + L
    const handleKeyDown = (event: KeyboardEvent) => {
      if (
        (event.ctrlKey || event.metaKey) &&
        event.shiftKey &&
        event.key === "L"
      ) {
        event.preventDefault();
        const enabled = logger.toggle();
        // Show a temporary notification
        const notification = document.createElement("div");
        notification.textContent = `Logging ${
          enabled ? "enabled" : "disabled"
        }`;
        notification.style.cssText = `
          position: fixed;
          top: 20px;
          right: 20px;
          background: ${enabled ? "#10b981" : "#6b7280"};
          color: white;
          padding: 8px 16px;
          border-radius: 6px;
          font-size: 14px;
          font-weight: 500;
          z-index: 9999;
          transition: opacity 0.3s;
        `;
        document.body.appendChild(notification);

        setTimeout(() => {
          notification.style.opacity = "0";
          setTimeout(() => notification.remove(), 300);
        }, 2000);
      }
    };

    window.addEventListener("keydown", handleKeyDown);

    // Log available debug commands on first load in development
    if (import.meta.env.DEV && !internal.init) {
      internal.init = true;
      console.log(
        "%cPress Ctrl/Cmd+Shift+L to toggle logging",
        "color: #6b7280;"
      );
    }

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      delete (window as any).__clayLogger;
    };
  }, []);
}
