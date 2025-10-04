import { useState, useEffect } from "react";
import { Users, Plus, Trash2, Crown, User as UserIcon } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  DialogDescription,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { toast } from "sonner";
import {
  listProjectMembers,
  addProjectMember,
  removeProjectMember,
  updateMemberRole,
  transferProjectOwnership,
  type ProjectMember,
} from "@/lib/api/project-members";

interface ProjectMembersDialogProps {
  projectId: string;
  currentUserId?: string;
  trigger?: React.ReactNode;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

export function ProjectMembersDialog({
  projectId,
  currentUserId,
  trigger,
  open: controlledOpen,
  onOpenChange: controlledOnOpenChange,
}: ProjectMembersDialogProps) {
  const [internalOpen, setInternalOpen] = useState(false);

  // Use controlled state if provided, otherwise use internal state
  const open = controlledOpen !== undefined ? controlledOpen : internalOpen;
  const setOpen = controlledOnOpenChange || setInternalOpen;
  const [members, setMembers] = useState<ProjectMember[]>([]);
  const [loading, setLoading] = useState(false);
  const [newUserIdInput, setNewUserIdInput] = useState("");
  const [removingMemberId, setRemovingMemberId] = useState<string | null>(null);
  const [transferringToUserId, setTransferringToUserId] = useState<string | null>(null);

  const currentUserRole = members.find((m) => m.user_id === currentUserId)?.role;
  const isCurrentUserOwner = currentUserRole === "owner";
  const ownerCount = members.filter((m) => m.role === "owner").length;

  const loadMembers = async () => {
    setLoading(true);
    try {
      const data = await listProjectMembers(projectId);
      setMembers(data);
    } catch (error) {
      toast.error("Failed to load project members");
      console.error(error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (open) {
      loadMembers();
    }
  }, [open, projectId]);

  const handleAddMember = async () => {
    if (!newUserIdInput.trim()) {
      toast.error("Please enter a user ID");
      return;
    }

    try {
      await addProjectMember(projectId, {
        user_id: newUserIdInput.trim(),
        role: "member",
      });
      toast.success("Member added successfully");
      setNewUserIdInput("");
      await loadMembers();
    } catch (error: any) {
      const message = error?.message || "Failed to add member";
      toast.error(message);
      console.error(error);
    }
  };

  const handleRemoveMember = async (userId: string) => {
    try {
      await removeProjectMember(projectId, userId);
      toast.success("Member removed successfully");
      setRemovingMemberId(null);
      await loadMembers();
    } catch (error: any) {
      const message = error?.message || "Failed to remove member";
      toast.error(message);
      console.error(error);
    }
  };

  const handleUpdateRole = async (userId: string, newRole: "owner" | "member") => {
    try {
      await updateMemberRole(projectId, userId, { role: newRole });
      toast.success("Role updated successfully");
      await loadMembers();
    } catch (error: any) {
      const message = error?.message || "Failed to update role";
      toast.error(message);
      console.error(error);
    }
  };

  const handleTransferOwnership = async () => {
    if (!transferringToUserId) return;

    try {
      await transferProjectOwnership(projectId, {
        new_owner_user_id: transferringToUserId,
      });
      toast.success("Ownership transferred successfully");
      setTransferringToUserId(null);
      await loadMembers();
    } catch (error: any) {
      const message = error?.message || "Failed to transfer ownership";
      toast.error(message);
      console.error(error);
    }
  };

  return (
    <>
      <Dialog open={open} onOpenChange={setOpen}>
        {trigger !== null && (
          <DialogTrigger asChild>
            {trigger || (
              <Button variant="ghost" size="sm">
                <Users className="h-4 w-4 mr-2" />
                Members
              </Button>
            )}
          </DialogTrigger>
        )}
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Project Members</DialogTitle>
            <DialogDescription>
              Manage who has access to this project
            </DialogDescription>
          </DialogHeader>

          {/* Add Member Section */}
          {isCurrentUserOwner && (
            <div className="flex gap-2">
              <Input
                placeholder="Enter user ID to add..."
                value={newUserIdInput}
                onChange={(e) => setNewUserIdInput(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    handleAddMember();
                  }
                }}
              />
              <Button onClick={handleAddMember}>
                <Plus className="h-4 w-4 mr-2" />
                Add
              </Button>
            </div>
          )}

          {/* Members List */}
          <ScrollArea className="h-[400px] pr-4">
            {loading ? (
              <div className="text-center py-8 text-muted-foreground">
                Loading members...
              </div>
            ) : members.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                No members found
              </div>
            ) : (
              <div className="space-y-2">
                {members.map((member) => {
                  const isOwner = member.role === "owner";
                  const isCurrentUser = member.user_id === currentUserId;
                  const canRemove =
                    isCurrentUserOwner &&
                    !isCurrentUser &&
                    !(isOwner && ownerCount === 1);

                  return (
                    <div
                      key={member.id}
                      className="flex items-center justify-between p-3 border rounded-lg hover:bg-accent/50 transition-colors"
                    >
                      <div className="flex items-center gap-3">
                        <div className="h-8 w-8 rounded-full bg-primary/10 flex items-center justify-center">
                          {isOwner ? (
                            <Crown className="h-4 w-4 text-primary" />
                          ) : (
                            <UserIcon className="h-4 w-4 text-muted-foreground" />
                          )}
                        </div>
                        <div>
                          <div className="flex items-center gap-2">
                            <span className="font-medium">
                              {member.username}
                            </span>
                            {isCurrentUser && (
                              <Badge variant="secondary" className="text-xs">
                                You
                              </Badge>
                            )}
                          </div>
                          <span className="text-xs text-muted-foreground">
                            {member.user_id}
                          </span>
                        </div>
                      </div>

                      <div className="flex items-center gap-2">
                        {/* Role Selector */}
                        {isCurrentUserOwner && !isCurrentUser ? (
                          <Select
                            value={member.role}
                            onValueChange={(value) =>
                              handleUpdateRole(
                                member.user_id,
                                value as "owner" | "member"
                              )
                            }
                            disabled={isOwner && ownerCount === 1}
                          >
                            <SelectTrigger className="w-28">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="owner">Owner</SelectItem>
                              <SelectItem value="member">Member</SelectItem>
                            </SelectContent>
                          </Select>
                        ) : (
                          <Badge variant={isOwner ? "default" : "secondary"}>
                            {member.role}
                          </Badge>
                        )}

                        {/* Transfer Ownership Button */}
                        {isCurrentUserOwner &&
                          !isCurrentUser &&
                          !isOwner && (
                            <Button
                              variant="outline"
                              size="sm"
                              onClick={() =>
                                setTransferringToUserId(member.user_id)
                              }
                            >
                              <Crown className="h-4 w-4" />
                            </Button>
                          )}

                        {/* Remove Button */}
                        {canRemove && (
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => setRemovingMemberId(member.user_id)}
                          >
                            <Trash2 className="h-4 w-4 text-destructive" />
                          </Button>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </ScrollArea>
        </DialogContent>
      </Dialog>

      {/* Remove Member Confirmation */}
      <AlertDialog
        open={!!removingMemberId}
        onOpenChange={() => setRemovingMemberId(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remove Member</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to remove this member from the project?
              They will lose access to all conversations and data.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => removingMemberId && handleRemoveMember(removingMemberId)}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Remove
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Transfer Ownership Confirmation */}
      <AlertDialog
        open={!!transferringToUserId}
        onOpenChange={() => setTransferringToUserId(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Transfer Ownership</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to transfer project ownership? You will be
              demoted to a regular member. This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={handleTransferOwnership}>
              Transfer Ownership
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}