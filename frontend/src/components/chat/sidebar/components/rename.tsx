import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { useSnapshot } from "valtio";
import { sidebarStore, sidebarActions } from "@/lib/store/chat/sidebar-store";

interface RenameConversationDialogProps {
  onRename: () => void;
}

export function RenameConversationDialog({ onRename }: RenameConversationDialogProps) {
  const sidebarSnapshot = useSnapshot(sidebarStore);

  return (
    <Dialog open={sidebarSnapshot.renameDialogOpen} onOpenChange={(open) => !open && sidebarActions.closeRenameDialog()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Rename Conversation</DialogTitle>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          <div className="grid grid-cols-4 items-center gap-4">
            <Label htmlFor="title" className="text-right">
              Title
            </Label>
            <Input
              id="title"
              value={sidebarSnapshot.newTitle}
              onChange={(e) => sidebarActions.setNewTitle(e.target.value)}
              className="col-span-3"
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  onRename();
                }
              }}
              placeholder="Enter conversation title"
              autoFocus
            />
          </div>
        </div>
        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => sidebarActions.closeRenameDialog()}
          >
            Cancel
          </Button>
          <Button
            type="button"
            onClick={onRename}
            disabled={!sidebarSnapshot.newTitle.trim()}
          >
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}