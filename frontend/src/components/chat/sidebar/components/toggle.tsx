import { PanelLeftClose, PanelLeftOpen } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useSnapshot } from "valtio";
import { sidebarStore, sidebarActions } from "@/store/sidebar-store";

export function MobileMenuToggle() {
  const sidebarSnapshot = useSnapshot(sidebarStore);

  return (
    <Button
      variant="ghost"
      size="sm"
      onClick={() => sidebarActions.toggleMobileMenu()}
      className="fixed top-4 left-4 z-40 h-10 w-10 p-0 md:hidden rounded-full shadow-lg bg-background border"
    >
      {sidebarSnapshot.isMobileMenuOpen ? (
        <PanelLeftClose className="h-5 w-5" />
      ) : (
        <PanelLeftOpen className="h-5 w-5" />
      )}
    </Button>
  );
}