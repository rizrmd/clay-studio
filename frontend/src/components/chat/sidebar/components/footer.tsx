import { ChevronDown } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useSnapshot } from "valtio";
import { useAuth } from "@/hooks/use-auth";
import { sidebarStore } from "@/lib/store/chat/sidebar-store";

interface ConversationSidebarFooterProps {
  isCollapsed: boolean;
  onLogout: () => void;
  onProfile: () => void;
}

export function ConversationSidebarFooter({
  isCollapsed,
  onLogout,
  onProfile,
}: ConversationSidebarFooterProps) {
  const sidebarSnapshot = useSnapshot(sidebarStore);
  const { user } = useAuth();

  if (isCollapsed && !sidebarSnapshot.isMobileMenuOpen) {
    return (
      <button
        className="h-8 w-8 p-0 cursor-pointer hover:bg-accent rounded-none flex items-center justify-center pointer-events-auto"
        onClick={onLogout}
        type="button"
      >
        <div className="h-6 w-6 rounded-full bg-primary/10 flex items-center justify-center text-primary font-medium text-xs pointer-events-none">
          {(user?.username || "G").charAt(0).toUpperCase()}
        </div>
      </button>
    );
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          className="w-full justify-between p-2 cursor-pointer hover:bg-accent rounded-md flex items-center pointer-events-auto"
          type="button"
        >
          <div className="flex items-center gap-2 pointer-events-none">
            <div className="h-6 w-6 rounded-full bg-primary/10 flex items-center justify-center text-primary font-medium text-xs">
              {(user?.username || "G").charAt(0).toUpperCase()}
            </div>
            <span className="text-sm">{user?.username || "Guest"}</span>
          </div>
          <ChevronDown className="h-4 w-4 pointer-events-none" />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-56 z-50">
        <DropdownMenuItem onClick={onProfile} className="cursor-pointer">
          Profile
        </DropdownMenuItem>
        <DropdownMenuItem onClick={onLogout} className="cursor-pointer">
          Sign out
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
