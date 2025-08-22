import { Chat } from "@/components/chat";
import { ConversationSidebar } from "@/components/conversation-sidebar";
import { useState } from "react";

export function MainApp() {
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);

  const toggleSidebar = () => {
    setIsSidebarCollapsed(!isSidebarCollapsed);
  };

  return (
    <div className="h-screen flex">
      <ConversationSidebar
        isCollapsed={isSidebarCollapsed}
        onToggle={toggleSidebar}
      />
      <div className="flex flex-1 flex-col">
        <Chat />
      </div>
    </div>
  );
}
