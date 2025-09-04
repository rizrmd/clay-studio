import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useAuth } from "@/hooks/use-auth";
import { BookA, LogOut, User2 } from "lucide-react";
import { useLocation, useNavigate } from "react-router-dom";

export function AppHeader() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();

  const handleLogout = async () => {
    await logout();
    navigate("/auth");
  };

  // Don't show header on auth/setup pages
  if (
    location.pathname.startsWith("/auth") ||
    location.pathname.startsWith("/setup")
  ) {
    return null;
  }

  return (
    <header className="border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container flex h-14 items-center max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="mr-4 flex">
          <button
            onClick={() => navigate("/projects")}
            className="mr-6 flex items-center space-x-2"
          >
            <span className="font-bold text-lg">{user?.clientName}</span>
          </button>
        </div>

        <div className="flex flex-1 items-center justify-between space-x-2 md:justify-end">
          {user && (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  className="relative h-8 px-3 rounded-md"
                >
                  <span className="text-sm capitalize">{user.username}</span>
                  <User2 className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent className="w-56" align="end" forceMount>
                <div className="flex items-center justify-start gap-2 p-2">
                  <div className="flex flex-col space-y-1 leading-none">
                    <p className="font-medium">{user.username}</p>
                    <p className="text-xs text-muted-foreground">{user.role}</p>
                  </div>
                </div>
                <DropdownMenuSeparator />
                <DropdownMenuItem onClick={() => navigate("/profile")}>
                  <BookA className="mr-2 h-4 w-4" />
                  Profile
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem onClick={handleLogout}>
                  <LogOut className="mr-2 h-4 w-4" />
                  Log out
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          )}
        </div>
      </div>
    </header>
  );
}
