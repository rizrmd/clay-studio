import { useEffect } from "react";
import { Users, Plus, Trash2, Crown, User as UserIcon } from "lucide-react";
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
import { proxy, useSnapshot } from "valtio";

interface MembersPageProps {
  projectId: string;
  currentUserId?: string;
}

const membersPageState = proxy({
  members: [] as ProjectMember[],
  loading: false,
  newUserIdInput: "",
  removingMemberId: null as string | null,
  transferringToUserId: null as string | null,
});

export function MembersPage({ projectId, currentUserId }: MembersPageProps) {
  const state = useSnapshot(membersPageState);

  const currentUserRole = Array.isArray(state.members)
    ? state.members.find((m) => m.user_id === currentUserId)?.role
    : undefined;
  const isCurrentUserOwner = currentUserRole === "owner";
  const ownerCount = Array.isArray(state.members)
    ? state.members.filter((m) => m.role === "owner").length
    : 0;

  const loadMembers = async () => {
    membersPageState.loading = true;
    try {
      const data = await listProjectMembers(projectId);
      console.log("[MembersPage] API response:", data);
      if (Array.isArray(data)) {
        membersPageState.members = data;
      } else {
        console.error("[MembersPage] Expected array but got:", typeof data, data);
        toast.error("Invalid members data format");
        membersPageState.members = [];
      }
    } catch (error) {
      toast.error("Failed to load project members");
      console.error(error);
    } finally {
      membersPageState.loading = false;
    }
  };

  useEffect(() => {
    loadMembers();
  }, [projectId]);

  const handleAddMember = async () => {
    if (!state.newUserIdInput.trim()) {
      toast.error("Please enter a user ID");
      return;
    }

    try {
      await addProjectMember(projectId, {
        user_id: state.newUserIdInput.trim(),
        role: "member",
      });
      toast.success("Member added successfully");
      membersPageState.newUserIdInput = "";
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
      membersPageState.removingMemberId = null;
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
    if (!state.transferringToUserId) return;

    try {
      await transferProjectOwnership(projectId, {
        new_owner_user_id: state.transferringToUserId,
      });
      toast.success("Ownership transferred successfully");
      membersPageState.transferringToUserId = null;
      await loadMembers();
    } catch (error: any) {
      const message = error?.message || "Failed to transfer ownership";
      toast.error(message);
      console.error(error);
    }
  };

  return (
    <>
      <div className="flex flex-col h-full bg-background">
        <div className="border-b p-4">
          <div className="flex items-center gap-2">
            <Users className="h-5 w-5" />
            <h1 className="text-lg font-semibold">Project Members</h1>
          </div>
          <p className="text-sm text-muted-foreground mt-1">
            Manage who has access to this project
          </p>
        </div>

        <div className="flex-1 overflow-hidden p-4">
          {/* Add Member Section */}
          {isCurrentUserOwner && (
            <div className="flex gap-2 mb-4">
              <Input
                placeholder="Enter user ID to add..."
                value={state.newUserIdInput}
                onChange={(e) => membersPageState.newUserIdInput = e.target.value}
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
          <ScrollArea className="h-[calc(100vh-200px)]">
            {state.loading ? (
              <div className="text-center py-8 text-muted-foreground">
                Loading members...
              </div>
            ) : state.members.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                No members found
              </div>
            ) : (
              <div className="space-y-2">
                {state.members.map((member) => {
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
                                membersPageState.transferringToUserId = member.user_id
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
                            onClick={() => membersPageState.removingMemberId = member.user_id}
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
        </div>
      </div>

      {/* Remove Member Confirmation */}
      <AlertDialog
        open={!!state.removingMemberId}
        onOpenChange={() => membersPageState.removingMemberId = null}
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
              onClick={() => state.removingMemberId && handleRemoveMember(state.removingMemberId)}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Remove
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Transfer Ownership Confirmation */}
      <AlertDialog
        open={!!state.transferringToUserId}
        onOpenChange={() => membersPageState.transferringToUserId = null}
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
